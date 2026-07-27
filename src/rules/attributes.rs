//! Attribute modifiers, derived stats, skill totals, and the level curve.
//!
//! Ported from the original `characterCalculations.ts`, with the stats nothing
//! consumed pruned away (GDD 0, mechanic carry-over table).

use crate::model::{Attributes, CrewMember, DerivedStats, Loadout, Progression, Skill, Skills};
use serde::{Deserialize, Serialize};

/// The standard `(score - 10) / 2`, rounded *down* — including for negatives,
/// which is why this uses Euclidean division rather than Rust's truncation.
pub fn attribute_modifier(score: i32) -> i32 {
    (score - 10).div_euclid(2)
}

/// Health and initiative. The original also computed stamina, carrying capacity
/// and a critical chance/multiplier that no rule ever read; GDD 0 says to keep
/// only what a rule consumes, and stamina turned out to be consumed by nothing
/// but its own assertion.
pub fn derived_stats(attributes: &Attributes, level: i32) -> DerivedStats {
    let con = attribute_modifier(attributes.constitution);
    let dex = attribute_modifier(attributes.dexterity);
    let wis = attribute_modifier(attributes.wisdom);

    DerivedStats {
        health: 10 + con + level * (2 + con),
        initiative: dex + wis,
    }
}

/// Skill totals: training ranks, plus the skill's attribute pair, plus half the
/// member's level, plus mastery on their own specialty. Never negative.
pub fn effective_skills(
    attributes: &Attributes,
    training: &Skills,
    progression: &Progression,
    specialty_skill: Skill,
) -> Skills {
    let level_bonus = progression.level / 2;
    let mut skills = Skills::default();

    for skill in Skill::ALL {
        let (first, second) = skill.attribute_pair();
        let pair_bonus =
            attribute_modifier(attributes.get(first)) + attribute_modifier(attributes.get(second));
        let mastery_bonus = if skill == specialty_skill {
            progression.mastery_level
        } else {
            0
        };
        let total = training.get(skill) + pair_bonus + level_bonus + mastery_bonus;
        skills.set(skill, total.max(0));
    }

    skills
}

/// The attributes a member fights with once their kit is accounted for.
pub fn equipped_attributes(member: &CrewMember, loadout: &Loadout<'_>) -> Attributes {
    let mut attributes = member.attributes;
    loadout.attribute_bonuses().apply_to(&mut attributes);
    attributes
}

/// A member's skill totals with equipment attribute *and* skill bonuses folded
/// in — what the crew screen shows.
pub fn equipped_skills(member: &CrewMember, loadout: &Loadout<'_>) -> Skills {
    let attributes = equipped_attributes(member, loadout);
    let mut skills = effective_skills(
        &attributes,
        &member.training,
        &member.progression,
        member.specialty_skill,
    );
    for skill in Skill::ALL {
        skills.add(skill, loadout.skill_bonus(skill));
    }
    skills
}

/// Experience needed to clear a given level: `level^2 * 100`.
pub fn experience_to_next(level: i32) -> i32 {
    level * level * 100
}

/// Cumulative experience spent reaching a level from level 1.
pub fn total_experience(level: i32) -> i32 {
    (1..level).map(experience_to_next).sum()
}

/// The D&D-style proficiency bonus every check gets for free.
pub fn proficiency_bonus(level: i32) -> i32 {
    (level - 1) / 4 + 2
}

/// How long it takes to get good at the one thing you are for. Authored in
/// `game_config.json`: the shape is a rule, the numbers are balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasteryTuning {
    /// Doors of your own trade needed for the first rank.
    pub doors_for_first: u32,
    /// Added to that requirement at every rank after.
    pub doors_step: u32,
    pub max_level: i32,
}

impl Default for MasteryTuning {
    fn default() -> Self {
        Self {
            doors_for_first: 3,
            doors_step: 2,
            max_level: 10,
        }
    }
}

impl MasteryTuning {
    /// Doors still owed before the next rank, at the rank they hold now.
    pub fn doors_for_next(&self, level: i32) -> u32 {
        self.doors_for_first + self.doors_step * level.max(0) as u32
    }

    pub fn is_capped(&self, level: i32) -> bool {
        level >= self.max_level
    }
}

/// One door of the member's own trade, cleared. Mastery was declared on every
/// dossier, read by [`effective_skills`], and written by nothing at all — a
/// hand's specialisation could never actually deepen. It advances here, and
/// only here: you get good at a trade by practising it successfully, so a
/// failed door teaches nothing and somebody else's door teaches nothing.
///
/// Returns true when that door earned a rank, so the run can say so.
pub fn work_the_trade(progression: &mut Progression, tuning: &MasteryTuning) -> bool {
    if tuning.is_capped(progression.mastery_level) {
        return false;
    }

    progression.specialty_doors += 1;
    let needed = tuning.doors_for_next(progression.mastery_level);
    if progression.specialty_doors < needed {
        return false;
    }

    progression.specialty_doors -= needed;
    progression.mastery_level += 1;
    if tuning.is_capped(progression.mastery_level) {
        progression.specialty_doors = 0;
    }
    true
}

/// Award experience, levelling as many times as it covers. Returns how many
/// levels were gained.
pub fn award_experience(progression: &mut Progression, amount: i32) -> i32 {
    progression.experience += amount;
    let mut levels = 0;

    while progression.experience >= progression.experience_to_next {
        progression.experience -= progression.experience_to_next;
        progression.level += 1;
        progression.experience_to_next = experience_to_next(progression.level);
        progression.attribute_points += 1;
        progression.skill_points += 2;
        levels += 1;
    }

    levels
}

/// A rough power rating, used for sorting the roster and for balance checks.
pub fn power_level(member: &CrewMember, loadout: &Loadout<'_>) -> i32 {
    let attributes = equipped_attributes(member, loadout);
    let skills = equipped_skills(member, loadout);
    attributes.total() + skills.total() + (loadout.len() as i32) * 5 + member.progression.level * 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::crew::Condition;
    use crate::model::equipment::SlotAssignment;
    use crate::model::{CharacterClass, Rarity};

    fn attrs(str_: i32, dex: i32, int: i32, wis: i32, cha: i32, con: i32) -> Attributes {
        Attributes {
            strength: str_,
            dexterity: dex,
            intelligence: int,
            wisdom: wis,
            charisma: cha,
            constitution: con,
        }
    }

    fn member(attributes: Attributes, training: Skills, progression: Progression) -> CrewMember {
        CrewMember {
            id: "test".to_owned(),
            name: "Test Subject".to_owned(),
            specialty: "Test".to_owned(),
            specialty_skill: Skill::Stealth,
            background: "A control group of one".to_owned(),
            rarity: Rarity::Common,
            class: CharacterClass::Wildcard,
            attributes,
            training,
            progression,
            equipment: SlotAssignment::default(),
            condition: Condition::default(),
            special_ability: "None".to_owned(),
            personality_traits: Vec::new(),
            hire_cost: 0,
        }
    }

    #[test]
    fn attribute_modifiers_match_the_dnd_table() {
        assert_eq!(attribute_modifier(3), -4);
        assert_eq!(attribute_modifier(8), -1);
        assert_eq!(attribute_modifier(9), -1);
        assert_eq!(attribute_modifier(10), 0);
        assert_eq!(attribute_modifier(11), 0);
        assert_eq!(attribute_modifier(12), 1);
        assert_eq!(attribute_modifier(18), 4);
        assert_eq!(attribute_modifier(20), 5);
    }

    #[test]
    fn derived_stats_come_from_constitution_and_level() {
        let a = attrs(14, 16, 12, 13, 10, 15);
        let stats = derived_stats(&a, 1);
        // CON 15 -> +2, STR 14 -> +2, DEX 16 -> +3, WIS 13 -> +1
        assert_eq!(stats.health, 10 + 2 + (2 + 2));
        assert_eq!(stats.initiative, 3 + 1);
    }

    #[test]
    fn health_scales_with_level() {
        let a = attrs(10, 10, 10, 10, 10, 14);
        let low = derived_stats(&a, 1).health;
        let high = derived_stats(&a, 5).health;
        assert!(high > low);
    }

    #[test]
    fn skills_add_the_attribute_pair_and_half_the_level() {
        let a = attrs(10, 16, 14, 12, 10, 10);
        let training = Skills {
            stealth: 5,
            lockpicking: 4,
            ..Skills::default()
        };
        let progression = Progression {
            level: 4,
            ..Progression::default()
        };

        let skills = effective_skills(&a, &training, &progression, Skill::Lockpicking);
        // Stealth: 5 + DEX(+3) + WIS(+1) + level bonus 2 = 11
        assert_eq!(skills.stealth, 11);
        // Lockpicking: 4 + DEX(+3) + INT(+2) + 2 = 11
        assert_eq!(skills.lockpicking, 11);
    }

    #[test]
    fn a_trade_is_learned_at_its_own_doors() {
        // Mastery was declared, read by `effective_skills`, and written by
        // nothing — every dossier in the game read 0/10 forever.
        let tuning = MasteryTuning::default();
        let mut progression = Progression::default();

        for _ in 0..tuning.doors_for_first - 1 {
            assert!(!work_the_trade(&mut progression, &tuning));
        }
        assert_eq!(progression.mastery_level, 0);

        assert!(
            work_the_trade(&mut progression, &tuning),
            "the rank never came"
        );
        assert_eq!(progression.mastery_level, 1);
        assert_eq!(progression.specialty_doors, 0);
    }

    #[test]
    fn every_rank_costs_more_doors_than_the_one_before() {
        let tuning = MasteryTuning::default();
        let mut progression = Progression::default();
        let mut costs = Vec::new();

        for _ in 0..4 {
            let mut doors = 0;
            while !work_the_trade(&mut progression, &tuning) {
                doors += 1;
                assert!(doors < 500, "a rank that never arrives");
            }
            costs.push(doors + 1);
        }

        for pair in costs.windows(2) {
            assert!(pair[1] > pair[0], "the ladder is flat: {:?}", costs);
        }
    }

    #[test]
    fn mastery_stops_at_the_cap_and_stays_there() {
        let tuning = MasteryTuning::default();
        let mut progression = Progression::default();

        for _ in 0..5_000 {
            work_the_trade(&mut progression, &tuning);
        }

        assert_eq!(progression.mastery_level, tuning.max_level);
        assert_eq!(progression.specialty_doors, 0);
        assert!(!work_the_trade(&mut progression, &tuning));
    }

    #[test]
    fn a_rank_of_mastery_is_worth_a_point_of_the_trade() {
        // The reason it matters: mastery feeds straight into the specialty
        // skill, so ranks compound with the planning decision that earned them.
        let tuning = MasteryTuning::default();
        let attributes = attrs(10, 10, 10, 10, 10, 10);
        let mut progression = Progression::default();
        let before = effective_skills(
            &attributes,
            &Skills::default(),
            &progression,
            Skill::Stealth,
        )
        .get(Skill::Stealth);

        while !work_the_trade(&mut progression, &tuning) {}
        let after = effective_skills(
            &attributes,
            &Skills::default(),
            &progression,
            Skill::Stealth,
        )
        .get(Skill::Stealth);

        assert_eq!(after - before, 1);
    }

    #[test]
    fn mastery_lands_on_the_specialty_only() {
        let a = Attributes::default();
        let training = Skills::default();
        let progression = Progression {
            mastery_level: 5,
            ..Progression::default()
        };

        let skills = effective_skills(&a, &training, &progression, Skill::Hacking);
        assert_eq!(skills.hacking, 5);
        assert_eq!(skills.social, 0);
    }

    #[test]
    fn skills_never_go_negative() {
        let a = attrs(3, 3, 3, 3, 3, 3);
        let skills = effective_skills(
            &a,
            &Skills::default(),
            &Progression::default(),
            Skill::Social,
        );
        for skill in Skill::ALL {
            assert!(skills.get(skill) >= 0, "{} went negative", skill.label());
        }
    }

    #[test]
    fn the_level_curve_is_quadratic_and_cumulative() {
        assert_eq!(experience_to_next(1), 100);
        assert_eq!(experience_to_next(2), 400);
        assert_eq!(experience_to_next(3), 900);
        assert!(
            experience_to_next(5) - experience_to_next(4)
                > experience_to_next(2) - experience_to_next(1)
        );
        assert_eq!(total_experience(1), 0);
        assert_eq!(total_experience(2), 100);
        assert_eq!(total_experience(4), 100 + 400 + 900);
    }

    #[test]
    fn levelling_up_grants_one_attribute_and_two_skill_points() {
        let mut progression = Progression::default();
        let levels = award_experience(&mut progression, 100);

        assert_eq!(levels, 1);
        assert_eq!(progression.level, 2);
        assert_eq!(progression.experience, 0);
        assert_eq!(progression.experience_to_next, 400);
        assert_eq!(progression.attribute_points, 1);
        assert_eq!(progression.skill_points, 2);
    }

    #[test]
    fn a_large_award_can_carry_several_levels() {
        let mut progression = Progression::default();
        let levels = award_experience(&mut progression, 100 + 400 + 900);

        assert_eq!(levels, 3);
        assert_eq!(progression.level, 4);
        assert_eq!(progression.experience, 0);
    }

    #[test]
    fn proficiency_climbs_every_four_levels() {
        assert_eq!(proficiency_bonus(1), 2);
        assert_eq!(proficiency_bonus(4), 2);
        assert_eq!(proficiency_bonus(5), 3);
        assert_eq!(proficiency_bonus(9), 4);
    }

    #[test]
    fn power_level_counts_equipment_and_level() {
        let subject = member(
            attrs(12, 14, 10, 10, 10, 12),
            Skills::default(),
            Progression::default(),
        );
        let bare = power_level(&subject, &Loadout::empty());
        assert!(bare > 0);

        let higher = member(
            attrs(12, 14, 10, 10, 10, 12),
            Skills::default(),
            Progression {
                level: 5,
                ..Progression::default()
            },
        );
        assert!(power_level(&higher, &Loadout::empty()) > bare);
    }
}
