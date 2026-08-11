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
    /// Work somebody on the payroll brought in, if anybody did.
    pub lead: Option<super::leads::Lead>,
    /// Weeks of a tail still to run after this one.
    pub surveillance_weeks: u32,
    /// Trades the city stopped charging the outfit for this week. Only the ones
    /// whose penalty actually moved: attention that thinned without changing a
    /// die is not news (GDD 5.4).
    pub trades_cooled: Vec<crate::model::Skill>,
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
        if let Some(lead) = &self.lead {
            notes.push(lead.headline());
        }
        for skill in &self.trades_cooled {
            notes.push(format!(
                "The city has stopped watching {} so closely",
                skill.label().to_lowercase()
            ));
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

    // Heat is bought down; a reputation for a method is only waited out. This
    // is the other thing a quiet week buys, and the one the fixer cannot bribe
    // their way past (GDD 5.4).
    let trades_cooled = session.scrutiny.cool(&config.scrutiny);

    // The first and only draw of the week from the RNG before the board.
    let law = super::law::roll_attention(session, config);
    let heat_shed = heat_before - session.heat;

    // A new week is a fresh set of eyes: the attention spent scouting resets.
    session.attention_spent_this_week = 0;

    // The competition moves before the board ages, so a mark somebody else took
    // never gets to ripen one more week on the way out (GDD 5.4).
    let rival = super::rivals::roll_rivals(session, data);

    session.age_board();
    let marks_before = session.board.len();
    session.refresh_board(config, data);
    let new_marks = session.board.len().saturating_sub(marks_before);

    // A crew who are happy hear things. Rolled after the board is stocked, so
    // a tip-off is work on top of the week's own rather than instead of it
    // (GDD 5.5).
    let lead = super::leads::roll_lead(session, data);

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
        lead,
        surveillance_weeks: session.surveillance_weeks,
        trades_cooled,
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
mod tests;
