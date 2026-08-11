use super::*;
use crate::data::GameData;
use crate::model::crew::Injury;

fn setup(seed: u64) -> (GameData, GameSession) {
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config, &data, seed);
    (data, session)
}

#[test]
fn a_healthy_hand_is_quoted_nothing_and_cannot_be_treated() {
    let (data, mut session) = setup(1);
    let id = session.crew[0].id.clone();

    let quoted = quote(&session.crew[0].condition, &data.config.treatment);
    assert!(!quoted.is_needed());
    assert_eq!(quoted.cost, 0);

    let budget = session.budget;
    assert!(treat(&mut session, &data.config, &id).is_err());
    assert_eq!(session.budget, budget);
}

#[test]
fn a_serious_injury_costs_more_than_a_scrape() {
    let (data, mut session) = setup(2);
    let config = &data.config.treatment;

    session.crew[0].condition.injuries = vec![Injury::minor("Bruised ribs")];
    let minor = quote(&session.crew[0].condition, config).cost;
    session.crew[0].condition.injuries = vec![Injury::major("Torn shoulder")];
    let major = quote(&session.crew[0].condition, config).cost;

    assert!(major > minor, "a torn shoulder billed like a bruise");
}

#[test]
fn the_longer_the_wait_would_have_been_the_dearer_the_cure() {
    let (data, mut session) = setup(3);
    let config = &data.config.treatment;

    session.crew[0].condition.injuries = vec![Injury::major("Torn shoulder")];
    let fresh = quote(&session.crew[0].condition, config).cost;
    session.crew[0].condition.injuries[0].weeks_remaining = 1;
    let nearly_healed = quote(&session.crew[0].condition, config).cost;

    assert!(
        nearly_healed < fresh,
        "paying to skip one week cost the same as skipping three"
    );
}

#[test]
fn treatment_clears_everything_and_leaves_them_tired() {
    let (data, mut session) = setup(4);
    let id = session.crew[0].id.clone();
    session.crew[0].condition.injuries =
        vec![Injury::minor("Bruised ribs"), Injury::major("Bad hand")];
    session.crew[0].condition.fatigue = 20;
    session.budget = 5_000_000;

    let quoted = quote(&session.crew[0].condition, &data.config.treatment);
    assert!(treat(&mut session, &data.config, &id).is_ok());

    let member = session.member(&id).unwrap();
    assert!(member.condition.injuries.is_empty());
    assert_eq!(
        member.condition.fatigue,
        20 + data.config.treatment.fatigue_cost,
        "a body put back together in an afternoon knows it"
    );
    assert_eq!(session.budget, 5_000_000 - quoted.cost);
    assert_eq!(session.tally.injuries_treated, 2);
}

#[test]
fn a_treatment_nobody_can_afford_leaves_them_hurt() {
    let (data, mut session) = setup(5);
    let id = session.crew[0].id.clone();
    session.crew[0].condition.injuries = vec![Injury::major("Torn shoulder")];
    session.budget = 0;

    assert!(treat(&mut session, &data.config, &id).is_err());
    assert_eq!(session.member(&id).unwrap().condition.injuries.len(), 1);
    assert_eq!(session.budget, 0);
}

#[test]
fn treatment_puts_an_unfit_hand_back_to_work_the_same_week() {
    // The point of the verb: an injured hand was a fact to be waited out.
    // Now it is a bill, and paying it buys the week back.
    let (data, mut session) = setup(6);
    let id = session.crew[0].id.clone();
    session.crew[0].condition.injuries = vec![
        Injury::major("Torn shoulder"),
        Injury::major("Cracked rib"),
        Injury::minor("Sprain"),
    ];
    assert!(!data
        .config
        .condition
        .can_work(&session.member(&id).unwrap().condition));

    session.budget = 5_000_000;
    assert!(treat(&mut session, &data.config, &id).is_ok());
    assert!(data
        .config
        .condition
        .can_work(&session.member(&id).unwrap().condition));
}
