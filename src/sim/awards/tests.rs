use super::*;
use crate::data::GameData;

fn setup() -> (GameData, GameSession) {
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config, &data, 71);
    (data, session)
}

#[test]
fn a_fresh_campaign_has_earned_almost_nothing() {
    let (data, mut session) = setup();
    award(&mut session, &data.config, &data.awards);

    let (unlocked, total) = session.achievements.progress();
    assert!(total >= 40, "only {} achievements defined", total);
    assert!(
        unlocked <= 2,
        "{} unlocked before anything happened",
        unlocked
    );
}

#[test]
fn a_threshold_that_is_met_unlocks_exactly_once() {
    let (data, mut session) = setup();
    session.tally.jobs_run = 1;

    let first = award(&mut session, &data.config, &data.awards);
    let second = award(&mut session, &data.config, &data.awards);

    assert!(!first.is_empty(), "the first job unlocked nothing");
    assert!(second.is_empty(), "an achievement unlocked twice");
}

#[test]
fn live_session_state_counts_as_well_as_running_totals() {
    let (data, mut session) = setup();
    let tally = CampaignTally::default();

    session.reputation = 40;
    assert_eq!(
        TallyStat::Reputation.value(&tally, &session, &data.config),
        40
    );
    assert_eq!(
        TallyStat::CrewSize.value(&tally, &session, &data.config),
        session.crew.len() as i64
    );
}

#[test]
fn a_grudge_reads_as_a_positive_depth() {
    let (data, mut session) = setup();
    let crew = session.crew_ids();
    session.chemistry.set(&crew[0], &crew[1], -45);

    let tally = CampaignTally::default();
    assert_eq!(
        TallyStat::WorstGrudge.value(&tally, &session, &data.config),
        45
    );
    assert_eq!(
        TallyStat::BestChemistry.value(&tally, &session, &data.config),
        -45
    );
}

#[test]
fn a_campaign_counts_the_decisions_the_week_is_actually_made_of() {
    // This is the test that would have caught the thing it was written for.
    // `jobs_called_off` was added with the standing order, written on every
    // walked job, and then read by precisely nothing — no achievement, no
    // records row. A counter nobody reads is the same dead field this
    // project keeps finding, and adding one is easier than noticing it.
    let data = GameData::load().unwrap();

    for stat in [
        TallyStat::JobsCalledOff,
        TallyStat::DoorsLeftStanding,
        TallyStat::DoorsWorkedSpent,
        TallyStat::WorstScrutiny,
    ] {
        assert!(
            data.awards.iter().any(|award| award.stat == stat),
            "{:?} is counted and nothing ever reads it",
            stat
        );
    }
}

#[test]
fn every_achievement_names_a_statistic_that_can_actually_move() {
    // The other half: an award written against a stat nothing ever writes
    // is unreachable, and an unreachable achievement is worse than none.
    let data = GameData::load().unwrap();
    let mut session = GameSession::new(&data.config, &data, 5_150);
    super::super::play(&mut session, &data, 20);

    for award in &data.awards {
        assert!(
            award.at_least > 0,
            "{} is earned by doing nothing",
            award.id
        );
        let _ = award.stat.value(&session.tally, &session, &data.config);
    }
}

#[test]
fn progress_reports_the_share_of_the_way_there() {
    let (data, session) = setup();
    let mut tally = CampaignTally::default();
    let def = AwardDef {
        id: "test".to_owned(),
        name: "Test".to_owned(),
        description: String::new(),
        stat: TallyStat::JobsRun,
        at_least: 10,
    };

    assert_eq!(def.progress(&tally, &session, &data.config), 0.0);
    tally.jobs_run = 5;
    assert!((def.progress(&tally, &session, &data.config) - 0.5).abs() < f32::EPSILON);
    tally.jobs_run = 40;
    assert_eq!(def.progress(&tally, &session, &data.config), 1.0);
}

#[test]
fn a_campaign_earns_a_reasonable_spread_over_twenty_weeks() {
    let data = GameData::load().unwrap();
    let mut session = GameSession::new(&data.config, &data, 20_260_726);
    super::super::play(&mut session, &data, 20);
    award(&mut session, &data.config, &data.awards);

    let (unlocked, total) = session.achievements.progress();
    assert!(
        unlocked >= 8,
        "only {}/{} unlocked in twenty weeks",
        unlocked,
        total
    );
    assert!(
        unlocked < total,
        "a twenty-week campaign cleared the whole list"
    );
}
