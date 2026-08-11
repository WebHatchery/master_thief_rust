use super::super::{auto_assign, run_job};
use super::*;
use crate::rules::Scrutiny;

fn setup(seed: u64) -> (GameData, GameSession) {
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config, &data, seed);
    (data, session)
}

fn first_target<'a>(data: &'a GameData, session: &GameSession) -> &'a HeistTarget {
    data.targets.get(&session.board[0].target_id).unwrap()
}

#[test]
fn waiting_on_a_mark_pays_more_than_taking_it_fresh() {
    // The bet the ripening creates: the same job, the same seed, the same
    // plan — worth measurably more for having been left alone.
    let data = GameData::load().unwrap();
    let target = data.targets.get("velvet_room").unwrap().clone();

    let payout_at = |ripeness: u32| {
        let mut session = GameSession::new(&data.config, &data, 8_080);
        session.board.retain(|entry| entry.target_id == target.id);
        if session.board.is_empty() {
            session
                .board
                .push(crate::state::BoardEntry::new(&target.id));
        }
        session.board[0].ripeness = ripeness;
        // The doors are harder, so hold the dice still and read the money.
        for member in &mut session.crew {
            member.training = crate::model::Skills {
                stealth: 40,
                athletics: 40,
                combat: 40,
                lockpicking: 40,
                hacking: 40,
                social: 40,
            };
        }
        let plan = auto_assign(&session, &data, &target);
        run_job(&mut session, &data, &plan).payout
    };

    let fresh = payout_at(0);
    let ripe = payout_at(data.config.board.ripeness_max);
    assert!(
        ripe > fresh,
        "a mark left three weeks paid {} against {} taken fresh",
        ripe,
        fresh
    );
}

#[test]
fn the_report_shows_the_crew_taking_their_share_of_the_gross() {
    // The results screen reads all three numbers, so the job has to settle
    // them consistently: gross, what the crew took, what reached the outfit.
    let (data, mut session) = setup(2_468);
    let target = first_target(&data, &session).clone();
    let plan = auto_assign(&session, &data, &target);

    let report = run_job(&mut session, &data, &plan);

    assert!(report.gross > 0);
    assert_eq!(report.cut.net_of(report.gross), report.payout);
    assert!(report.payout < report.gross, "the crew worked for nothing");
    assert!(
        !report.cut.reasons.is_empty(),
        "the share moved off the base rate and said nothing about why"
    );
}

#[test]
fn a_finished_mark_leaves_the_board() {
    let (data, mut session) = setup(606);
    let target = first_target(&data, &session).clone();
    let plan = auto_assign(&session, &data, &target);

    run_job(&mut session, &data, &plan);
    assert!(session.board_entry(&target.id).is_none());
}

#[test]
fn every_door_the_crew_worked_teaches_the_city_its_trade() {
    let (data, mut session) = setup(1_705);
    let target = first_target(&data, &session).clone();
    let plan = auto_assign(&session, &data, &target);
    assert!(session.scrutiny.is_empty());

    let report = run_job(&mut session, &data, &plan);

    assert!(
        !report.trades_noticed.is_empty(),
        "a whole job and the city learned nothing"
    );
    for (skill, attention) in &report.trades_noticed {
        assert!(*attention > 0);
        assert_eq!(session.scrutiny.get(*skill), *attention);
    }
    let doors_by_trade: usize = report.trades_noticed.len();
    assert!(doors_by_trade <= report.doors.len());
}

#[test]
fn a_job_walked_out_of_is_scored_against_the_building_not_the_doors_tried() {
    // Two of three cleared and out is two thirds of a job. Scoring it on
    // what was attempted would call it a clean sweep, which is the opposite
    // of what happened — and would have paid like one.
    let (data, mut session) = setup(9_140);
    let target = first_target(&data, &session).clone();
    let mut plan = auto_assign(&session, &data, &target);
    plan.delegated = false;
    plan.walk_after = Some(1);
    let doors = plan.assignments.len();

    let report = run_job(&mut session, &data, &plan);

    assert_eq!(report.doors_total(), doors.max(report.doors.len()));
    assert!(report.success_rate() <= 1.0);
    if report.was_called_off() {
        assert!(
            report.success_rate() < 1.0,
            "a job the crew walked out of read as every door cleared"
        );
    }
}

#[test]
fn every_door_cleared_is_worth_something_even_on_a_job_being_lost() {
    // A failed job used to pay a flat share of the mark whatever happened,
    // so on a job the crew were losing, getting one more door open was
    // worth exactly nothing — and being wiped out paid the same as coming
    // one door short. Both halves of that are now false.
    let data = GameData::load().unwrap();
    let target = data.targets.get("velvet_room").unwrap().clone();
    let worth = target.potential_payout as f32;
    let pay = &data.config.payout;

    let failed = |cleared: usize, total: usize| {
        let rate = cleared as f32 / total as f32;
        assert!(rate < pay.success_threshold, "that is not a failed job");
        (worth * rate * pay.failed_share) as i64
    };

    assert_eq!(failed(0, 3), 0, "a total wipeout still paid out");
    assert!(failed(1, 3) > failed(0, 3));
    assert!(failed(1, 4) > 0);
    assert!(
        failed(1, 3) > failed(1, 4),
        "getting a third of the way in paid the same as a quarter"
    );
}

#[test]
fn staying_pays_better_per_door_than_leaving_and_the_score_beats_both() {
    // The three shares have to stay in this order or the standing order
    // stops being a trade: leaving buys safety at a price, staying is worth
    // more per door because they were in there longer, and neither is worth
    // finishing the job.
    let pay = &GameData::load().unwrap().config.payout;

    assert!(pay.walked_share < pay.failed_share);
    assert!(pay.failed_share < 1.0);
    assert!(pay.clean_bonus > 1.0);
    assert!(pay.clean_threshold > pay.success_threshold);
}

#[test]
fn walking_late_is_worth_more_than_walking_early() {
    // The curve that makes the standing order a judgement rather than a
    // switch: a tighter order is safer and poorer, and the player is
    // choosing where on that line to sit.
    let data = GameData::load().unwrap();
    let target = data.targets.get("velvet_room").unwrap().clone();
    let worth = target.potential_payout as f32;
    let pay = &data.config.payout;

    let quoted = |cleared: usize, total: usize| {
        (worth * (cleared as f32 / total as f32) * pay.walked_share) as i64
    };

    assert!(quoted(2, 3) > quoted(1, 3));
    assert!(quoted(1, 3) > quoted(0, 3));
    // And never as much as finishing it: the forfeited score is the price.
    assert!(quoted(3, 3) < (worth * 1.0) as i64);
}

#[test]
fn getting_in_teaches_the_city_more_than_being_beaten_does() {
    // Otherwise the pressure would land hardest on the crew already having
    // the worst week, which is the wrong shape for a cost the player is
    // meant to spend down deliberately.
    let tuning = &GameData::load().unwrap().config.scrutiny;
    assert!(tuning.per_door_cleared > tuning.per_door_failed);
    assert!(tuning.per_door_failed > 0, "a failed door taught nothing");
}

#[test]
fn a_file_the_city_already_has_shows_up_on_the_next_job_by_name() {
    // The whole contract: what the settlement writes has to be readable on
    // the planning screen before the next commit (pillar 2).
    let (data, mut session) = setup(1_706);
    let target = first_target(&data, &session).clone();
    let door = data.encounters_for(&target)[0].clone();
    let trade = door.primary_skill;

    session
        .scrutiny
        .note(trade, data.config.scrutiny.ceiling(), &data.config.scrutiny);
    let check =
        crate::sim::plan::candidate_check(&session, &data, &target, &door, &session.crew[0], &[]);

    let entry = check
        .entries
        .iter()
        .find(|entry| entry.label == Scrutiny::label(trade))
        .expect("the city's file is charged by name");
    assert_eq!(entry.value, -data.config.scrutiny.max_penalty);
}
