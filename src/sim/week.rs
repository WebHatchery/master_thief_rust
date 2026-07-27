//! Advancing the week: rest, healing, the payroll, the city's attention, and a
//! new board.
//!
//! Lying low — advancing with no job run — is a legitimate move, and this is
//! where it pays (GDD 5.6). It is no longer free: the safehouse and the crew
//! both send a bill every week, so resting is a purchase and the fixer has to
//! decide they can afford it.
//!
//! Order matters and is fixed. Everything that draws from the run's RNG does so
//! at the same point every week — the law roll, then the board, then the hiring
//! pool — so a seed replays a campaign exactly (GDD 5.7).

use super::law::LawEvent;
use super::payroll::{settle_payroll, PayrollOutcome};
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
    /// The bill, and whether the outfit covered it.
    pub payroll: PayrollOutcome,
    /// The city's move, if it made one.
    pub law: Option<LawEvent>,
    /// A mark another outfit got to first, if one went.
    pub rival: Option<super::rivals::RivalJob>,
    /// Weeks of a tail still to run after this one.
    pub surveillance_weeks: u32,
}

impl WeekSummary {
    /// The one line the week is always worth reporting.
    pub fn ledger_line(&self) -> String {
        use macroquad_toolkit::ui::format_money;
        if self.payroll.was_short() {
            format!(
                "Week {} — payroll short by {}",
                self.week,
                format_money(self.payroll.shortfall)
            )
        } else {
            format!(
                "Week {} — {} paid out, {} fatigue shed, heat down {}",
                self.week,
                format_money(self.payroll.paid),
                self.fatigue_shed,
                self.heat_shed
            )
        }
    }

    /// What went wrong this week, worst first. Empty on a clean one.
    pub fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if let Some(event) = &self.law {
            warnings.push(event.headline.clone());
        }
        if let Some(job) = &self.rival {
            warnings.push(job.headline());
        }
        for name in &self.payroll.walkouts {
            warnings.push(format!("{} took their kit and left", name));
        }
        for name in &self.payroll.notices {
            warnings.push(format!("{} is talking about walking", name));
        }
        warnings
    }

    /// The week's ordinary news.
    pub fn notes(&self) -> Vec<String> {
        let mut notes = Vec::new();
        if self.injuries_healed > 0 {
            notes.push(format!("{} back on their feet", self.injuries_healed));
        }
        if self.new_marks > 0 {
            notes.push(format!("{} new marks on the board", self.new_marks));
        }
        notes
    }
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

    // The bill comes before anything else the fixer might want to spend on.
    let payroll = settle_payroll(session, config);

    let heat_before = session.heat;
    session.heat = (session.heat - config.heat_decay_per_week).max(0);
    session.surveillance_weeks = session.surveillance_weeks.saturating_sub(1);

    // The first and only draw of the week from the RNG before the board.
    let law = super::law::roll_attention(session, config);
    let heat_shed = heat_before - session.heat;

    // A new week is a fresh set of eyes: the attention spent scouting resets.
    session.casing_this_week = 0;

    // The competition moves before the board ages, so a mark somebody else took
    // never gets to ripen one more week on the way out (GDD 5.4).
    let rival = super::rivals::roll_rivals(session, data);

    session.age_board();
    let marks_before = session.board.len();
    session.refresh_board(config, data);
    let new_marks = session.board.len().saturating_sub(marks_before);

    // Word gets around; a different set of people come asking each week.
    session.refresh_recruits(config, data);
    let crew = session.crew_ids();
    session.chemistry.retain_crew(&crew);

    // A pair who did not stand in the same building this week drift back toward
    // indifference. Warmth has to be kept up, and a grudge can be waited out
    // (GDD 5.5).
    let worked_together = crew_on_last_job(session);
    session
        .chemistry
        .cool_off(&worked_together, config.chemistry_cooling);

    // A week that ran nothing is a week spent lying low, which is a real
    // strategy and worth counting (GDD 5.6).
    let ran_a_job = session
        .history
        .last()
        .is_some_and(|record| record.week == session.week);
    if !ran_a_job {
        session.tally.quiet_weeks += 1;
    }
    for member in &mut session.crew {
        member.condition.worked_this_week = false;
    }
    session.week += 1;

    WeekSummary {
        week: session.week,
        fatigue_shed,
        injuries_healed,
        heat_shed,
        new_marks,
        payroll,
        law,
        rival,
        surveillance_weeks: session.surveillance_weeks,
    }
}

/// Whoever stood in a building together this week. Read off the crew's own
/// job records rather than tracked separately, so it cannot drift from what
/// actually happened.
fn crew_on_last_job(session: &GameSession) -> Vec<String> {
    let ran_this_week = session
        .history
        .last()
        .is_some_and(|record| record.week == session.week);
    if !ran_this_week {
        return Vec::new();
    }
    session
        .crew
        .iter()
        .filter(|member| member.condition.worked_this_week)
        .map(|member| member.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::crew::Injury;
    use crate::sim::law::LawEventKind;
    use crate::sim::payroll::weekly_outgoings;

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

    #[test]
    fn a_quiet_week_costs_the_outfit_a_weeks_pay() {
        // The whole point of the change: "rest until everyone is fresh" is now
        // a purchase, and the fixer can read the price.
        let (data, mut session) = setup(7);
        let due = weekly_outgoings(&session, &data.config.payroll);
        let budget = session.budget;

        let summary = advance_week(&mut session, &data);

        assert!(due > 0);
        assert_eq!(summary.payroll.total_due(), due);
        assert_eq!(session.budget, budget - due);
        assert!(summary.ledger_line().contains("paid out"));
    }

    #[test]
    fn an_outfit_that_never_works_runs_itself_into_the_ground() {
        // Twenty weeks of lying low used to leave a rested crew and a full
        // wallet. It should now leave neither.
        let (data, mut session) = setup(8);

        for _ in 0..20 {
            advance_week(&mut session, &data);
        }

        assert!(session.budget <= 0, "idling never cost anything");
        assert!(
            session.crew.len() < 3 || session.crew.iter().any(|m| m.condition.notice_given),
            "twenty unpaid weeks and the crew stayed cheerful"
        );
    }

    #[test]
    fn a_missed_payroll_is_reported_rather_than_swallowed() {
        let (data, mut session) = setup(9);
        session.budget = 0;

        let summary = advance_week(&mut session, &data);

        assert!(summary.payroll.was_short());
        assert!(summary.ledger_line().contains("short by"));
    }

    #[test]
    fn a_hot_week_can_end_with_somebody_in_a_cell() {
        let (data, mut session) = setup(10);
        session.budget = 5_000_000;

        let mut arrested = false;
        for _ in 0..60 {
            session.heat = data.config.law.custody_threshold + 30;
            let summary = advance_week(&mut session, &data);
            if summary
                .law
                .as_ref()
                .is_some_and(|event| event.kind == LawEventKind::Arrest)
            {
                arrested = true;
                break;
            }
        }

        assert!(arrested, "sixty of the hottest weeks and nobody was taken");
        assert_eq!(session.custody.len(), 1);
        assert!(session
            .custody
            .first()
            .is_some_and(|record| record.bail > 0));
    }

    #[test]
    fn a_pair_the_fixer_stops_using_goes_off_the_boil() {
        let (data, mut session) = setup(12);
        let ids = session.crew_ids();
        session.chemistry.set(&ids[0], &ids[1], 60);

        // A quiet week is a week nobody stood in a building together.
        advance_week(&mut session, &data);

        assert_eq!(
            session.chemistry.get(&ids[0], &ids[1]),
            60 - data.config.chemistry_cooling,
            "warmth survived a week of nobody working"
        );
    }

    #[test]
    fn a_pair_who_worked_together_keep_what_they_built() {
        let (data, mut session) = setup(13);
        let ids = session.crew_ids();
        session.chemistry.set(&ids[0], &ids[1], 60);
        session.chemistry.set(&ids[0], &ids[2], 60);

        // Mark two of them as having worked, and file a job for this week so
        // the week knows the outfit was out.
        session.crew[0].condition.worked_this_week = true;
        session.crew[1].condition.worked_this_week = true;
        session.history.push(crate::state::JobRecord {
            week: session.week,
            target_name: "Somewhere".to_owned(),
            difficulty: crate::model::DifficultyBand::Easy,
            success: true,
            doors_passed: 1,
            doors_total: 1,
            payout: 0,
            delegated: false,
            reputation: 0,
            notoriety: 0,
        });

        advance_week(&mut session, &data);

        assert_eq!(session.chemistry.get(&ids[0], &ids[1]), 60);
        assert_eq!(
            session.chemistry.get(&ids[0], &ids[2]),
            60 - data.config.chemistry_cooling
        );
        assert!(
            session.crew.iter().all(|m| !m.condition.worked_this_week),
            "the week did not reset who worked"
        );
    }

    #[test]
    fn a_mark_left_to_ripen_can_be_taken_by_somebody_else() {
        // The whole reason rivals exist: ripening made waiting profitable and
        // perfectly calculable. This is the part that cannot be calculated.
        let (data, mut session) = setup(14);
        session.budget = 50_000_000;
        let mut lost = 0;

        for _ in 0..60 {
            for entry in &mut session.board {
                entry.ripeness = data.config.board.ripeness_max;
                entry.weeks_remaining = 9;
            }
            if advance_week(&mut session, &data).rival.is_some() {
                lost += 1;
            }
        }

        assert!(lost > 0, "sixty ripe weeks and nobody else took anything");
        assert_eq!(session.tally.marks_lost_to_rivals, lost as i64);
    }

    #[test]
    fn a_board_nobody_is_sitting_on_is_never_poached() {
        // Rivals answer hesitation, not existence. A crew who take their work
        // promptly should never meet one.
        let (data, mut session) = setup(15);
        session.budget = 50_000_000;

        for _ in 0..40 {
            for entry in &mut session.board {
                entry.ripeness = 0;
            }
            assert!(advance_week(&mut session, &data).rival.is_none());
        }
        assert_eq!(session.tally.marks_lost_to_rivals, 0);
    }

    #[test]
    fn a_tail_runs_out_on_its_own() {
        let (data, mut session) = setup(11);
        session.surveillance_weeks = 2;

        let summary = advance_week(&mut session, &data);
        assert_eq!(summary.surveillance_weeks, 1);
        advance_week(&mut session, &data);
        assert_eq!(session.surveillance_weeks, 0);
    }

    #[test]
    fn the_same_seed_advances_the_same_week() {
        let data = GameData::load().unwrap();
        let mut a = GameSession::new(&data.config, &data, 4242);
        let mut b = GameSession::new(&data.config, &data, 4242);
        a.heat = 90;
        b.heat = 90;

        for _ in 0..8 {
            let left = advance_week(&mut a, &data);
            let right = advance_week(&mut b, &data);
            assert_eq!(left, right);
        }
        assert_eq!(a.budget, b.budget);
        assert_eq!(a.custody.len(), b.custody.len());
    }
}
