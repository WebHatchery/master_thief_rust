use super::*;
use crate::model::Skill;

fn data() -> GameData {
    GameData::load().unwrap()
}

#[test]
fn the_board_only_offers_marks_the_crews_name_can_open() {
    let data = data();
    let session = GameSession::new(&data.config, &data, 7);

    for entry in &session.board {
        let target = data.targets.get(&entry.target_id).unwrap();
        assert!(target.required_reputation <= session.reputation);
    }
}

#[test]
fn reputation_opens_new_marks() {
    let data = data();
    let mut session = GameSession::new(&data.config, &data, 7);
    let early = session.eligible_targets(&data).len();

    session.reputation = 100;
    assert!(session.eligible_targets(&data).len() > early);
}

#[test]
fn the_same_seed_lays_out_the_same_board() {
    let data = data();
    let a = GameSession::new(&data.config, &data, 20260726);
    let b = GameSession::new(&data.config, &data, 20260726);

    let ids_a: Vec<&str> = a.board.iter().map(|e| e.target_id.as_str()).collect();
    let ids_b: Vec<&str> = b.board.iter().map(|e| e.target_id.as_str()).collect();
    assert_eq!(ids_a, ids_b);
}

#[test]
fn a_mark_left_sitting_is_worth_more_and_costs_more() {
    // The board used to be a stock list: waiting changed nothing, so there
    // was no reason not to take the best mark the moment it appeared.
    let data = data();
    let mut session = GameSession::new(&data.config, &data, 31);
    let board = &data.config.board;
    let entry = session.board[0].clone();
    let base = data.targets.get(&entry.target_id).unwrap().potential_payout;

    assert_eq!(entry.payout_bonus_pct(board), 0);
    assert_eq!(entry.door_penalty(board), 0);
    assert_eq!(entry.ripened_payout(base, board), base);

    session.age_board();
    session.age_board();
    let ripened = &session.board[0];

    assert_eq!(ripened.ripeness, 2);
    assert_eq!(
        ripened.payout_bonus_pct(board),
        2 * board.ripeness_payout_pct
    );
    assert_eq!(ripened.door_penalty(board), 2 * board.ripeness_door_penalty);
    assert!(ripened.ripened_payout(base, board) > base);
}

#[test]
fn ripening_stops_before_a_mark_becomes_the_whole_campaign() {
    let data = data();
    let mut session = GameSession::new(&data.config, &data, 32);
    let board = &data.config.board;

    // Ripeness keeps counting, but neither number does past the cap.
    for _ in 0..3 {
        session.age_board();
    }
    let capped = session.board[0].clone();
    let mut past = capped.clone();
    past.ripeness = 50;

    assert_eq!(
        past.payout_bonus_pct(board),
        board.ripeness_max as i64 * board.ripeness_payout_pct
    );
    assert_eq!(
        past.door_penalty(board),
        board.ripeness_max as i32 * board.ripeness_door_penalty
    );
}

#[test]
fn the_board_ages_out_marks_whose_window_closed() {
    let data = data();
    let mut session = GameSession::new(&data.config, &data, 11);
    let starting = session.board.len();
    assert!(starting > 0);

    for _ in 0..4 {
        session.age_board();
    }
    assert!(session.board.is_empty());
}

#[test]
fn a_mark_quotes_the_city_watch_on_its_own_doors_before_anybody_commits() {
    // Pillar 2 again, one screen earlier than the planning breakdown: the
    // whole point of the city hardening against a method is that the player
    // chooses a *different mark*, and they can only do that on the board.
    let data = data();
    let mut session = GameSession::new(&data.config, &data, 44);
    let target = data
        .targets
        .get(&session.board[0].target_id)
        .unwrap()
        .clone();
    assert!(session.watched_doors(&target, &data).is_empty());

    let doors = data.encounters_for(&target);
    let trade = doors[0].primary_skill;
    session
        .scrutiny
        .note(trade, data.config.scrutiny.ceiling(), &data.config.scrutiny);

    let watched = session.watched_doors(&target, &data);
    assert!(!watched.is_empty(), "a watched trade quoted nothing");
    for (index, penalty) in watched {
        assert_eq!(doors[index].primary_skill, trade);
        assert_eq!(penalty, data.config.scrutiny.max_penalty);
    }
}

#[test]
fn a_trade_nobody_has_worked_is_quoted_at_nothing() {
    let data = data();
    let session = GameSession::new(&data.config, &data, 45);
    for skill in Skill::ALL {
        assert_eq!(session.scrutiny.penalty(skill, &data.config.scrutiny), 0);
    }
}
