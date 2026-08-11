use super::*;
use crate::data::GameData;

fn setup(seed: u64) -> (GameData, GameSession) {
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config, &data, seed);
    (data, session)
}

#[test]
fn a_quiet_outfit_is_never_troubled() {
    let (data, mut session) = setup(1);
    session.heat = data.config.law.attention_threshold;

    assert_eq!(attention_chance(&session, &data.config.law), 0.0);
    for _ in 0..50 {
        assert!(roll_attention(&mut session, &data.config).is_none());
    }
}

#[test]
fn the_risk_climbs_with_the_heat_and_then_stops() {
    let (data, mut session) = setup(2);
    let law = &data.config.law;

    session.heat = law.attention_threshold + 10;
    let mild = attention_chance(&session, law);
    session.heat = law.attention_threshold + 30;
    let bad = attention_chance(&session, law);
    session.heat = 10_000;

    assert!(bad > mild && mild > 0.0);
    assert_eq!(attention_chance(&session, law), law.attention_chance_max);
}

#[test]
fn a_hot_outfit_eventually_gets_a_visit() {
    let (data, mut session) = setup(3);
    session.heat = data.config.law.custody_threshold + 20;
    session.budget = 500_000;

    let mut events = Vec::new();
    for _ in 0..80 {
        session.heat = data.config.law.custody_threshold + 20;
        if let Some(event) = roll_attention(&mut session, &data.config) {
            events.push(event.kind);
        }
    }

    assert!(!events.is_empty(), "eighty hot weeks and nobody called");
    assert!(
        events.contains(&LawEventKind::Arrest),
        "custody never happened: {:?}",
        events
    );
}

#[test]
fn an_arrest_takes_the_hand_the_city_has_seen_most_of() {
    let (data, mut session) = setup(4);
    session.crew[1].progression.jobs_completed = 12;
    let expected = session.crew[1].name.clone();
    let roster = session.crew.len();

    let event = arrest(&mut session, &data.config.law);

    assert_eq!(event.taken.as_deref(), Some(expected.as_str()));
    assert_eq!(session.crew.len(), roster - 1);
    assert_eq!(session.custody.len(), 1);
    assert!(session.custody[0].bail > 0);
}

#[test]
fn bail_returns_them_rested_and_unimpressed() {
    let (data, mut session) = setup(5);
    session.crew[0].condition.fatigue = 70;
    session.crew[0]
        .condition
        .injuries
        .push(crate::model::crew::Injury::major("Cracked rib"));
    arrest(&mut session, &data.config.law);

    let id = session.custody[0].member.id.clone();
    let bail = session.custody[0].bail;
    session.budget = bail + 10;

    assert!(post_bail(&mut session, &data.config, &id).is_ok());
    assert_eq!(session.budget, 10);
    assert!(session.custody.is_empty());

    let back = session.member(&id).expect("they are on the payroll again");
    assert_eq!(back.condition.fatigue, 0);
    assert!(back.condition.injuries.is_empty());
    assert_eq!(back.condition.loyalty, data.config.law.bail_return_loyalty);
}

#[test]
fn bail_nobody_can_afford_leaves_them_where_they_are() {
    let (data, mut session) = setup(6);
    arrest(&mut session, &data.config.law);
    let id = session.custody[0].member.id.clone();
    session.budget = 0;

    assert!(post_bail(&mut session, &data.config, &id).is_err());
    assert_eq!(session.custody.len(), 1);
}

#[test]
fn a_raid_takes_a_share_of_the_cash_and_cools_the_city_a_little() {
    let (data, mut session) = setup(7);
    session.budget = 200_000;
    session.heat = 90;

    let event = raid(&mut session, &data.config.law);

    assert!(event.cash_lost > 0);
    assert_eq!(session.budget, 200_000 - event.cash_lost);
    assert!(session.heat < 90);
}

#[test]
fn heat_can_be_bought_down_and_notoriety_never_can() {
    // GDD 12, open question 5, settled: the ledger is monotonic, the
    // short-term half is not.
    let (data, mut session) = setup(8);
    session.heat = 60;
    session.notoriety = 40;
    session.budget = 1_000_000;

    let cost = bribe_cost(&session, &data.config.law);
    assert!(grease_palms(&mut session, &data.config).is_ok());

    assert_eq!(session.heat, 60 - data.config.law.bribe_heat_relief);
    assert_eq!(session.notoriety, 40, "notoriety is the ledger");
    assert_eq!(session.budget, 1_000_000 - cost);
}

#[test]
fn silence_gets_more_expensive_as_the_name_grows() {
    let (data, mut session) = setup(9);
    session.notoriety = 5;
    let early = bribe_cost(&session, &data.config.law);
    session.notoriety = 90;

    assert!(bribe_cost(&session, &data.config.law) > early);
}

#[test]
fn there_is_nothing_to_buy_when_nobody_is_looking() {
    let (data, mut session) = setup(10);
    session.heat = 0;
    session.budget = 1_000_000;

    assert!(grease_palms(&mut session, &data.config).is_err());
    assert_eq!(session.budget, 1_000_000);
}
