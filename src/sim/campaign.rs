//! Playing a whole campaign without a window.
//!
//! GDD 13 calls M4 done when a twenty-week campaign is playable and the crew
//! visibly changes. This is that claim, written as something that can fail.

use crate::data::GameData;
use crate::state::GameSession;

/// What a headless campaign did, so a test can ask whether it was a campaign
/// rather than twenty quiet weeks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CampaignLog {
    pub weeks: u32,
    pub jobs_run: usize,
    pub jobs_won: usize,
    pub levels_gained: i32,
    pub injuries_taken: usize,
    pub loot_found: usize,
    pub hires: usize,
    /// Weeks the outfit could not cover its own bill.
    pub weeks_short: usize,
    /// Hands who walked over money.
    pub walkouts: usize,
    /// Weeks the city took an interest, of any kind.
    pub law_incidents: usize,
    /// Weeks that ended with the city charging the outfit for one of its own
    /// habits (GDD 5.4).
    pub weeks_under_watch: usize,
    /// The worst a single trade was ever watched.
    pub watch_peak: i32,
    pub arrests: usize,
    pub bails_posted: usize,
    /// Every narrative line the campaign printed, in order. GDD 13 calls M6
    /// done when a full campaign rarely repeats one, so the campaign has to
    /// remember what it said.
    pub narrative_lines: Vec<String>,
}

impl CampaignLog {
    /// Share of printed lines that were the first time the player saw them.
    pub fn narrative_freshness(&self) -> f32 {
        if self.narrative_lines.is_empty() {
            return 1.0;
        }
        let mut unique = self.narrative_lines.clone();
        unique.sort();
        unique.dedup();
        unique.len() as f32 / self.narrative_lines.len() as f32
    }
}

/// Play `weeks` of campaign the way an unattended fixer would: case what the
/// budget allows, delegate the best mark on the board, and advance.
pub fn play(session: &mut GameSession, data: &GameData, weeks: u32) -> CampaignLog {
    let mut log = CampaignLog::default();
    let starting_levels: i32 = session
        .crew
        .iter()
        .map(|member| member.progression.level)
        .sum();

    for _ in 0..weeks {
        log.weeks += 1;
        keep_the_outfit_standing(session, data, &mut log);

        // Take on a hand when the money is comfortable and somebody is asking.
        // Comfortable now means a month of everybody's wages, not a fixed
        // number: a bigger bench is a standing cost, not a one-off purchase.
        if session.budget > super::weekly_outgoings(session, &data.config.payroll) * 6 {
            if let Some(recruit) = session.recruits.first().cloned() {
                if session.hire(data, &recruit).is_ok() {
                    log.hires += 1;
                    session.tally.hires += 1;
                }
            }
        }

        if let Some(target_id) = richest_openable_mark(session, data) {
            // Scout the mark it means to run, a door at a time, for as long as
            // the week's attention and the money both hold out.
            let doors = data
                .targets
                .get(&target_id)
                .map(|target| target.encounters.len())
                .unwrap_or(0);
            while session.attention_left_this_week(&data.config) > 0 {
                let Some(index) = session
                    .board
                    .iter()
                    .position(|entry| entry.target_id == target_id)
                else {
                    break;
                };
                let entry = &session.board[index];
                if entry.is_fully_cased(doors)
                    || session.budget < entry.next_casing_cost(&data.config)
                {
                    break;
                }
                session.budget -= entry.next_casing_cost(&data.config);
                session.board[index].casing += 1;
                session.attention_spent_this_week += 1;
            }

            if session.available_crew(&data.config.condition).count() > 0 {
                let target = data.targets.get(&target_id).cloned();
                if let Some(target) = target {
                    let plan = super::auto_assign(session, data, &target);
                    if !plan.assignments.is_empty() {
                        let report = super::run_job(session, data, &plan);
                        log.jobs_run += 1;
                        log.jobs_won += usize::from(report.success);
                        log.loot_found += report.loot.len();
                        log.injuries_taken +=
                            report.doors.iter().filter(|d| d.injury.is_some()).count();
                        log.narrative_lines
                            .extend(report.doors.iter().map(|d| d.narrative.clone()));
                    }
                }
            }
        }

        let summary = super::advance_week(session, data);
        super::award(session, &data.config, &data.awards);

        log.weeks_short += usize::from(summary.payroll.was_short());
        log.walkouts += summary.payroll.walkouts.len();
        if let Some(event) = &summary.law {
            log.law_incidents += 1;
            log.arrests += usize::from(event.taken.is_some());
        }
        if let Some((_, worst)) = session.scrutiny.watched(&data.config.scrutiny).first() {
            log.weeks_under_watch += 1;
            log.watch_peak = log.watch_peak.max(*worst);
        }
    }

    let levels: i32 = session
        .crew
        .iter()
        .map(|member| member.progression.level)
        .sum();
    log.levels_gained = levels - starting_levels - log.hires as i32;
    log
}

/// What an unattended fixer spends money on before they spend it on work: the
/// people who are threatening to leave, the ones already in a cell, and the
/// city's attention. Only ever with money the outfit can spare — a headless
/// campaign that bankrupts itself buying goodwill proves nothing.
fn keep_the_outfit_standing(session: &mut GameSession, data: &GameData, log: &mut CampaignLog) {
    let payroll = &data.config.payroll;
    let comfortable = super::weekly_outgoings(session, payroll) * 4;

    // Nobody works from a cell, so bail comes first.
    let held: Vec<String> = session
        .custody
        .iter()
        .filter(|record| session.budget - record.bail > comfortable)
        .map(|record| record.member.id.clone())
        .collect();
    for id in held {
        if super::post_bail(session, &data.config, &id).is_ok() {
            log.bails_posted += 1;
        }
    }

    // Then anybody who has said they are done.
    let wavering: Vec<String> = session
        .crew
        .iter()
        .filter(|member| member.condition.notice_given)
        .filter(|member| session.budget - super::bonus_cost(member, payroll) > comfortable)
        .map(|member| member.id.clone())
        .collect();
    for id in wavering {
        let _ = super::pay_bonus(session, &data.config, &id);
    }

    // Then the city, but only once it is genuinely dangerous.
    if super::attention_chance(session, &data.config.law) >= 0.25
        && session.budget - super::bribe_cost(session, &data.config.law) > comfortable
    {
        let _ = super::grease_palms(session, &data.config);
    }
}

/// The best-paying mark the crew's name currently opens, judged on what it is
/// worth today rather than what it was advertised at — a mark left sitting has
/// been ripening (GDD 5.4).
fn richest_openable_mark(session: &GameSession, data: &GameData) -> Option<String> {
    session
        .board
        .iter()
        .filter_map(|entry| {
            data.targets
                .get(&entry.target_id)
                .map(|target| (entry, target))
        })
        .filter(|(_, target)| target.required_reputation <= session.reputation)
        .max_by_key(|(entry, target)| {
            entry.ripened_payout(target.potential_payout, &data.config.board)
        })
        .map(|(_, target)| target.id.clone())
}

#[cfg(test)]
mod tests;
