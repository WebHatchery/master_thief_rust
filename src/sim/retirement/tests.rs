use super::*;

fn setup(seed: u64) -> (GameData, GameSession) {
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config, &data, seed);
    (data, session)
}

#[test]
fn the_take_is_the_cash_and_what_the_lockup_fetches() {
    let (data, mut session) = setup(1);
    session.budget = 500_000;

    let quoted = quote(&session, &data, &data.config);
    assert_eq!(quoted.cash, 500_000);
    assert!(quoted.lockup > 0, "the starting kit is worth something");
    assert_eq!(quoted.take(), quoted.cash + quoted.lockup);
}

#[test]
fn getting_out_while_the_city_is_watching_costs_real_money() {
    // The last decision the campaign asks for: cool off, then walk.
    let (data, mut session) = setup(2);
    session.budget = 100_000;

    session.heat = 0;
    let quiet = quote(&session, &data, &data.config).take();
    session.heat = 200;
    let hunted = quote(&session, &data, &data.config).take();

    assert!(
        hunted < quiet,
        "a hunted outfit liquidated at a calm outfit's price"
    );
}

#[test]
fn retiring_ends_it_and_cannot_be_done_twice() {
    let (data, mut session) = setup(3);
    session.budget = 250_000;

    let first = retire(&mut session, &data, &data.config);
    assert!(first.is_ok());
    assert!(session.retired.is_some());
    assert_eq!(
        session.tally.final_take,
        session.retired.as_ref().unwrap().take()
    );

    assert!(retire(&mut session, &data, &data.config).is_err());
}

#[test]
fn anybody_still_in_a_cell_is_named_and_stays_there() {
    let (data, mut session) = setup(4);
    session.heat = 90;
    crate::sim::law::tests_support_arrest(&mut session, &data.config);

    let quoted = quote(&session, &data, &data.config);
    assert_eq!(quoted.left_behind.len(), 1);
    assert!(!quoted.left_behind[0].is_empty());
}

#[test]
fn the_reckoning_reads_as_a_sentence() {
    let (data, mut session) = setup(5);
    session.budget = 1_250_000;
    session.week = 31;

    let headline = retire(&mut session, &data, &data.config).unwrap();
    assert!(headline.contains("week 31"));
    assert!(headline.contains('$'));
}
