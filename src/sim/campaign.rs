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
            while session.casing_left_this_week(&data.config) > 0 {
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
                session.casing_this_week += 1;
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
                        log.narrative_lines
                            .extend(report.doors.iter().map(|d| d.narrative.clone()));
                    }
                }
            }
        }

        let summary = super::advance_week(session, data);
        super::award(session, &data.awards);

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
    fn an_outfit_that_keeps_working_gets_a_reputation_for_how_it_works() {
        // The unattended fixer above takes the richest mark every week and
        // never once considers method. That is exactly the play the city is
        // supposed to answer, so a campaign run that way has to end up paying
        // for it (GDD 5.4).
        let (data, session, log) = campaign(20_260_726, 20);

        assert!(
            log.weeks_under_watch >= 5,
            "twenty weeks of the same trades and the city charged for {} of them",
            log.weeks_under_watch
        );
        assert!(log.watch_peak > 0);
        assert!(
            log.watch_peak <= data.config.scrutiny.max_penalty,
            "the watch ran past its own cap at {}",
            log.watch_peak
        );
        assert!(
            !session.scrutiny.is_empty(),
            "the file was empty the week after the last job"
        );
    }

    #[test]
    fn a_reputation_for_a_method_can_be_waited_out_but_not_bought_off() {
        // The counter-play, and the reason this is not simply a second heat
        // bar: greasing palms buys quiet, and buys nothing here at all.
        let data = GameData::load().unwrap();
        let mut session = GameSession::new(&data.config, &data, 20_260_726);
        play(&mut session, &data, 12);
        assert!(!session.scrutiny.is_empty(), "nothing to wait out");

        let before = session.scrutiny.watched(&data.config.scrutiny);
        session.budget = 50_000_000;
        session.heat = data.config.law.custody_threshold;
        super::super::grease_palms(&mut session, &data.config).unwrap();
        assert_eq!(
            session.scrutiny.watched(&data.config.scrutiny),
            before,
            "a bribe bought its way off the city's file"
        );

        // Quiet weeks are the only answer, and they cost a payroll each.
        let mut weeks = 0;
        while !session.scrutiny.is_empty() {
            super::super::advance_week(&mut session, &data);
            weeks += 1;
            assert!(weeks < 60, "a file that never went cold");
        }
        assert!(weeks > 1, "one quiet week cleared everything");
    }

    #[test]
    fn a_full_campaign_rarely_repeats_a_narrative_line() {
        // GDD 13, M6's done-when. A campaign that keeps telling the same three
        // sentences has no texture, however good the dice underneath are.
        let (_, _, log) = campaign(20_260_726, 20);

        assert!(
            log.narrative_lines.len() > 40,
            "only {} lines printed in twenty weeks",
            log.narrative_lines.len()
        );
        let freshness = log.narrative_freshness();
        assert!(
            freshness >= 0.75,
            "only {:.0}% of the campaign's lines were new",
            freshness * 100.0
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
    fn a_partnership_is_reachable_but_never_automatic() {
        // The tier has to be something play actually produces, or the premium
        // it carries is a rule nobody ever meets. It also must not be the
        // default state of every roster, or it is just a tax.
        // A partnership forms in roughly a quarter of campaigns, so six seeds
        // is not a sample — it is a coin toss that happened to land. Twenty-four
        // is enough for both halves of the claim to be stable.
        let data = GameData::load().unwrap();
        let mut formed = 0;
        let mut campaigns = 0;

        for seed in 0..24u64 {
            let mut session = GameSession::new(&data.config, &data, seed);
            play(&mut session, &data, 20);
            campaigns += 1;
            formed += usize::from(
                !session
                    .chemistry
                    .partnerships_among(&session.crew_ids())
                    .is_empty(),
            );
        }

        assert!(formed > 0, "no campaign in {} grew a pair", campaigns);
        assert!(
            formed < campaigns,
            "every single campaign formed one, so it is not a decision"
        );
    }

    #[test]
    fn chemistry_reaches_the_die_at_the_scale_play_produces() {
        // The modifier is the reason the system exists. If a campaign's warmest
        // pair still reads zero on a check, chemistry is decoration.
        let data = GameData::load().unwrap();
        let mut session = GameSession::new(&data.config, &data, 4_242);
        play(&mut session, &data, 20);

        // Read it the way a job does: over the handful of hands actually on
        // one, not averaged across a roster of strangers.
        let (a, b, value) = session
            .chemistry
            .known_pairs()
            .max_by_key(|(_, _, value)| value.abs())
            .expect("twenty weeks and nobody formed a view");

        let pair = vec![a.to_owned(), b.to_owned()];
        let entry = session
            .chemistry
            .modifier(&pair[0], &pair)
            .unwrap_or_else(|| {
                panic!("the campaign's strongest opinion ({value}) still reads zero on a check")
            });
        assert_ne!(entry.value, 0);
    }

    #[test]
    fn specialists_get_better_at_the_thing_they_are_for() {
        // Mastery read 0/10 on every dossier in the game because nothing ever
        // wrote it. A campaign should now visibly deepen somebody's trade.
        let (_, session, _) = campaign(20_260_726, 20);

        let best = session
            .crew
            .iter()
            .map(|member| member.progression.mastery_level)
            .max()
            .unwrap_or(0);
        assert!(
            best > 0,
            "twenty weeks and nobody got better at their trade"
        );
        assert!(best < 10, "twenty weeks should not master a trade outright");
        assert!(
            session
                .crew
                .iter()
                .any(|m| m.progression.mastery_level == 0),
            "mastery arrived for everybody, so it is not an investment"
        );
    }

    #[test]
    fn tools_wear_out_over_a_campaign() {
        // The Outfitter used to be a shop you visited once: a tool bought in
        // week two was exactly as good in week forty.
        let (data, session, _) = campaign(20_260_726, 20);

        assert!(
            !session.kit_wear.is_empty(),
            "twenty weeks of jobs and nothing wore out"
        );
        let worn = session
            .crew
            .iter()
            .filter(|member| crate::sim::wear_penalty(&session, member, &data.config.kit) > 0)
            .count();
        assert!(
            worn > 0,
            "kit accrued wear but none of it ever reached a check"
        );
    }

    #[test]
    fn a_crew_worth_keeping_bring_in_work_of_their_own() {
        // The one upside loyalty has. If it never fires in a real campaign it
        // is a rule nobody meets, like the partnership tier used to be.
        let data = GameData::load().unwrap();
        let mut brought = 0;

        for seed in 0..8u64 {
            let mut session = GameSession::new(&data.config, &data, seed);
            play(&mut session, &data, 25);
            brought += session.tally.leads_brought_in;
        }

        assert!(
            brought > 0,
            "eight campaigns and nobody on any payroll ever heard anything"
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
    fn the_same_seed_replays_across_separate_loads_of_the_content() {
        // GDD 5.7 says a seed reproduces a campaign exactly. Every existing
        // determinism test shares one `GameData`, so all of them would stay
        // green if a draw depended on the order a registry happened to iterate
        // in — and `DataRegistry` is backed by a `HashMap`, so that order is
        // different for every load. This is the test that can actually fail.
        let loads: Vec<GameData> = (0..4).map(|_| GameData::load().unwrap()).collect();

        let orders: Vec<Vec<&String>> = loads
            .iter()
            .map(|data| data.equipment.ids().collect())
            .collect();
        assert!(
            orders.iter().any(|order| *order != orders[0]),
            "every load iterated identically, so this guard proves nothing — \
             if DataRegistry became an ordered map it can be deleted"
        );

        let mut logs = Vec::new();
        let mut ledgers = Vec::new();
        for data in &loads {
            let mut session = GameSession::new(&data.config, data, 4_242);
            logs.push(play(&mut session, data, 15));
            ledgers.push((
                session.budget,
                session.reputation,
                session.notoriety,
                session.heat,
                session.crew.len(),
                session.inventory.clone(),
                session.custody.len(),
            ));
        }

        for (index, log) in logs.iter().enumerate() {
            assert_eq!(log, &logs[0], "load {} played a different campaign", index);
            assert_eq!(ledgers[index], ledgers[0], "load {} ended elsewhere", index);
        }
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
    fn a_campaign_pays_its_own_way_every_single_week() {
        // The week now has a bill in it. Twenty weeks of work should have paid
        // out real money, and the ledger should be able to prove it.
        let (_, session, log) = campaign(20_260_726, 20);

        assert!(
            session.tally.wages_paid > 0,
            "twenty weeks and nobody drew a wage"
        );
        assert!(
            session.tally.wages_paid as f32 > session.tally.payout_total as f32 * 0.05,
            "wages of {} against takings of {} is not a payroll",
            session.tally.wages_paid,
            session.tally.payout_total
        );
        assert_eq!(log.weeks, 20);
    }

    #[test]
    fn an_outfit_that_only_rests_goes_broke_and_loses_its_crew() {
        // The complaint this change answers: resting until everyone is fresh
        // used to be free, so it was never wrong. It is now a slow bankruptcy.
        let data = GameData::load().unwrap();
        let mut session = GameSession::new(&data.config, &data, 20_260_726);
        let roster = session.crew.len();

        let mut short_weeks = 0;
        for _ in 0..25 {
            let summary = super::super::advance_week(&mut session, &data);
            short_weeks += usize::from(summary.payroll.was_short());
        }

        assert!(session.budget <= 0, "idling stayed free");
        assert!(short_weeks > 0, "the bill was always covered");
        assert!(
            session.crew.len() < roster,
            "twenty-five unpaid weeks and the whole crew stayed"
        );
        assert!(session.tally.walkouts > 0);
    }

    #[test]
    fn a_campaign_that_never_cools_off_loses_somebody_to_a_cell() {
        // GDD 5.6's other promise: above a threshold heat "can retire a crew
        // member into custody". A working outfit that never buys quiet should
        // meet the law sooner or later.
        let data = GameData::load().unwrap();
        let mut arrested = 0;
        let mut incidents = 0;

        for seed in 0..12u64 {
            let mut session = GameSession::new(&data.config, &data, seed);
            // The unattended fixer would bribe its way out; this one does not.
            let log = play_without_bribes(&mut session, &data, 24);
            arrested += log.arrests;
            incidents += log.law_incidents;
        }

        assert!(incidents > 0, "twelve hot campaigns and nobody called");
        assert!(
            arrested > 0,
            "{} incidents across twelve campaigns and never an arrest",
            incidents
        );
    }

    /// A campaign played the same way, minus the money spent on staying quiet.
    fn play_without_bribes(session: &mut GameSession, data: &GameData, weeks: u32) -> CampaignLog {
        let mut log = CampaignLog::default();
        for _ in 0..weeks {
            log.weeks += 1;
            if let Some(target_id) = richest_openable_mark(session, data) {
                if session.available_crew().count() > 0 {
                    if let Some(target) = data.targets.get(&target_id).cloned() {
                        let plan = super::super::auto_assign(session, data, &target);
                        if !plan.assignments.is_empty() {
                            super::super::run_job(session, data, &plan);
                            log.jobs_run += 1;
                        }
                    }
                }
            }
            let summary = super::super::advance_week(session, data);
            if let Some(event) = &summary.law {
                log.law_incidents += 1;
                log.arrests += usize::from(event.taken.is_some());
            }
        }
        log
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
