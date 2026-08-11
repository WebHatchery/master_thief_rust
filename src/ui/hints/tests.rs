use super::*;
use crate::model::crew::Injury;

const SCREENS: [Screen; 7] = [
    Screen::Crew,
    Screen::Board,
    Screen::Shop,
    Screen::Planning,
    Screen::Run,
    Screen::Results,
    Screen::Records,
];

fn setup() -> (GameData, GameSession) {
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config, &data, 808);
    (data, session)
}

/// Past the "a crew of three cannot cover six skills" advice, so the arms
/// underneath it can be reached.
fn pad_the_roster(session: &mut GameSession) {
    while session.crew.len() < 4 {
        let mut extra = session.crew[0].clone();
        extra.id = format!("{}_spare_{}", extra.id, session.crew.len());
        session.crew.push(extra);
    }
    // A bigger roster is a bigger bill, and the payroll warning outranks
    // everything — which is correct, and not what these tests are about.
    session.budget = 5_000_000;
}

#[test]
fn every_screen_has_something_to_say() {
    // The version of this test that shipped for seven iterations looped
    // over the screens and did `let _ = screen;` — it never called
    // `hint_for` at all. It could not have failed for any reason.
    let (data, session) = setup();
    for screen in SCREENS {
        let hint = hint_for(&session, &data, screen).unwrap_or_else(|| {
            panic!("{:?} had nothing to say", screen);
        });
        assert!(!hint.is_empty());
        assert!(
            hint.len() < 140,
            "{:?}'s hint is {} characters and the bar is one line",
            screen,
            hint.len()
        );
    }
}

#[test]
fn the_bill_outranks_every_other_piece_of_advice() {
    let (data, mut session) = setup();
    session.budget = 0;
    for screen in SCREENS {
        let hint = hint_for(&session, &data, screen).unwrap();
        assert!(
            hint.contains("payroll"),
            "{:?} advised something else",
            screen
        );
    }
}

#[test]
fn a_spent_hand_is_worth_saying_before_unspent_points_are() {
    // One is a hand the fixer is about to misuse; the other is a hand they
    // have not got round to.
    let (data, mut session) = setup();
    session.crew[0].progression.skill_points = 3;
    session.crew[0].condition.fatigue = data.config.condition.fatigue_work_threshold + 5;

    let hint = hint_for(&session, &data, Screen::Crew).unwrap();
    assert!(
        hint.contains("spent"),
        "advised training a spent hand: {}",
        hint
    );
}

#[test]
fn the_crew_screen_says_why_the_plus_buttons_are_missing() {
    // The buttons hide when the week has no hours left. A hint telling the
    // player to press them would be pointing at nothing.
    let (data, mut session) = setup();
    pad_the_roster(&mut session);
    session.crew[0].progression.skill_points = 3;
    session.attention_spent_this_week = data.config.attention_per_week;

    let hint = hint_for(&session, &data, Screen::Crew).unwrap();
    assert!(hint.contains("hour"), "{}", hint);
    assert!(hint.contains("not"), "{}", hint);
}

#[test]
fn the_board_leads_with_the_city_once_it_has_a_file_on_you() {
    let (data, mut session) = setup();
    let blind = hint_for(&session, &data, Screen::Board).unwrap();
    assert!(blind.contains("Casing"));

    session.scrutiny.note(
        crate::model::Skill::Stealth,
        data.config.scrutiny.ceiling(),
        &data.config.scrutiny,
    );
    let watched = hint_for(&session, &data, Screen::Board).unwrap();
    assert_ne!(watched, blind);
    assert!(watched.contains("file"), "{}", watched);
}

#[test]
fn no_hint_describes_a_mechanic_the_game_does_not_have() {
    // The specific rot this file grew: hints outlive the systems they were
    // written for. "Looks" became hours shared with training, and skill
    // points stopped being free, and the advice said neither for a while.
    let (data, mut session) = setup();
    session.crew[0]
        .condition
        .injuries
        .push(Injury::minor("Sprain"));

    let all: Vec<&str> = SCREENS
        .into_iter()
        .filter_map(|screen| hint_for(&session, &data, screen))
        .collect();
    let joined = all.join(" ");

    assert!(
        !joined.contains("looks a week"),
        "the hints still ration looks rather than hours"
    );
    assert!(
        joined.contains("hours") || joined.contains("hour"),
        "nothing tells the player the week is rationed at all"
    );
}
