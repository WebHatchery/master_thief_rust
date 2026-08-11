use super::*;
use crate::data::GameData;

fn setup(seed: u64) -> (GameData, GameSession) {
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config, &data, seed);
    (data, session)
}

fn carrier(session: &GameSession) -> String {
    session
        .crew
        .iter()
        .find(|member| member.equipment.item_ids().count() > 0)
        .expect("somebody carries the starting kit")
        .id
        .clone()
}

#[test]
fn new_kit_costs_nothing_and_carries_no_penalty() {
    let (data, session) = setup(1);
    let member = session.member(&carrier(&session)).unwrap();

    assert_eq!(wear_penalty(&session, member, &data.config.kit), 0);
    assert!(!refit_quote(&session, member, &data.config.kit).is_needed());
}

#[test]
fn kit_wears_a_job_at_a_time_and_eventually_bites() {
    let (data, mut session) = setup(2);
    let id = carrier(&session);
    let config = &data.config.kit;

    for _ in 0..config.jobs_per_penalty {
        wear_kit(&mut session, &id);
    }

    let member = session.member(&id).unwrap();
    assert_eq!(wear_penalty(&session, member, config), 1);
    assert!(refit_quote(&session, member, config).is_needed());
}

#[test]
fn no_single_tool_can_ruin_a_hand_on_its_own() {
    let (data, mut session) = setup(3);
    let id = carrier(&session);
    let config = &data.config.kit;

    for _ in 0..config.jobs_per_penalty * (config.max_penalty_per_item as u32 + 10) {
        wear_kit(&mut session, &id);
    }

    let member = session.member(&id).unwrap();
    let carried = member.equipment.item_ids().count() as i32;
    assert_eq!(
        wear_penalty(&session, member, config),
        carried * config.max_penalty_per_item
    );
}

#[test]
fn a_refit_clears_the_penalty_and_the_money_is_gone() {
    let (data, mut session) = setup(4);
    let id = carrier(&session);
    for _ in 0..data.config.kit.jobs_per_penalty * 2 {
        wear_kit(&mut session, &id);
    }
    session.budget = 5_000_000;

    let member = session.member(&id).unwrap();
    let quoted = refit_quote(&session, member, &data.config.kit);
    assert!(quoted.penalty_cleared > 0);

    assert!(refit(&mut session, &data.config, &id).is_ok());
    let member = session.member(&id).unwrap();
    assert_eq!(wear_penalty(&session, member, &data.config.kit), 0);
    assert_eq!(session.budget, 5_000_000 - quoted.cost);
}

#[test]
fn the_longer_it_is_left_the_dearer_the_refit() {
    let (data, mut session) = setup(5);
    let id = carrier(&session);

    for _ in 0..3 {
        wear_kit(&mut session, &id);
    }
    let early = refit_quote(&session, session.member(&id).unwrap(), &data.config.kit).cost;
    for _ in 0..10 {
        wear_kit(&mut session, &id);
    }
    let late = refit_quote(&session, session.member(&id).unwrap(), &data.config.kit).cost;

    assert!(late > early, "neglect was free");
}

#[test]
fn a_refit_nobody_can_afford_leaves_the_kit_worn() {
    let (data, mut session) = setup(6);
    let id = carrier(&session);
    for _ in 0..20 {
        wear_kit(&mut session, &id);
    }
    session.budget = 0;

    assert!(refit(&mut session, &data.config, &id).is_err());
    assert!(wear_penalty(&session, session.member(&id).unwrap(), &data.config.kit) > 0);
}

#[test]
fn kit_nobody_is_carrying_does_not_wear() {
    let (data, mut session) = setup(7);
    let bare = session
        .crew
        .iter()
        .find(|member| member.equipment.item_ids().count() == 0)
        .map(|member| member.id.clone());

    if let Some(id) = bare {
        wear_kit(&mut session, &id);
        assert_eq!(
            wear_penalty(&session, session.member(&id).unwrap(), &data.config.kit),
            0
        );
    }
    assert!(session.kit_wear.values().all(|wear| *wear > 0));
}
