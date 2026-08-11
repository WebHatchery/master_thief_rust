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
