//! Advancing the week: rest, healing, heat decay, and a new board.
//!
//! Lying low — advancing with no job run — is a legitimate move, and this is
//! where it pays (GDD 5.6).

use crate::data::GameData;
use crate::rules::attribute_modifier;
use crate::state::GameSession;

/// What the week changed, for the notification the player actually reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeekSummary {
    pub week: u32,
    pub fatigue_shed: i32,
    pub injuries_healed: usize,
    pub heat_shed: i32,
    pub new_marks: usize,
}

pub fn advance_week(session: &mut GameSession, data: &GameData) -> WeekSummary {
    let config = &data.config;
    let mut fatigue_shed = 0;
    let mut injuries_healed = 0;

    for member in &mut session.crew {
        let constitution = attribute_modifier(member.attributes.constitution);
        let recovery = (config.rest_recovery + constitution * 5).max(5);
        let before = member.condition.fatigue;
        member.condition.add_fatigue(-recovery);
        fatigue_shed += before - member.condition.fatigue;

        for injury in &mut member.condition.injuries {
            injury.weeks_remaining = injury.weeks_remaining.saturating_sub(1);
        }
        let before_count = member.condition.injuries.len();
        member
            .condition
            .injuries
            .retain(|injury| injury.weeks_remaining > 0);
        injuries_healed += before_count - member.condition.injuries.len();

        // Idle hands drift; a rested, uninjured crew steadies.
        if member.condition.injuries.is_empty() {
            member.condition.adjust_loyalty(config.idle_loyalty_drift);
        } else {
            member.condition.adjust_loyalty(-config.idle_loyalty_drift);
        }
    }

    let heat_before = session.heat;
    session.heat = (session.heat - config.heat_decay_per_week).max(0);
    let heat_shed = heat_before - session.heat;

    session.age_board();
    let marks_before = session.board.len();
    session.refresh_board(config, data);
    let new_marks = session.board.len().saturating_sub(marks_before);

    // Word gets around; a different set of people come asking each week.
    session.refresh_recruits(config, data);
    let crew = session.crew_ids();
    session.chemistry.retain_crew(&crew);

    session.week += 1;

    WeekSummary {
        week: session.week,
        fatigue_shed,
        injuries_healed,
        heat_shed,
        new_marks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::crew::Injury;

    fn setup(seed: u64) -> (GameData, GameSession) {
        let data = GameData::load().unwrap();
        let session = GameSession::new(&data.config, &data, seed);
        (data, session)
    }

    #[test]
    fn a_week_of_rest_sheds_fatigue() {
        let (data, mut session) = setup(1);
        session.crew[0].condition.fatigue = 70;

        let summary = advance_week(&mut session, &data);
        assert!(session.crew[0].condition.fatigue < 70);
        assert!(summary.fatigue_shed > 0);
        assert_eq!(summary.week, 2);
    }

    #[test]
    fn injuries_clear_on_their_own_schedule_not_instantly() {
        let (data, mut session) = setup(2);
        session.crew[0]
            .condition
            .injuries
            .push(Injury::major("Torn shoulder"));

        advance_week(&mut session, &data);
        assert_eq!(session.crew[0].condition.injuries.len(), 1);

        advance_week(&mut session, &data);
        advance_week(&mut session, &data);
        assert!(session.crew[0].condition.injuries.is_empty());
    }

    #[test]
    fn lying_low_cools_the_city() {
        let (data, mut session) = setup(3);
        session.heat = 40;

        let summary = advance_week(&mut session, &data);
        assert_eq!(session.heat, 40 - data.config.heat_decay_per_week);
        assert_eq!(summary.heat_shed, data.config.heat_decay_per_week);
    }

    #[test]
    fn heat_never_goes_below_nothing() {
        let (data, mut session) = setup(4);
        session.heat = 2;

        advance_week(&mut session, &data);
        assert_eq!(session.heat, 0);
    }

    #[test]
    fn the_board_keeps_itself_stocked_as_marks_expire() {
        let (data, mut session) = setup(5);
        for _ in 0..6 {
            advance_week(&mut session, &data);
        }
        assert!(!session.board.is_empty());
    }

    #[test]
    fn a_hurt_crew_loses_faith_while_a_rested_one_settles() {
        let (data, mut session) = setup(6);
        session.crew[0].condition.loyalty = 50;
        session.crew[1]
            .condition
            .injuries
            .push(Injury::major("Broken hand"));
        session.crew[1].condition.loyalty = 50;

        advance_week(&mut session, &data);
        assert!(session.crew[0].condition.loyalty > 50);
        assert!(session.crew[1].condition.loyalty < 50);
    }
}
