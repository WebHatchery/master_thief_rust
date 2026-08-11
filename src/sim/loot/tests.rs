use super::*;

fn setup() -> (GameData, HeistTarget) {
    let data = GameData::load().unwrap();
    let target = data.targets.get("velvet_room").unwrap().clone();
    (data, target)
}

#[test]
fn a_failed_job_leaves_with_nothing() {
    let (data, target) = setup();
    let mut rng = SeededRng::new(1);
    let found = roll_loot(
        &mut rng,
        &data,
        &target,
        &[Outcome::CriticalFailure; 3],
        false,
    );
    assert!(found.is_empty());
}

#[test]
fn a_target_with_nothing_worth_taking_drops_nothing() {
    let (data, mut target) = setup();
    target.possible_loot.clear();
    let mut rng = SeededRng::new(2);
    assert!(roll_loot(&mut rng, &data, &target, &[Outcome::Success], true).is_empty());
}

#[test]
fn everything_dropped_is_something_the_catalogue_knows() {
    let (data, target) = setup();
    let mut rng = SeededRng::new(3);

    for _ in 0..200 {
        for id in roll_loot(&mut rng, &data, &target, &[Outcome::Success], true) {
            assert!(data.equipment.contains(&id), "unknown drop: {}", id);
            assert!(target.possible_loot.contains(&id));
        }
    }
}

#[test]
fn an_id_no_longer_in_the_catalogue_is_skipped_rather_than_crashing() {
    let (data, mut target) = setup();
    target.possible_loot = vec!["a_tool_that_was_cut".to_owned()];
    let mut rng = SeededRng::new(4);
    assert!(roll_loot(&mut rng, &data, &target, &[Outcome::Success], true).is_empty());
}

#[test]
fn brilliance_is_where_the_extra_things_turn_up() {
    let (data, target) = setup();

    let mut plain = 0usize;
    let mut brilliant = 0usize;
    for seed in 0..300u64 {
        let mut rng = SeededRng::new(seed);
        plain += roll_loot(&mut rng, &data, &target, &[Outcome::Success; 3], true).len();
        let mut rng = SeededRng::new(seed);
        brilliant += roll_loot(
            &mut rng,
            &data,
            &target,
            &[Outcome::CriticalSuccess; 3],
            true,
        )
        .len();
    }

    assert!(
        brilliant > plain,
        "criticals should pay: {} vs {}",
        brilliant,
        plain
    );
}

#[test]
fn the_same_seed_finds_the_same_things() {
    let (data, target) = setup();
    let mut a = SeededRng::new(20260726);
    let mut b = SeededRng::new(20260726);

    assert_eq!(
        roll_loot(&mut a, &data, &target, &[Outcome::CriticalSuccess], true),
        roll_loot(&mut b, &data, &target, &[Outcome::CriticalSuccess], true)
    );
}
