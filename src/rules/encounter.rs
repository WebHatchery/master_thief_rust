//! The d20 core. One door, one specialist, one roll, and every modifier named.

use super::attributes::{
    attribute_modifier, effective_skills, equipped_attributes, proficiency_bonus,
};
use super::outcome::{ModifierEntry, Outcome};
use crate::model::{CrewMember, Encounter, EquipmentSlot, Loadout, RunEffect, Skill};
use serde::{Deserialize, Serialize};

/// Everything a check needs. `extra` carries modifiers the rules engine does
/// not own — environment, crew chemistry, city-wide heat — already named by
/// whoever computed them.
pub struct CheckInputs<'a> {
    pub member: &'a CrewMember,
    pub loadout: &'a Loadout<'a>,
    pub encounter: &'a Encounter,
    pub extra: &'a [ModifierEntry],
}

/// A check, fully computed but not yet rolled. This is what the planning screen
/// shows before the player commits, and what the results screen shows after.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckBreakdown {
    pub member_id: String,
    pub member_name: String,
    pub encounter_id: String,
    pub skill: Skill,
    pub dc: i32,
    pub entries: Vec<ModifierEntry>,
}

impl CheckBreakdown {
    /// Everything added to the d20.
    pub fn bonus(&self) -> i32 {
        self.entries.iter().map(|entry| entry.value).sum()
    }

    /// Entries worth drawing.
    pub fn significant(&self) -> impl Iterator<Item = &ModifierEntry> {
        self.entries.iter().filter(|entry| entry.is_significant())
    }

    /// The lowest natural roll that still gets through the door. Natural 1
    /// always fails, so it never reports below 2.
    pub fn roll_needed(&self) -> i32 {
        (self.dc - self.bonus()).clamp(2, 21)
    }

    /// Probability of at least scraping through, given the nat-1 / nat-20 rules.
    pub fn success_chance(&self) -> f32 {
        let needed = self.roll_needed();
        // Rolls 2..=19 must meet the target; 20 always succeeds; 1 always fails.
        let passing = (2..=19).filter(|roll| *roll >= needed).count() + 1;
        passing as f32 / 20.0
    }

    /// Probability of the two failure bands.
    pub fn failure_chance(&self) -> f32 {
        1.0 - self.success_chance()
    }
}

/// The outcome of one door, with the arithmetic that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterResult {
    pub check: CheckBreakdown,
    pub roll: i32,
    pub total: i32,
    pub outcome: Outcome,
    pub experience_gained: i32,
    pub stress_inflicted: i32,
    pub run_effect: RunEffect,
}

impl EncounterResult {
    pub fn passed(&self) -> bool {
        self.outcome.passed()
    }

    /// The margin by which the door was cleared or missed.
    pub fn margin(&self) -> i32 {
        self.total - self.check.dc
    }
}

/// Build a check without rolling it. Deterministic and side-effect free, so the
/// planning screen and the run can share one code path.
pub fn build_check(inputs: CheckInputs<'_>) -> CheckBreakdown {
    let CheckInputs {
        member,
        loadout,
        encounter,
        extra,
    } = inputs;

    let skill = encounter.primary_skill;
    let attributes = equipped_attributes(member, loadout);
    let skills = effective_skills(
        &attributes,
        &member.training,
        &member.progression,
        member.specialty_skill,
    );

    let mut entries = vec![ModifierEntry::new(
        format!("{} skill", skill.label()),
        skills.get(skill),
    )];

    if let Some(entry) = focus_attribute_entry(encounter, &attributes) {
        entries.push(entry);
    }

    let kit_bonus = loadout.skill_bonus(skill)
        + loadout
            .iter()
            .map(|(slot, _)| encounter.equipment_bonuses.get(slot))
            .sum::<i32>();
    entries.push(ModifierEntry::new("Equipment", kit_bonus));

    entries.push(ModifierEntry::new(
        "Proficiency",
        proficiency_bonus(member.progression.level),
    ));

    entries.extend(condition_entries(member));
    entries.extend(extra.iter().cloned());

    CheckBreakdown {
        member_id: member.id.clone(),
        member_name: member.name.clone(),
        encounter_id: encounter.id.clone(),
        skill,
        dc: encounter.difficulty,
        entries,
    }
}

/// An encounter that names a primary attribute leans on it *extra*. Where none
/// is named there is no attribute line at all: the skill total already folds in
/// its own attribute pair (GDD 5.1), and the original engine's habit of adding
/// the pair a second time made every door roughly five points easier than its
/// DC claimed — which the distribution soak caught immediately.
fn focus_attribute_entry(
    encounter: &Encounter,
    attributes: &crate::model::Attributes,
) -> Option<ModifierEntry> {
    let kind = encounter.primary_attribute?;
    Some(ModifierEntry::new(
        format!("{} focus", kind.short_label()),
        attribute_modifier(attributes.get(kind)),
    ))
}

/// Fatigue, loyalty, and injuries, each named separately so a player can see
/// which one is costing them the job.
fn condition_entries(member: &CrewMember) -> Vec<ModifierEntry> {
    let condition = &member.condition;
    let mut entries = Vec::new();

    if condition.fatigue > 50 {
        entries.push(ModifierEntry::new(
            "Fatigue",
            -((condition.fatigue - 50) / 10),
        ));
    }

    if condition.loyalty > 80 {
        entries.push(ModifierEntry::new("Loyalty", 1));
    } else if condition.loyalty < 40 {
        entries.push(ModifierEntry::new("Wavering loyalty", -2));
    }

    let injury_penalty: i32 = condition
        .injuries
        .iter()
        .map(|injury| injury.severity.check_penalty())
        .sum();
    if injury_penalty != 0 {
        entries.push(ModifierEntry::new(
            format!("Injuries ({})", condition.injuries.len()),
            injury_penalty,
        ));
    }

    entries
}

/// Apply a d20 to a prepared check. The roll comes from the run's seeded RNG;
/// nothing here draws its own randomness.
pub fn resolve(check: CheckBreakdown, roll: i32) -> EncounterResult {
    resolve_with_effects(check, roll, RunEffect::None, RunEffect::None)
}

/// Resolve, carrying the encounter's critical run effects through to the result.
pub fn resolve_with_effects(
    check: CheckBreakdown,
    roll: i32,
    on_critical_success: RunEffect,
    on_critical_failure: RunEffect,
) -> EncounterResult {
    let total = roll + check.bonus();
    let outcome = Outcome::classify(roll, total, check.dc);
    let run_effect = match outcome {
        Outcome::CriticalSuccess => on_critical_success,
        Outcome::CriticalFailure => on_critical_failure,
        _ => RunEffect::None,
    };

    EncounterResult {
        experience_gained: outcome.experience_for(check.dc),
        stress_inflicted: outcome.stress_for(check.dc),
        total,
        outcome,
        run_effect,
        roll,
        check,
    }
}

/// The equipment slots an encounter rewards bringing, for the outfit screen.
pub fn rewarded_slots(encounter: &Encounter) -> Vec<(EquipmentSlot, i32)> {
    EquipmentSlot::ALL
        .into_iter()
        .map(|slot| (slot, encounter.equipment_bonuses.get(slot)))
        .filter(|(_, value)| *value != 0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::crew::{Condition, Injury};
    use crate::model::equipment::{EquipmentRarity, SlotAssignment};
    use crate::model::target::{Complexity, SlotBonuses};
    use crate::model::{
        AttributeKind, Attributes, CharacterClass, EquipmentDef, Progression, Rarity, Skills,
    };

    fn test_member() -> CrewMember {
        CrewMember {
            id: "vera".to_owned(),
            name: "Vera Sloan".to_owned(),
            specialty: "Safecracker".to_owned(),
            specialty_skill: Skill::Lockpicking,
            background: "Twelve years on the other side of the counter".to_owned(),
            rarity: Rarity::Rare,
            class: CharacterClass::Infiltrator,
            attributes: Attributes {
                strength: 10,
                dexterity: 16,
                intelligence: 14,
                wisdom: 12,
                charisma: 10,
                constitution: 12,
            },
            training: Skills {
                lockpicking: 5,
                stealth: 4,
                ..Skills::default()
            },
            progression: Progression::default(),
            equipment: SlotAssignment::default(),
            condition: Condition::default(),
            special_ability: "Reads a tumbler by feel".to_owned(),
            personality_traits: vec!["Methodical".to_owned()],
            hire_cost: 4000,
        }
    }

    fn test_encounter() -> Encounter {
        Encounter {
            id: "vault_door".to_owned(),
            name: "Main Vault Door".to_owned(),
            description: "Open the massive steel vault with finesse".to_owned(),
            primary_skill: Skill::Lockpicking,
            primary_attribute: None,
            secondary_skill: None,
            difficulty: 15,
            complexity: Complexity::Complex,
            failure_consequence: "The time-lock alarm trips".to_owned(),
            success_bonus: None,
            critical_failure_effect: None,
            critical_success_reward: None,
            equipment_bonuses: SlotBonuses::default(),
            critical_success_run_effect: RunEffect::None,
            critical_failure_run_effect: RunEffect::None,
            complication_only: false,
        }
    }

    fn picks() -> EquipmentDef {
        EquipmentDef {
            id: "diamond_picks".to_owned(),
            name: "Diamond-Tipped Picks".to_owned(),
            slot: EquipmentSlot::Tool,
            rarity: EquipmentRarity::Advanced,
            description: "Cuts where lesser steel slips".to_owned(),
            attribute_bonuses: Default::default(),
            skill_bonuses: Skills {
                lockpicking: 3,
                ..Skills::default()
            },
            special_effects: Vec::new(),
            cost: 3200,
            required_level: 0,
            required_class: Vec::new(),
        }
    }

    fn check_for(
        member: &CrewMember,
        encounter: &Encounter,
        loadout: &Loadout<'_>,
    ) -> CheckBreakdown {
        build_check(CheckInputs {
            member,
            loadout,
            encounter,
            extra: &[],
        })
    }

    #[test]
    fn a_check_names_every_modifier_it_applies() {
        let member = test_member();
        let encounter = test_encounter();
        let check = check_for(&member, &encounter, &Loadout::empty());

        assert_eq!(check.dc, 15);
        assert_eq!(check.skill, Skill::Lockpicking);
        assert!(check.entries.iter().any(|e| e.label == "Lockpicking skill"));
        assert!(check.entries.iter().any(|e| e.label == "Proficiency"));
        // Lockpicking: 5 training + DEX(+3) + INT(+2) + level bonus 0 = 10,
        // plus proficiency (+2). The attribute pair is inside the skill and is
        // deliberately not counted a second time.
        assert_eq!(check.bonus(), 12);
    }

    #[test]
    fn a_named_primary_attribute_adds_a_focus_line_and_nothing_else_does() {
        let member = test_member();
        let mut encounter = test_encounter();

        let plain = check_for(&member, &encounter, &Loadout::empty());
        assert!(!plain.entries.iter().any(|e| e.label.contains("focus")));

        encounter.primary_attribute = Some(AttributeKind::Dexterity);
        let focused = check_for(&member, &encounter, &Loadout::empty());
        assert!(focused.entries.iter().any(|e| e.label == "DEX focus"));
        assert_eq!(focused.bonus() - plain.bonus(), 3);
    }

    #[test]
    fn equipment_skill_bonuses_reach_the_check() {
        let member = test_member();
        let encounter = test_encounter();
        let tool = picks();
        let mut assignment = SlotAssignment::default();
        assignment.set(EquipmentSlot::Tool, Some(tool.id.clone()));
        let loadout = Loadout::resolve(&assignment, |id| (id == tool.id).then_some(&tool));

        let bare = check_for(&member, &encounter, &Loadout::empty()).bonus();
        let kitted = check_for(&member, &encounter, &loadout).bonus();
        assert_eq!(kitted - bare, 3);
    }

    #[test]
    fn an_encounters_slot_bonus_rewards_bringing_the_right_kit() {
        let member = test_member();
        let mut encounter = test_encounter();
        encounter.equipment_bonuses = SlotBonuses {
            tool: 2,
            ..SlotBonuses::default()
        };
        let tool = picks();
        let mut assignment = SlotAssignment::default();
        assignment.set(EquipmentSlot::Tool, Some(tool.id.clone()));
        let loadout = Loadout::resolve(&assignment, |id| (id == tool.id).then_some(&tool));

        let kitted = check_for(&member, &encounter, &loadout).bonus();
        let bare = check_for(&member, &encounter, &Loadout::empty()).bonus();
        assert_eq!(kitted - bare, 5);
        assert_eq!(rewarded_slots(&encounter), vec![(EquipmentSlot::Tool, 2)]);
    }

    #[test]
    fn heavy_fatigue_costs_the_crew_dice() {
        let mut member = test_member();
        let encounter = test_encounter();
        let rested = check_for(&member, &encounter, &Loadout::empty()).bonus();

        member.condition.fatigue = 80;
        let tired = check_for(&member, &encounter, &Loadout::empty()).bonus();
        assert_eq!(rested - tired, 3);
    }

    #[test]
    fn loyalty_cuts_both_ways() {
        let mut member = test_member();
        let encounter = test_encounter();
        let base = check_for(&member, &encounter, &Loadout::empty()).bonus();

        member.condition.loyalty = 95;
        assert_eq!(
            check_for(&member, &encounter, &Loadout::empty()).bonus() - base,
            1
        );

        member.condition.loyalty = 20;
        assert_eq!(
            check_for(&member, &encounter, &Loadout::empty()).bonus() - base,
            -2
        );
    }

    #[test]
    fn injuries_stack_their_penalties() {
        let mut member = test_member();
        let encounter = test_encounter();
        let healthy = check_for(&member, &encounter, &Loadout::empty()).bonus();

        member
            .condition
            .injuries
            .push(Injury::minor("Bruised ribs"));
        member
            .condition
            .injuries
            .push(Injury::major("Torn shoulder"));
        let hurt = check_for(&member, &encounter, &Loadout::empty()).bonus();
        assert_eq!(healthy - hurt, 4);
    }

    #[test]
    fn extra_modifiers_are_carried_through_verbatim() {
        let member = test_member();
        let encounter = test_encounter();
        let extra = [
            ModifierEntry::new("Night", 2),
            ModifierEntry::new("Heat", -1),
        ];
        let check = build_check(CheckInputs {
            member: &member,
            loadout: &Loadout::empty(),
            encounter: &encounter,
            extra: &extra,
        });

        assert!(check.entries.contains(&ModifierEntry::new("Night", 2)));
        assert!(check.entries.contains(&ModifierEntry::new("Heat", -1)));
    }

    #[test]
    fn a_natural_one_fails_however_good_the_crew_is() {
        let member = test_member();
        let mut encounter = test_encounter();
        encounter.difficulty = 5;
        let result = resolve(check_for(&member, &encounter, &Loadout::empty()), 1);
        assert_eq!(result.outcome, Outcome::CriticalFailure);
    }

    #[test]
    fn a_natural_twenty_succeeds_however_bad_the_odds_are() {
        let member = test_member();
        let mut encounter = test_encounter();
        encounter.difficulty = 25;
        let result = resolve(check_for(&member, &encounter, &Loadout::empty()), 20);
        assert_eq!(result.outcome, Outcome::CriticalSuccess);
    }

    #[test]
    fn critical_run_effects_only_fire_on_criticals() {
        let member = test_member();
        let mut encounter = test_encounter();
        // Bonus 12 + roll 10 = 22: a critical needs DC 12 or lower, and a
        // plain pass needs DC 18-22.
        encounter.difficulty = 20;
        let check = check_for(&member, &encounter, &Loadout::empty());

        let crit = resolve_with_effects(check.clone(), 20, RunEffect::SkipNext, RunEffect::None);
        assert_eq!(crit.run_effect, RunEffect::SkipNext);

        let plain = resolve_with_effects(check, 10, RunEffect::SkipNext, RunEffect::None);
        assert_eq!(plain.run_effect, RunEffect::None);
    }

    #[test]
    fn the_reported_chance_matches_the_roll_needed() {
        let member = test_member();
        let encounter = test_encounter();
        let check = check_for(&member, &encounter, &Loadout::empty());

        // Bonus 12 against DC 15 means a 3 or better gets through.
        assert_eq!(check.roll_needed(), 3);
        assert!((check.success_chance() - 0.90).abs() < 1e-6);
        assert!((check.failure_chance() - 0.10).abs() < 1e-6);
    }

    #[test]
    fn an_impossible_check_still_leaves_the_natural_twenty() {
        let member = test_member();
        let mut encounter = test_encounter();
        encounter.difficulty = 60;
        let check = check_for(&member, &encounter, &Loadout::empty());

        assert_eq!(check.roll_needed(), 21);
        assert!((check.success_chance() - 0.05).abs() < 1e-6);
    }

    #[test]
    fn the_margin_reports_how_close_it_was() {
        let member = test_member();
        let encounter = test_encounter();
        let result = resolve(check_for(&member, &encounter, &Loadout::empty()), 10);
        assert_eq!(result.margin(), result.total - 15);
        assert!(result.passed());
    }
}
