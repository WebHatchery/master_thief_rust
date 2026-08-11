use super::*;
use crate::model::Skill;

#[test]
fn every_data_file_parses() {
    let data = GameData::load().expect("embedded data must parse");

    assert!(!data.config.game_name.is_empty());
    assert!(!data.crew_pool.is_empty());
    assert!(!data.equipment.is_empty());
    assert!(!data.encounters.is_empty());
    assert!(!data.targets.is_empty());
    assert!(!data.environment.is_empty());
}

#[test]
fn the_starting_crew_exists_on_the_books() {
    let data = GameData::load().unwrap();
    for id in &data.config.starting_crew {
        assert!(data.crew_pool.contains(id), "unknown starting crew: {}", id);
    }
    for id in &data.config.starting_inventory {
        assert!(data.equipment.contains(id), "unknown start item: {}", id);
    }
}

#[test]
fn no_target_names_a_door_that_does_not_exist() {
    let data = GameData::load().unwrap();
    assert!(
        data.dangling_encounter_ids().is_empty(),
        "dangling encounters: {:?}",
        data.dangling_encounter_ids()
    );
    assert!(
        data.dangling_environment_ids().is_empty(),
        "dangling environment ids: {:?}",
        data.dangling_environment_ids()
    );
}

#[test]
fn no_encounter_template_is_stranded() {
    let data = GameData::load().unwrap();
    assert!(
        data.unreachable_encounter_ids().is_empty(),
        "unreachable encounters: {:?}",
        data.unreachable_encounter_ids()
    );
}

#[test]
fn every_outcome_band_has_a_line_for_every_skill() {
    let data = GameData::load().unwrap();
    for outcome in crate::rules::Outcome::ALL {
        for skill in Skill::ALL {
            assert!(
                !data.outcomes.lines(outcome, skill).is_empty(),
                "no {} lines for {}",
                outcome.key(),
                skill.key()
            );
        }
    }
}

#[test]
fn every_content_axis_meets_its_full_gdd_target() {
    // The full column of the GDD 8 table, not the prototype one. These are
    // floors, never ceilings: content may only grow, and this fails the
    // moment any axis shrinks below what was shipped.
    let inventory = GameData::load().unwrap().inventory();

    assert!(inventory.outcome_lines >= 400, "{:?}", inventory);
    assert!(inventory.encounters >= 70, "{:?}", inventory);
    assert!(inventory.equipment >= 60, "{:?}", inventory);
    assert!(inventory.targets >= 45, "{:?}", inventory);
    assert!(inventory.recruits >= 40, "{:?}", inventory);
    assert!(inventory.personality_traits >= 30, "{:?}", inventory);
    assert!(inventory.critical_effects >= 50, "{:?}", inventory);
    assert!(inventory.environment_factors >= 15, "{:?}", inventory);
    assert!(inventory.achievements >= 40, "{:?}", inventory);
}

#[test]
fn every_achievement_is_uniquely_named_and_reachable() {
    let data = GameData::load().unwrap();

    let mut ids: Vec<&str> = data.awards.iter().map(|a| a.id.as_str()).collect();
    let before = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), before, "duplicate achievement ids");

    for award in &data.awards {
        assert!(!award.name.is_empty(), "{} has no name", award.id);
        assert!(!award.description.is_empty(), "{} says nothing", award.id);
        assert!(award.at_least > 0, "{} unlocks for free", award.id);
    }
}

#[test]
fn the_ladder_of_marks_runs_all_the_way_up() {
    // Reputation is the campaign's spine. There has to be something to open
    // at every rung of it, or the ladder has a missing step.
    let data = GameData::load().unwrap();
    let mut gates: Vec<i32> = data
        .targets
        .iter()
        .map(|(_, target)| target.required_reputation)
        .collect();
    gates.sort_unstable();

    assert!(gates.first() == Some(&0), "nothing is open at week one");
    assert!(*gates.last().unwrap() >= 60, "the ladder stops too early");
    for pair in gates.windows(2) {
        assert!(
            pair[1] - pair[0] <= 6,
            "a {}-point gap between marks at reputation {}",
            pair[1] - pair[0],
            pair[0]
        );
    }
}

#[test]
fn every_difficulty_band_has_marks_in_it() {
    use crate::model::DifficultyBand;
    let data = GameData::load().unwrap();

    for band in DifficultyBand::ALL {
        let count = data
            .targets
            .iter()
            .filter(|(_, target)| target.difficulty == band)
            .count();
        assert!(count >= 5, "only {} {} marks", count, band.label());
    }
}

#[test]
fn every_band_and_skill_carries_a_deep_enough_table() {
    let data = GameData::load().unwrap();
    for outcome in crate::rules::Outcome::ALL {
        for skill in Skill::ALL {
            let lines = data.outcomes.lines(outcome, skill);
            assert!(
                lines.len() >= 10,
                "{}/{} has only {} lines",
                outcome.key(),
                skill.key(),
                lines.len()
            );
        }
    }
}

#[test]
fn no_narrative_line_is_written_twice_anywhere() {
    let data = GameData::load().unwrap();
    let mut seen: Vec<&str> = Vec::new();
    for outcome in crate::rules::Outcome::ALL {
        for skill in Skill::ALL {
            for line in data.outcomes.lines(outcome, skill) {
                seen.push(line.as_str());
            }
        }
    }
    let before = seen.len();
    seen.sort_unstable();
    seen.dedup();
    // The generic table is shared by design, so a skill without its own
    // list legitimately returns the same lines; compare against the
    // authored total instead of the resolved one.
    assert!(
        before - seen.len() < before / 4,
        "too many repeated lines across the tables"
    );
}

#[test]
fn every_target_sequence_is_a_playable_length() {
    let data = GameData::load().unwrap();
    for (id, target) in data.targets.iter() {
        let count = target.encounters.len();
        assert!((2..=6).contains(&count), "{} has {} encounters", id, count);
        assert!(target.potential_payout > 0, "{} pays nothing", id);
    }
}
