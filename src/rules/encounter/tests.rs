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
        difficulty: 15,
        complexity: Complexity::Complex,
        failure_consequence: "The time-lock alarm trips".to_owned(),
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

fn check_for(member: &CrewMember, encounter: &Encounter, loadout: &Loadout<'_>) -> CheckBreakdown {
    build_check(CheckInputs {
        member,
        loadout,
        encounter,
        tuning: &ConditionTuning::default(),
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
fn a_hand_working_their_notice_says_so_on_every_door() {
    let mut member = test_member();
    let encounter = test_encounter();
    member.condition.loyalty = 15;

    let sullen = check_for(&member, &encounter, &Loadout::empty());
    assert!(sullen
        .entries
        .iter()
        .any(|e| e.label == "Wavering loyalty" && e.value == -3));

    member.condition.notice_given = true;
    let leaving = check_for(&member, &encounter, &Loadout::empty());
    assert!(leaving
        .entries
        .iter()
        .any(|e| e.label == "Working their notice"));
    assert_eq!(
        leaving.bonus(),
        sullen.bonus(),
        "notice renames, not rebalances"
    );
}

#[test]
fn the_tuning_is_data_so_a_harsher_city_is_one_edit_away() {
    let mut member = test_member();
    let encounter = test_encounter();
    member.condition.fatigue = 60;

    let standard = ConditionTuning::default();
    let harsh = ConditionTuning {
        fatigue_free_threshold: 30,
        fatigue_step: 5,
        ..standard
    };

    assert_eq!(standard.fatigue_modifier(60), -1);
    assert_eq!(harsh.fatigue_modifier(60), -6);

    let check = build_check(CheckInputs {
        member: &member,
        loadout: &Loadout::empty(),
        encounter: &encounter,
        tuning: &harsh,
        extra: &[],
    });
    assert!(check
        .entries
        .iter()
        .any(|e| e.label == "Fatigue" && e.value == -6));
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
        tuning: &ConditionTuning::default(),
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
fn an_untrained_hand_still_shows_their_zero() {
    let mut member = test_member();
    member.training = Skills::default();
    member.attributes = Attributes::default();
    let encounter = test_encounter();

    let check = check_for(&member, &encounter, &Loadout::empty());
    let shown: Vec<&str> = check
        .significant()
        .map(|entry| entry.label.as_str())
        .collect();

    assert_eq!(check.entries[0].value, 0);
    assert_eq!(shown.first(), Some(&"Lockpicking skill"));
    assert!(!shown.contains(&"Equipment"), "empty kit stays hidden");
}

#[test]
fn the_margin_reports_how_close_it_was() {
    let member = test_member();
    let encounter = test_encounter();
    let result = resolve(check_for(&member, &encounter, &Loadout::empty()), 10);
    assert_eq!(result.margin(), result.total - 15);
    assert!(result.passed());
}
