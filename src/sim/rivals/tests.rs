use super::*;
use crate::data::GameData;

fn setup(seed: u64) -> (GameData, GameSession) {
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config, &data, seed);
    (data, session)
}

#[test]
fn a_fresh_mark_interests_nobody() {
    let (data, session) = setup(1);
    for entry in &session.board {
        assert_eq!(entry.ripeness, 0);
        assert_eq!(interest_in(entry, &data.config.rivals), 0.0);
    }
}

#[test]
fn the_longer_a_mark_sits_the_more_people_notice() {
    let (data, mut session) = setup(2);
    let rivals = &data.config.rivals;

    session.board[0].ripeness = rivals.min_ripeness;
    let early = interest_in(&session.board[0], rivals);
    session.board[0].ripeness = rivals.min_ripeness + 3;
    let late = interest_in(&session.board[0], rivals);

    assert!(early > 0.0);
    assert!(late > early);
    session.board[0].ripeness = 500;
    assert_eq!(interest_in(&session.board[0], rivals), rivals.max_chance);
}

#[test]
fn a_ripe_board_eventually_loses_something() {
    let (data, mut session) = setup(3);
    let mut taken = 0;

    for _ in 0..200 {
        if session.board.is_empty() {
            session.refresh_board(&data.config, &data);
        }
        for entry in &mut session.board {
            entry.ripeness = 4;
        }
        if roll_rivals(&mut session, &data).is_some() {
            taken += 1;
        }
    }

    assert!(taken > 0, "two hundred ripe weeks and nobody else moved");
}

#[test]
fn at_most_one_mark_goes_in_a_week() {
    let (data, mut session) = setup(4);
    for entry in &mut session.board {
        entry.ripeness = 100;
    }
    let before = session.board.len();

    let lost = roll_rivals(&mut session, &data);

    assert!(
        session.board.len() >= before - 1,
        "the board was cleared out"
    );
    if lost.is_some() {
        assert_eq!(session.board.len(), before - 1);
    }
}

#[test]
fn losing_a_scouted_mark_reports_the_file_work_wasted() {
    let (data, mut session) = setup(5);
    // One mark, ripe and certain to go, with a file already paid for.
    session.board.truncate(1);
    session.board[0].ripeness = 100;
    session.board[0].casing = 2;

    let mut lost = None;
    for _ in 0..200 {
        if session.board.is_empty() {
            break;
        }
        lost = roll_rivals(&mut session, &data);
        if lost.is_some() {
            break;
        }
    }

    let lost = lost.expect("a certainty never happened");
    assert_eq!(lost.casing_wasted, 2);
    assert!(lost.headline().contains("file work wasted"));
    assert_eq!(session.tally.marks_lost_to_rivals, 1);
}

#[test]
fn the_same_seed_loses_the_same_marks() {
    let data = GameData::load().unwrap();
    let mut a = GameSession::new(&data.config, &data, 606);
    let mut b = GameSession::new(&data.config, &data, 606);
    for session in [&mut a, &mut b] {
        for entry in &mut session.board {
            entry.ripeness = 3;
        }
    }

    for _ in 0..20 {
        assert_eq!(roll_rivals(&mut a, &data), roll_rivals(&mut b, &data));
    }
}
