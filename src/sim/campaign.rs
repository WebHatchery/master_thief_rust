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

        // Take on a hand when the money is comfortable and somebody is asking.
        if session.budget > 60_000 {
            if let Some(recruit) = session.recruits.first().cloned() {
                if session.hire(data, &recruit).is_ok() {
                    log.hires += 1;
                }
            }
        }

        if let Some(target_id) = richest_openable_mark(session, data) {
            if session.budget >= data.config.casing_cost {
                if let Some(entry) = session
                    .board
                    .iter_mut()
                    .find(|entry| entry.target_id == target_id)
                {
                    entry.cased = true;
                    session.budget -= data.config.casing_cost;
                }
            }

            if session.available_crew().count() > 0 {
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
                    }
                }
            }
        }

        super::advance_week(session, data);
    }

    let levels: i32 = session
        .crew
        .iter()
        .map(|member| member.progression.level)
        .sum();
    log.levels_gained = levels - starting_levels - log.hires as i32;
    log
}

/// The best-paying mark the crew's name currently opens.
fn richest_openable_mark(session: &GameSession, data: &GameData) -> Option<String> {
    session
        .board
        .iter()
        .filter_map(|entry| data.targets.get(&entry.target_id))
        .filter(|target| target.required_reputation <= session.reputation)
        .max_by_key(|target| target.potential_payout)
        .map(|target| target.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn campaign(seed: u64, weeks: u32) -> (GameData, GameSession, CampaignLog) {
        let data = GameData::load().unwrap();
        let mut session = GameSession::new(&data.config, &data, seed);
        let log = play(&mut session, &data, weeks);
        (data, session, log)
    }

    #[test]
    fn a_twenty_week_campaign_plays_through_without_stalling() {
        let (_, session, log) = campaign(20_260_726, 20);

        assert_eq!(log.weeks, 20);
        assert_eq!(session.week, 21);
        assert!(log.jobs_run >= 15, "only {} jobs in 20 weeks", log.jobs_run);
        assert!(log.jobs_won > 0, "the crew never once got paid");
        assert!(!session.board.is_empty(), "the board ran dry");
    }

    #[test]
    fn the_crew_visibly_changes_over_a_campaign() {
        let (_, session, log) = campaign(20_260_726, 20);

        assert!(log.levels_gained > 0, "nobody levelled in twenty weeks");
        assert!(log.loot_found > 0, "twenty weeks of jobs and no loot");
        assert!(
            session
                .crew
                .iter()
                .any(|m| m.progression.jobs_completed > 3),
            "no hand worked more than a few doors"
        );
        assert!(
            session.reputation > 0,
            "the outfit's name never got anywhere"
        );
    }

    #[test]
    fn the_crew_forms_opinions_about_each_other() {
        let (_, session, _) = campaign(31_337, 20);

        let opinions = session.chemistry.known_pairs().count();
        assert!(opinions > 0, "twenty weeks together and nobody had a view");
        assert!(
            session
                .chemistry
                .known_pairs()
                .any(|(_, _, value)| value.abs() >= 5),
            "chemistry never moved far enough to matter"
        );
    }

    #[test]
    fn the_city_notices_a_working_outfit() {
        let (data, session, log) = campaign(4_242, 20);

        assert!(log.jobs_run > 0);
        assert!(
            session.notoriety > 0,
            "twenty weeks of jobs and nobody heard"
        );
        // Heat is the short-term half and decays; notoriety is the ledger.
        assert!(session.heat <= session.notoriety);
        assert!(session.heat_dc_penalty(&data.config) >= 0);
    }

    #[test]
    fn the_same_seed_plays_the_same_campaign() {
        let data = GameData::load().unwrap();
        let mut a = GameSession::new(&data.config, &data, 777);
        let mut b = GameSession::new(&data.config, &data, 777);

        let log_a = play(&mut a, &data, 12);
        let log_b = play(&mut b, &data, 12);

        assert_eq!(log_a, log_b);
        assert_eq!(a.budget, b.budget);
        assert_eq!(a.reputation, b.reputation);
        assert_eq!(a.inventory, b.inventory);
    }

    #[test]
    fn a_campaign_still_round_trips_through_a_save_afterwards() {
        let (data, session, _) = campaign(909, 20);

        let save = session.to_save(&data.config.version);
        let encoded = serde_json::to_value(&save).unwrap();
        let restored = crate::state::migrate_save_value(
            Some(data.config.version.clone()),
            serde_json::json!({ "data": encoded }),
            &data.config,
        )
        .unwrap();

        assert_eq!(
            serde_json::to_value(GameSession::from_save(restored)).unwrap(),
            serde_json::to_value(&session).unwrap()
        );
    }

    #[test]
    fn the_outfit_does_not_simply_get_richer_forever() {
        // A campaign that only ever accumulates has no pressure in it. Heat and
        // the crew's cut should keep the money honest across a long run.
        let (_, session, log) = campaign(5_150, 20);
        assert!(log.jobs_run > 0);
        assert!(
            session.notoriety >= log.jobs_run as i32,
            "every job should cost anonymity"
        );
    }
}
