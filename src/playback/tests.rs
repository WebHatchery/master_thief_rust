use super::*;
use crate::data::GameData;
use crate::sim;
use crate::state::GameSession;

fn report(seed: u64) -> JobReport {
    let data = GameData::load().unwrap();
    let mut session = GameSession::new(&data.config, &data, seed);
    let target = data
        .targets
        .get(&session.board[0].target_id)
        .unwrap()
        .clone();
    let plan = sim::auto_assign(&session, &data, &target);
    sim::run_job(&mut session, &data, &plan)
}

fn run_to_end(playback: &mut RunPlayback) {
    for _ in 0..10_000 {
        if playback.finished() {
            return;
        }
        playback.update(1.0 / 60.0, false);
    }
    panic!("playback never finished");
}

#[test]
fn a_run_opens_on_the_first_door_approaching() {
    let playback = RunPlayback::new(report(101));
    assert_eq!(playback.door_index(), 0);
    assert_eq!(playback.phase(), DoorPhase::Approach);
    assert!(!playback.finished());
}

#[test]
fn the_beats_arrive_in_order() {
    let mut playback = RunPlayback::new(report(102));
    let mut seen = vec![playback.phase()];

    for _ in 0..40 {
        playback.update(0.1, false);
        let phase = playback.phase();
        if seen.last() != Some(&phase) {
            seen.push(phase);
        }
        if playback.door_index() > 0 {
            break;
        }
    }

    assert_eq!(
        &seen[..4],
        &[
            DoorPhase::Approach,
            DoorPhase::Roll,
            DoorPhase::Tally,
            DoorPhase::Verdict
        ]
    );
}

#[test]
fn the_die_is_still_in_the_air_until_the_roll_beat_ends() {
    let mut playback = RunPlayback::new(report(103));
    assert!(!playback.roll_landed());

    playback.update(APPROACH + ROLL * 0.5, false);
    assert!(!playback.roll_landed());

    playback.update(ROLL, false);
    assert!(playback.roll_landed());
}

#[test]
fn modifiers_stack_one_at_a_time_and_finish_complete() {
    let mut playback = RunPlayback::new(report(104));
    assert_eq!(playback.revealed_modifiers(), 0);

    playback.update(APPROACH + ROLL + 0.001, false);
    let early = playback.revealed_modifiers();
    playback.update(TALLY * 0.5, false);
    let mid = playback.revealed_modifiers();

    assert!(early <= mid);
    playback.update(TALLY, false);

    let total = playback
        .current_door()
        .unwrap()
        .result
        .check
        .significant()
        .count();
    assert_eq!(playback.revealed_modifiers(), total);
}

#[test]
fn the_running_total_arrives_at_the_number_the_engine_rolled() {
    let mut playback = RunPlayback::new(report(105));
    playback.update(APPROACH + ROLL + TALLY + 0.01, false);

    let door = playback.current_door().unwrap();
    // `significant()` hides zero-valued modifiers, so the tally lands on the
    // roll plus every line the player was actually shown.
    let shown: i32 = door.result.check.significant().map(|e| e.value).sum();
    assert_eq!(playback.running_total(), door.result.roll + shown);
}

#[test]
fn the_floorplan_only_lights_doors_that_have_resolved() {
    let mut playback = RunPlayback::new(report(106));
    assert!(playback.outcomes_so_far().iter().all(|o| o.is_none()));

    playback.update(APPROACH + ROLL + TALLY + 0.01, false);
    let lit = playback.outcomes_so_far();
    assert!(lit[0].is_some(), "the door being read should be lit");
    assert!(
        lit.iter().skip(1).all(|o| o.is_none()),
        "doors ahead must stay dark"
    );
}

#[test]
fn a_verdict_is_reported_once_when_the_door_lands() {
    let mut playback = RunPlayback::new(report(111));
    playback.update(APPROACH + ROLL + TALLY + 0.01, false);

    assert!(playback.verdict_just_landed(DoorPhase::Tally, 0));
    assert!(!playback.verdict_just_landed(DoorPhase::Verdict, 0));
}

#[test]
fn a_run_reaches_its_last_door_and_stops() {
    let mut playback = RunPlayback::new(report(107));
    let doors = playback.report().doors.len();
    run_to_end(&mut playback);

    assert!(playback.finished());
    assert_eq!(playback.door_index(), doors - 1);
    assert!(playback.outcomes_so_far().iter().all(|o| o.is_some()));
}

#[test]
fn holding_the_skip_key_gets_there_sooner_and_changes_nothing_else() {
    let mut slow = RunPlayback::new(report(108));
    let mut fast = RunPlayback::new(report(108));

    for _ in 0..60 {
        slow.update(1.0 / 60.0, false);
        fast.update(1.0 / 60.0, true);
    }

    assert!(fast.door_index() >= slow.door_index());
    assert_eq!(
        fast.report().doors.len(),
        slow.report().doors.len(),
        "fast-forward must not skip a door"
    );
}

#[test]
fn skipping_to_the_end_leaves_the_report_intact() {
    let mut playback = RunPlayback::new(report(109));
    let expected: Vec<Outcome> = playback
        .report()
        .doors
        .iter()
        .map(|door| door.result.outcome)
        .collect();

    playback.skip_to_end();
    assert!(playback.finished());

    let report = playback.into_report();
    let actual: Vec<Outcome> = report.doors.iter().map(|d| d.result.outcome).collect();
    assert_eq!(actual, expected);
}

#[test]
fn a_job_with_no_doors_is_finished_the_moment_it_starts() {
    let mut empty = report(110);
    empty.doors.clear();
    let playback = RunPlayback::new(empty);

    assert!(playback.finished());
    assert!(playback.current_door().is_none());
    assert_eq!(playback.running_total(), 0);
}
