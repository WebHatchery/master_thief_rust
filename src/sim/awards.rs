//! What the campaign has to show for itself.
//!
//! Achievements are data, not code: `achievements.json` names a statistic and a
//! threshold, and everything else is one comparison. Adding one is an edit to a
//! JSON file, which is the same rule the rest of the game's content follows.

use crate::data::GameConfig;
use crate::state::GameSession;
use macroquad_toolkit::achievements::Achievement;
use serde::{Deserialize, Serialize};

/// Everything a campaign counts about itself. Cumulative and saved, because
/// several of these can only ever be answered by remembering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignTally {
    pub jobs_run: i64,
    pub jobs_won: i64,
    pub jobs_lost: i64,
    /// Jobs where every door was cleared.
    pub clean_sweeps: i64,
    pub doors_cleared: i64,
    pub critical_successes: i64,
    pub critical_failures: i64,
    pub complications_faced: i64,
    pub payout_total: i64,
    pub payout_best: i64,
    pub loot_found: i64,
    pub injuries_taken: i64,
    pub hires: i64,
    /// Jobs committed from a hand-made plan.
    pub planned_jobs: i64,
    pub delegated_jobs: i64,
    /// Jobs run on a mark nobody had cased.
    pub blind_jobs: i64,
    /// Jobs the crew walked out of on the fixer's standing order (GDD 5.2).
    #[serde(default)]
    pub jobs_called_off: i64,
    /// Doors left standing because the order came before them.
    #[serde(default)]
    pub doors_left_standing: i64,
    /// Doors worked by a hand who should have been resting (GDD 5.6).
    #[serde(default)]
    pub doors_worked_spent: i64,
    /// Weeks advanced without running anything.
    pub quiet_weeks: i64,
    pub heat_peak: i64,
    /// Wages and safehouse upkeep the outfit actually covered.
    #[serde(default)]
    pub wages_paid: i64,
    /// Weeks the outfit could not cover its own bill.
    #[serde(default)]
    pub weeks_missed_payroll: i64,
    /// Goodwill payments made to keep somebody from walking.
    #[serde(default)]
    pub bonuses_paid: i64,
    /// Hands who left over money.
    #[serde(default)]
    pub walkouts: i64,
    /// Weeks the city took an interest, of any kind.
    #[serde(default)]
    pub law_incidents: i64,
    /// Hands the city took into custody.
    #[serde(default)]
    pub arrests: i64,
    #[serde(default)]
    pub cash_seized: i64,
    #[serde(default)]
    pub bribes_paid: i64,
    #[serde(default)]
    pub heat_bought_down: i64,
    #[serde(default)]
    pub bails_paid: i64,
    /// Marks another outfit took while the crew were still thinking about it.
    #[serde(default)]
    pub marks_lost_to_rivals: i64,
    /// Spent on doctors rather than waiting injuries out.
    #[serde(default)]
    pub treatment_paid: i64,
    #[serde(default)]
    pub injuries_treated: i64,
    /// Spent keeping the outfit's tools in working order.
    #[serde(default)]
    pub refits_paid: i64,
    /// Taken in selling kit back out rather than working for it.
    #[serde(default)]
    pub fenced_total: i64,
    #[serde(default)]
    pub items_fenced: i64,
    /// Marks the crew brought in themselves.
    #[serde(default)]
    pub leads_brought_in: i64,
    /// Paid to hands the outfit let go.
    #[serde(default)]
    pub severance_paid: i64,
    #[serde(default)]
    pub dismissals: i64,
    /// What the outfit walked away with, once it walked away.
    #[serde(default)]
    pub final_take: i64,
}

/// The statistics an achievement can be written against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TallyStat {
    JobsRun,
    JobsWon,
    JobsLost,
    CleanSweeps,
    DoorsCleared,
    CriticalSuccesses,
    CriticalFailures,
    ComplicationsFaced,
    PayoutTotal,
    PayoutBest,
    LootFound,
    InjuriesTaken,
    Hires,
    PlannedJobs,
    DelegatedJobs,
    BlindJobs,
    /// Jobs the crew were told to walk out of, and what that left behind.
    JobsCalledOff,
    DoorsLeftStanding,
    /// Doors opened by somebody past the working threshold.
    DoorsWorkedSpent,
    QuietWeeks,
    HeatPeak,
    WagesPaid,
    WeeksMissedPayroll,
    BonusesPaid,
    Walkouts,
    LawIncidents,
    Arrests,
    CashSeized,
    BribesPaid,
    HeatBoughtDown,
    BailsPaid,
    MarksLostToRivals,
    TreatmentPaid,
    InjuriesTreated,
    RefitsPaid,
    FencedTotal,
    ItemsFenced,
    LeadsBroughtIn,
    SeverancePaid,
    Dismissals,
    FinalTake,
    /// Live campaign state rather than a running total.
    Week,
    Reputation,
    Notoriety,
    Budget,
    CrewSize,
    BestLevel,
    BestMastery,
    /// The warmest pair on the payroll.
    BestChemistry,
    /// How deep the worst grudge runs, as a positive number.
    WorstGrudge,
    /// Equipment ids in the lockup.
    InventorySize,
    /// The worst the city is currently charging for one of the outfit's own
    /// habits (GDD 5.4). Live state, not a running total — a file goes cold.
    WorstScrutiny,
}

impl TallyStat {
    pub fn value(self, tally: &CampaignTally, session: &GameSession, config: &GameConfig) -> i64 {
        match self {
            TallyStat::JobsRun => tally.jobs_run,
            TallyStat::JobsWon => tally.jobs_won,
            TallyStat::JobsLost => tally.jobs_lost,
            TallyStat::CleanSweeps => tally.clean_sweeps,
            TallyStat::DoorsCleared => tally.doors_cleared,
            TallyStat::CriticalSuccesses => tally.critical_successes,
            TallyStat::CriticalFailures => tally.critical_failures,
            TallyStat::ComplicationsFaced => tally.complications_faced,
            TallyStat::PayoutTotal => tally.payout_total,
            TallyStat::PayoutBest => tally.payout_best,
            TallyStat::LootFound => tally.loot_found,
            TallyStat::InjuriesTaken => tally.injuries_taken,
            TallyStat::Hires => tally.hires,
            TallyStat::PlannedJobs => tally.planned_jobs,
            TallyStat::DelegatedJobs => tally.delegated_jobs,
            TallyStat::BlindJobs => tally.blind_jobs,
            TallyStat::JobsCalledOff => tally.jobs_called_off,
            TallyStat::DoorsLeftStanding => tally.doors_left_standing,
            TallyStat::DoorsWorkedSpent => tally.doors_worked_spent,
            TallyStat::QuietWeeks => tally.quiet_weeks,
            TallyStat::HeatPeak => tally.heat_peak,
            TallyStat::WagesPaid => tally.wages_paid,
            TallyStat::WeeksMissedPayroll => tally.weeks_missed_payroll,
            TallyStat::BonusesPaid => tally.bonuses_paid,
            TallyStat::Walkouts => tally.walkouts,
            TallyStat::LawIncidents => tally.law_incidents,
            TallyStat::Arrests => tally.arrests,
            TallyStat::CashSeized => tally.cash_seized,
            TallyStat::BribesPaid => tally.bribes_paid,
            TallyStat::HeatBoughtDown => tally.heat_bought_down,
            TallyStat::BailsPaid => tally.bails_paid,
            TallyStat::MarksLostToRivals => tally.marks_lost_to_rivals,
            TallyStat::TreatmentPaid => tally.treatment_paid,
            TallyStat::InjuriesTreated => tally.injuries_treated,
            TallyStat::RefitsPaid => tally.refits_paid,
            TallyStat::FencedTotal => tally.fenced_total,
            TallyStat::ItemsFenced => tally.items_fenced,
            TallyStat::LeadsBroughtIn => tally.leads_brought_in,
            TallyStat::SeverancePaid => tally.severance_paid,
            TallyStat::Dismissals => tally.dismissals,
            TallyStat::FinalTake => tally.final_take,

            TallyStat::Week => session.week as i64,
            TallyStat::Reputation => session.reputation as i64,
            TallyStat::Notoriety => session.notoriety as i64,
            TallyStat::Budget => session.budget,
            TallyStat::CrewSize => session.crew.len() as i64,
            TallyStat::BestLevel => session
                .crew
                .iter()
                .map(|member| member.progression.level as i64)
                .max()
                .unwrap_or(0),
            TallyStat::BestMastery => session
                .crew
                .iter()
                .map(|member| member.progression.mastery_level as i64)
                .max()
                .unwrap_or(0),
            TallyStat::BestChemistry => session
                .chemistry
                .known_pairs()
                .map(|(_, _, value)| value as i64)
                .max()
                .unwrap_or(0),
            TallyStat::WorstGrudge => session
                .chemistry
                .known_pairs()
                .map(|(_, _, value)| -(value as i64))
                .max()
                .unwrap_or(0),
            TallyStat::InventorySize => session.inventory.len() as i64,
            TallyStat::WorstScrutiny => session
                .scrutiny
                .watched(&config.scrutiny)
                .first()
                .map(|(_, penalty)| *penalty as i64)
                .unwrap_or(0),
        }
    }
}

/// One achievement, as authored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AwardDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub stat: TallyStat,
    pub at_least: i64,
}

impl AwardDef {
    pub fn is_earned(
        &self,
        tally: &CampaignTally,
        session: &GameSession,
        config: &GameConfig,
    ) -> bool {
        self.stat.value(tally, session, config) >= self.at_least
    }

    pub fn definition(&self) -> Achievement {
        Achievement::new(&self.id, &self.name, &self.description)
    }

    /// How far along the campaign is, 0..1, for a progress bar.
    pub fn progress(
        &self,
        tally: &CampaignTally,
        session: &GameSession,
        config: &GameConfig,
    ) -> f32 {
        if self.at_least <= 0 {
            return 1.0;
        }
        (self.stat.value(tally, session, config) as f32 / self.at_least as f32).clamp(0.0, 1.0)
    }
}

/// Unlock everything the campaign has earned. Returns the names of anything
/// newly unlocked, so the game can say so.
pub fn award(session: &mut GameSession, config: &GameConfig, defs: &[AwardDef]) -> Vec<String> {
    let tally = session.tally;
    let mut earned = Vec::new();

    for def in defs {
        if def.is_earned(&tally, session, config) && session.achievements.unlock(&def.id) {
            earned.push(def.name.clone());
        }
    }
    earned
}

#[cfg(test)]
mod tests;
