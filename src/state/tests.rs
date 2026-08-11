use super::*;

fn data() -> GameData {
    GameData::load().unwrap()
}

#[test]
fn a_new_campaign_hires_the_starting_crew_and_stocks_the_board() {
    let data = data();
    let session = GameSession::new(&data.config, &data, 42);

    assert_eq!(session.crew.len(), data.config.starting_crew.len());
    assert_eq!(session.budget, data.config.starting_budget);
    assert!(!session.board.is_empty());
    assert!(session.board.len() <= data.config.targets_on_board);
}

#[test]
fn a_save_round_trips_through_json() {
    let data = data();
    let mut session = GameSession::new(&data.config, &data, 99);
    session.budget -= 4200;
    session.heat = 31;
    session.crew[0].condition.fatigue = 45;
    session.scrutiny.note(
        crate::model::Skill::Hacking,
        data.config.scrutiny.ceiling(),
        &data.config.scrutiny,
    );

    let save = session.to_save(&data.config.version);
    let encoded = serde_json::to_value(&save).unwrap();
    let restored = migrate_save_value(
        Some(data.config.version.clone()),
        serde_json::json!({ "data": encoded }),
        &data.config,
    )
    .unwrap();

    assert_eq!(
        serde_json::to_value(GameSession::from_save(restored)).unwrap(),
        serde_json::to_value(&session).unwrap()
    );
}

#[test]
fn a_save_the_game_cannot_read_is_reported_not_swallowed() {
    let data = data();
    let result = migrate_save_value(
        Some("0.0.1".to_owned()),
        serde_json::json!({ "data": { "nonsense": true } }),
        &data.config,
    );
    assert!(result.is_err());
}

#[test]
fn heat_only_bites_above_the_safe_threshold() {
    let data = data();
    let mut session = GameSession::new(&data.config, &data, 3);

    session.heat = data.config.heat_safe_threshold;
    assert_eq!(session.heat_dc_penalty(&data.config), 0);

    session.heat = data.config.heat_safe_threshold + data.config.heat_dc_step * 2;
    assert_eq!(session.heat_dc_penalty(&data.config), 2);
}

#[test]
fn a_known_outfit_pays_danger_money_to_sign_anybody() {
    // Pillar 4: the two axes pull opposite ways. Reputation opened marks
    // and notoriety priced two rare purchases, which is not opposition —
    // being known now costs the outfit the thing reputation buys most of.
    let data = data();
    let mut session = GameSession::new(&data.config, &data, 60);
    let recruit = data
        .crew_pool
        .get(session.recruits.first().expect("somebody is asking"))
        .unwrap();

    let quiet = session.hire_fee(recruit, &data.config);
    assert_eq!(quiet, recruit.hire_cost, "an unknown outfit pays list");

    session.notoriety = 120;
    let known = session.hire_fee(recruit, &data.config);
    assert!(known > quiet, "infamy was free at the hiring table");

    session.notoriety = 100_000;
    let infamous = session.hire_fee(recruit, &data.config);
    assert_eq!(
        infamous,
        recruit.hire_cost
            + (recruit.hire_cost as f32 * data.config.recruiting.max_fee_premium) as i64,
        "the premium has to stop somewhere"
    );
}

#[test]
fn fewer_people_turn_up_for_an_outfit_everyone_is_watching() {
    let data = data();
    let mut session = GameSession::new(&data.config, &data, 61);
    assert_eq!(
        session.applicants_this_week(&data.config),
        data.config.recruit_pool_size
    );

    session.notoriety = data.config.recruiting.pool_shrink_per_notoriety;
    assert_eq!(
        session.applicants_this_week(&data.config),
        data.config.recruit_pool_size - 1
    );

    // Somebody is always desperate enough.
    session.notoriety = 100_000;
    assert_eq!(
        session.applicants_this_week(&data.config),
        data.config.recruiting.min_pool
    );
}

#[test]
fn the_premium_is_actually_charged_and_not_merely_displayed() {
    let data = data();
    let mut session = GameSession::new(&data.config, &data, 62);
    session.notoriety = 150;
    session.budget = 10_000_000;
    let id = session.recruits[0].clone();
    let fee = session.hire_fee(data.crew_pool.get(&id).unwrap(), &data.config);

    session.hire(&data, &id).unwrap();
    assert_eq!(session.budget, 10_000_000 - fee);
}

#[test]
fn the_opening_lockup_is_issued_to_the_people_who_can_use_it() {
    let data = data();
    let session = GameSession::new(&data.config, &data, 5);

    let carried: usize = session
        .crew
        .iter()
        .map(|member| member.equipment.item_ids().count())
        .sum();
    assert_eq!(carried, data.config.starting_inventory.len());
    assert!(session.unassigned_inventory(&data).is_empty());
}

#[test]
fn kit_taken_off_a_hand_reappears_in_the_lockup() {
    let data = data();
    let mut session = GameSession::new(&data.config, &data, 5);
    let (slot, item) = session
        .crew
        .iter()
        .find_map(|member| {
            crate::model::EquipmentSlot::ALL
                .into_iter()
                .find_map(|slot| member.equipment.get(slot).map(|id| (slot, id.to_owned())))
        })
        .expect("somebody is carrying the starting kit");

    let holder = session
        .crew
        .iter()
        .position(|member| member.equipment.get(slot) == Some(item.as_str()))
        .unwrap();
    session.crew[holder].equipment.set(slot, None);

    let lockup: Vec<&str> = session
        .unassigned_inventory(&data)
        .iter()
        .map(|def| def.id.as_str())
        .collect();
    assert_eq!(lockup, vec![item.as_str()]);
}

#[test]
fn a_hand_too_hurt_to_go_is_not_offered_for_work_and_a_tired_one_still_is() {
    // The distinction the change turns on: injuries are a wall, tiredness
    // is a price. Exhaustion used to take a name off this list, which made
    // "rest until everybody is fresh" the one move the week never argued
    // with (GDD 5.6).
    let data = data();
    let tuning = &data.config.condition;
    let mut session = GameSession::new(&data.config, &data, 5);
    assert_eq!(session.available_crew(tuning).count(), session.crew.len());

    session.crew[0].condition.fatigue = 95;
    assert_eq!(
        session.available_crew(tuning).count(),
        session.crew.len(),
        "a spent hand was taken out of the fixer's hands"
    );

    session.crew[0].condition.injuries = (0..tuning.max_injuries_for_work + 1)
        .map(|n| crate::model::crew::Injury::major(format!("Hurt {}", n)))
        .collect();
    assert_eq!(
        session.available_crew(tuning).count(),
        session.crew.len() - 1
    );
}
