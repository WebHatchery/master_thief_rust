//! The tutorial, such as it is: one line of advice per screen, shown until the
//! player turns it off.
//!
//! The original shipped a stepped tutorial with its own state machine. This is
//! the same information without the machinery — a heist planner's difficulty is
//! reading the numbers, not finding the buttons, so what a new fixer needs is a
//! sentence in the right place rather than a tour.

use super::{Screen, UiAction, UiContext, LOGICAL_WIDTH};
use crate::data::GameData;
use crate::state::GameSession;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

/// The advice for a screen, chosen by what the campaign has and has not done
/// yet — a hint that keeps telling you to do something you have already done is
/// worse than no hint, and one describing a mechanic the game no longer has is
/// worse than both.
///
/// Takes the three things it actually reads rather than the whole `UiContext`,
/// so the choosing can be tested without a window.
pub fn hint_for(session: &GameSession, data: &GameData, screen: Screen) -> Option<&'static str> {
    // The bill outranks every other piece of advice: a fixer who cannot make
    // payroll has one problem and it is not their skill assignments.
    if crate::sim::weeks_of_runway(session, &data.config.payroll) <= 1 {
        return Some(
            "The outfit cannot cover next week's payroll. Take a job, or let somebody go before they walk.",
        );
    }
    if session.crew.iter().any(|m| m.condition.notice_given) {
        return Some(
            "Somebody has given notice. The Outfit tab shows what a bonus costs — after next week it is too late.",
        );
    }
    if !session.custody.is_empty() {
        return Some("The city is holding one of yours. Bail is on the Outfit tab, and it is not getting cheaper.");
    }

    let condition = &data.config.condition;
    let hours = session.attention_left_this_week(&data.config);

    Some(match screen {
        Screen::Crew if crate::sim::attention_chance(session, &data.config.law) >= 0.25 => {
            "Heat this high risks a raid or an arrest every week. Grease palms on the Outfit tab, or lie low and pay for it."
        }
        // A hand about to be misused outranks a roster that is merely thin:
        // one is a mistake the fixer is making this second, the other is
        // standing advice that will still be true next week.
        Screen::Crew
            if session
                .crew
                .iter()
                .any(|m| condition.is_spent(m.condition.fatigue)) =>
        {
            "Somebody is spent. You can still send them — it costs on the die, gets them hurt easier, and they remember it."
        }
        Screen::Crew if session.crew.len() < 4 => {
            "Every door wants a specialist. Check the For Hire tab — a crew of three cannot cover six skills."
        }
        Screen::Crew
            if hours == 0 && session.crew.iter().any(|m| m.progression.skill_points > 0) =>
        {
            "Points are waiting but the week is not. Drilling costs an hour, and this week's went on the board."
        }
        Screen::Crew if session.crew.iter().any(|m| m.progression.skill_points > 0) => {
            "Somebody has points to spend. Drilling costs an hour of the same week casing spends, so the two compete."
        }
        Screen::Crew if session.crew.iter().any(|m| !m.condition.injuries.is_empty()) => {
            "An injury heals free but slowly. Treat pays a doctor to buy those weeks back — the week costs wages either way."
        }
        Screen::Crew => {
            "Fatigue costs dice from fifty, and past eighty a hand is spent — still sendable, and worse at everything."
        }
        Screen::Board if !session.scrutiny.is_empty() => {
            "The city has a trade of yours on file. Watched doors are flagged, with how many quiet weeks take a point off."
        }
        Screen::Board if session.board.iter().all(|entry| entry.is_blind()) => {
            "Casing buys one door at a time against the week's hours — the same hours training spends. Spread them or go deep."
        }
        Screen::Board => {
            "Plan the job to choose who takes which door. Delegate if you are in a hurry — it uses the same engine, worse."
        }
        Screen::Shop => {
            "Kit is issued to whoever is selected on the Crew screen. A tool on the wrong hand is worth nothing."
        }
        Screen::Planning => {
            "Every candidate shows the roll they need, and the standing order beside Commit says when the crew walk out."
        }
        Screen::Run => "Hold Space to hurry it along. The result was decided the moment you committed.",
        Screen::Results => {
            "Reputation opens better marks; notoriety brings heat. What you keep doing gets watched, trade by trade."
        }
        Screen::Records => {
            "Lying low cools the city and still costs a week's wages. Both curves are here; so is the bill."
        }
    })
}

/// Draw the hint bar, if there is a hint and the player still wants them.
pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    if !ctx.prefs.hints {
        return;
    }
    let Some(hint) = hint_for(ctx.session, ctx.data, ctx.screen) else {
        return;
    };

    let rect = Rect::new(18.0, 132.0, LOGICAL_WIDTH - 36.0, 26.0);
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.10, 0.12, 0.17, 0.95)).with_left_accent(3.0, dark::ACCENT),
    );
    draw_ui_text_ex(
        hint,
        rect.x + 14.0,
        rect.y + 18.0,
        TextStyle::new(13.0, Color::new(0.68, 0.76, 0.88, 1.0)).params(),
    );

    if button_rect_tone_at(
        Rect::new(rect.right() - 86.0, rect.y + 3.0, 78.0, 20.0),
        "Got it",
        true,
        ButtonTone::Secondary,
        ctx.mouse(),
    ) {
        actions.push(UiAction::ShowHints(false));
    }
}

#[cfg(test)]
mod tests {
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
}
