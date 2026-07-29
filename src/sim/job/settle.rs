//! What a finished job does to the campaign: the money, the standing, the
//! lockup, the wear on the kit, and what the city learned watching it.
//!
//! Split out of `job.rs` on its own responsibility — resolving doors and
//! settling the bill are two jobs, and the file was carrying both.

use super::{DoorOutcome, JobPlan, JobReport};
use crate::data::GameData;
use crate::model::{HeistTarget, Skill};
use crate::rules::outcome::Outcome;
use crate::state::GameSession;

/// Everything the job owes and is owed, applied in one place.
#[allow(clippy::too_many_arguments)]
pub fn settle(
    session: &mut GameSession,
    data: &GameData,
    target: &HeistTarget,
    plan: &JobPlan,
    doors: Vec<DoorOutcome>,
    delegation_misses: Vec<crate::sim::delegation::DelegationMiss>,
    called_off_with: Option<usize>,
) -> JobReport {
    let passed = doors.iter().filter(|door| door.result.passed()).count();
    // A job the crew walked out of is scored against the building, not against
    // the doors they got as far as. Clearing two of three and leaving is two
    // thirds of a job; scoring it on what was attempted would make it a clean
    // sweep, which is the opposite of what happened.
    let attempted = doors.len();
    let against = match called_off_with {
        Some(left) => attempted + left,
        None => attempted,
    };
    let rate = if against == 0 {
        0.0
    } else {
        passed as f32 / against as f32
    };
    // Leaving is never a win. The crew are out, whole, and the score is still
    // in the building.
    let pay = &data.config.payout;
    let success = called_off_with.is_none() && rate >= pay.success_threshold;

    // What the mark is worth today, not what it was worth when it appeared:
    // every week the crew left it sitting added to the take (GDD 5.4).
    let worth = session
        .board_entry(&target.id)
        .map(|entry| entry.ripened_payout(target.potential_payout, &data.config.board))
        .unwrap_or(target.potential_payout);

    let walk = &data.config.walk_away;
    // All three cases scale with how far the crew actually got. The failed one
    // used to be a flat share of the mark whatever happened, so on a job they
    // were losing, opening one more door was worth nothing and a total wipeout
    // paid the same as a near miss.
    let share = match (success, called_off_with.is_some()) {
        (true, _) if rate > pay.clean_threshold => pay.clean_bonus,
        (true, _) => 1.0,
        // Whatever they were carrying when the order came, at a fence's rate
        // for a half-finished job. Walking early is worth less than walking
        // late, and both are worth less than the score.
        (_, true) => pay.walked_share,
        _ => pay.failed_share,
    };
    let payout = (worth as f32 * rate * share) as i64;
    // What the hands who worked it want for having worked it, negotiated
    // against who they are and how they feel about the outfit (GDD 5.5).
    let cut = crate::sim::payroll::crew_cut_for(
        session,
        &data.config,
        &plan.crew_on_job(),
        plan.delegated,
    );
    let net = cut.net_of(payout);

    // The whole reason to set a standing order: a crew who leave when it starts
    // going wrong leave less behind. No botched-job notoriety at all, and the
    // job's own is discounted — that, and the doors nobody had to open, is what
    // the forfeited score buys (GDD 5.6).
    let notoriety = if called_off_with.is_some() {
        ((target.notoriety as f32 * walk.notoriety_share).round() as i32).max(1)
    } else {
        target.notoriety
            + if success {
                0
            } else {
                data.config.failure_notoriety
            }
    };
    let reputation = if success {
        data.config.reputation_per_job * difficulty_weight(target)
    } else {
        0
    };

    // What the crew carried out besides the money (GDD 0, "Build").
    let outcomes: Vec<Outcome> = doors.iter().map(|door| door.result.outcome).collect();
    let loot = crate::sim::loot::roll_loot(&mut session.rng, data, target, &outcomes, success);
    session.inventory.extend(loot.iter().cloned());

    // A night out is a night out: every hand who worked wears what they carried
    // once, not once per door (GDD 3, "repair equipment").
    for member_id in plan.crew_on_job() {
        crate::sim::kit::wear_kit(session, &member_id);
    }

    let was_cased = session
        .board_entry(&target.id)
        .map(|entry| !entry.is_blind())
        .unwrap_or(false);

    session.budget += net;
    session.notoriety += notoriety;
    session.heat += notoriety;
    session.reputation += reputation;
    let trades_noticed = note_the_method(session, data, &doors);
    session.board.retain(|entry| entry.target_id != target.id);

    record_job(
        session, target, plan, &doors, success, net, &loot, was_cased,
    );
    if let Some(left) = called_off_with {
        session.tally.jobs_called_off += 1;
        session.tally.doors_left_standing += left as i64;
    }

    JobReport {
        target_id: target.id.clone(),
        target_name: target.name.clone(),
        doors,
        success,
        loot,
        delegation_misses,
        gross: payout,
        cut,
        payout: net,
        notoriety_gained: notoriety,
        reputation_gained: reputation,
        heat_gained: notoriety,
        trades_noticed,
        called_off_with,
        delegated: plan.delegated,
    }
}

/// Tell the city what it just watched. Every door the crew worked teaches its
/// trade to every building in town, and getting through one teaches more than
/// being beaten by it (GDD 5.4).
///
/// Counted door by door rather than once per job on purpose: a mark whose three
/// doors are all wires is exactly the mark that makes the outfit famous for
/// wires, and the arithmetic has to say so.
fn note_the_method(
    session: &mut GameSession,
    data: &GameData,
    doors: &[DoorOutcome],
) -> Vec<(Skill, i32)> {
    let tuning = &data.config.scrutiny;
    let mut noticed: Vec<(Skill, i32)> = Vec::new();

    for door in doors {
        let skill = door.result.check.skill;
        let attention = if door.result.passed() {
            tuning.per_door_cleared
        } else {
            tuning.per_door_failed
        };
        session.scrutiny.note(skill, attention, tuning);

        match noticed.iter_mut().find(|(seen, _)| *seen == skill) {
            Some((_, total)) => *total += attention,
            None => noticed.push((skill, attention)),
        }
    }

    noticed.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    noticed
}

/// Everything the records screen and the achievements read afterwards.
#[allow(clippy::too_many_arguments)]
fn record_job(
    session: &mut GameSession,
    target: &HeistTarget,
    plan: &JobPlan,
    doors: &[DoorOutcome],
    success: bool,
    payout: i64,
    loot: &[String],
    was_cased: bool,
) {
    let passed = doors.iter().filter(|door| door.result.passed()).count();
    let tally = &mut session.tally;

    tally.jobs_run += 1;
    if success {
        tally.jobs_won += 1;
    } else {
        tally.jobs_lost += 1;
    }
    if passed == doors.len() && !doors.is_empty() {
        tally.clean_sweeps += 1;
    }
    tally.doors_cleared += passed as i64;
    tally.critical_successes += doors
        .iter()
        .filter(|d| d.result.outcome == Outcome::CriticalSuccess)
        .count() as i64;
    tally.critical_failures += doors
        .iter()
        .filter(|d| d.result.outcome == Outcome::CriticalFailure)
        .count() as i64;
    tally.complications_faced += doors.iter().filter(|d| d.was_complication).count() as i64;
    tally.injuries_taken += doors.iter().filter(|d| d.injury.is_some()).count() as i64;
    tally.payout_total += payout.max(0);
    tally.payout_best = tally.payout_best.max(payout);
    tally.loot_found += loot.len() as i64;
    if plan.delegated {
        tally.delegated_jobs += 1;
    } else {
        tally.planned_jobs += 1;
    }
    if !was_cased {
        tally.blind_jobs += 1;
    }
    tally.heat_peak = tally.heat_peak.max(session.heat as i64);

    session.history.push(crate::state::JobRecord {
        week: session.week,
        target_name: target.name.clone(),
        difficulty: target.difficulty,
        success,
        doors_passed: passed,
        doors_total: doors.len(),
        payout,
        delegated: plan.delegated,
        reputation: session.reputation,
        notoriety: session.notoriety,
    });
}

fn difficulty_weight(target: &HeistTarget) -> i32 {
    use crate::model::DifficultyBand;
    match target.difficulty {
        DifficultyBand::Easy => 1,
        DifficultyBand::Medium => 2,
        DifficultyBand::Hard => 3,
        DifficultyBand::Extreme => 5,
    }
}

pub fn empty_report(plan: &JobPlan) -> JobReport {
    JobReport {
        target_id: plan.target_id.clone(),
        target_name: plan.target_id.clone(),
        doors: Vec::new(),
        success: false,
        loot: Vec::new(),
        delegation_misses: Vec::new(),
        gross: 0,
        cut: crate::sim::payroll::CrewCut::default(),
        payout: 0,
        notoriety_gained: 0,
        reputation_gained: 0,
        heat_gained: 0,
        trades_noticed: Vec::new(),
        called_off_with: None,
        delegated: plan.delegated,
    }
}

#[cfg(test)]
mod tests {
    use super::super::{auto_assign, run_job};
    use super::*;
    use crate::rules::Scrutiny;

    fn setup(seed: u64) -> (GameData, GameSession) {
        let data = GameData::load().unwrap();
        let session = GameSession::new(&data.config, &data, seed);
        (data, session)
    }

    fn first_target<'a>(data: &'a GameData, session: &GameSession) -> &'a HeistTarget {
        data.targets.get(&session.board[0].target_id).unwrap()
    }

    #[test]
    fn waiting_on_a_mark_pays_more_than_taking_it_fresh() {
        // The bet the ripening creates: the same job, the same seed, the same
        // plan — worth measurably more for having been left alone.
        let data = GameData::load().unwrap();
        let target = data.targets.get("velvet_room").unwrap().clone();

        let payout_at = |ripeness: u32| {
            let mut session = GameSession::new(&data.config, &data, 8_080);
            session.board.retain(|entry| entry.target_id == target.id);
            if session.board.is_empty() {
                session
                    .board
                    .push(crate::state::BoardEntry::new(&target.id));
            }
            session.board[0].ripeness = ripeness;
            // The doors are harder, so hold the dice still and read the money.
            for member in &mut session.crew {
                member.training = crate::model::Skills {
                    stealth: 40,
                    athletics: 40,
                    combat: 40,
                    lockpicking: 40,
                    hacking: 40,
                    social: 40,
                };
            }
            let plan = auto_assign(&session, &data, &target);
            run_job(&mut session, &data, &plan).payout
        };

        let fresh = payout_at(0);
        let ripe = payout_at(data.config.board.ripeness_max);
        assert!(
            ripe > fresh,
            "a mark left three weeks paid {} against {} taken fresh",
            ripe,
            fresh
        );
    }

    #[test]
    fn the_report_shows_the_crew_taking_their_share_of_the_gross() {
        // The results screen reads all three numbers, so the job has to settle
        // them consistently: gross, what the crew took, what reached the outfit.
        let (data, mut session) = setup(2_468);
        let target = first_target(&data, &session).clone();
        let plan = auto_assign(&session, &data, &target);

        let report = run_job(&mut session, &data, &plan);

        assert!(report.gross > 0);
        assert_eq!(report.cut.net_of(report.gross), report.payout);
        assert!(report.payout < report.gross, "the crew worked for nothing");
        assert!(
            !report.cut.reasons.is_empty(),
            "the share moved off the base rate and said nothing about why"
        );
    }

    #[test]
    fn a_finished_mark_leaves_the_board() {
        let (data, mut session) = setup(606);
        let target = first_target(&data, &session).clone();
        let plan = auto_assign(&session, &data, &target);

        run_job(&mut session, &data, &plan);
        assert!(session.board_entry(&target.id).is_none());
    }

    #[test]
    fn every_door_the_crew_worked_teaches_the_city_its_trade() {
        let (data, mut session) = setup(1_705);
        let target = first_target(&data, &session).clone();
        let plan = auto_assign(&session, &data, &target);
        assert!(session.scrutiny.is_empty());

        let report = run_job(&mut session, &data, &plan);

        assert!(
            !report.trades_noticed.is_empty(),
            "a whole job and the city learned nothing"
        );
        for (skill, attention) in &report.trades_noticed {
            assert!(*attention > 0);
            assert_eq!(session.scrutiny.get(*skill), *attention);
        }
        let doors_by_trade: usize = report.trades_noticed.len();
        assert!(doors_by_trade <= report.doors.len());
    }

    #[test]
    fn a_job_walked_out_of_is_scored_against_the_building_not_the_doors_tried() {
        // Two of three cleared and out is two thirds of a job. Scoring it on
        // what was attempted would call it a clean sweep, which is the opposite
        // of what happened — and would have paid like one.
        let (data, mut session) = setup(9_140);
        let target = first_target(&data, &session).clone();
        let mut plan = auto_assign(&session, &data, &target);
        plan.delegated = false;
        plan.walk_after = Some(1);
        let doors = plan.assignments.len();

        let report = run_job(&mut session, &data, &plan);

        assert_eq!(report.doors_total(), doors.max(report.doors.len()));
        assert!(report.success_rate() <= 1.0);
        if report.was_called_off() {
            assert!(
                report.success_rate() < 1.0,
                "a job the crew walked out of read as every door cleared"
            );
        }
    }

    #[test]
    fn every_door_cleared_is_worth_something_even_on_a_job_being_lost() {
        // A failed job used to pay a flat share of the mark whatever happened,
        // so on a job the crew were losing, getting one more door open was
        // worth exactly nothing — and being wiped out paid the same as coming
        // one door short. Both halves of that are now false.
        let data = GameData::load().unwrap();
        let target = data.targets.get("velvet_room").unwrap().clone();
        let worth = target.potential_payout as f32;
        let pay = &data.config.payout;

        let failed = |cleared: usize, total: usize| {
            let rate = cleared as f32 / total as f32;
            assert!(rate < pay.success_threshold, "that is not a failed job");
            (worth * rate * pay.failed_share) as i64
        };

        assert_eq!(failed(0, 3), 0, "a total wipeout still paid out");
        assert!(failed(1, 3) > failed(0, 3));
        assert!(failed(1, 4) > 0);
        assert!(
            failed(1, 3) > failed(1, 4),
            "getting a third of the way in paid the same as a quarter"
        );
    }

    #[test]
    fn staying_pays_better_per_door_than_leaving_and_the_score_beats_both() {
        // The three shares have to stay in this order or the standing order
        // stops being a trade: leaving buys safety at a price, staying is worth
        // more per door because they were in there longer, and neither is worth
        // finishing the job.
        let pay = &GameData::load().unwrap().config.payout;

        assert!(pay.walked_share < pay.failed_share);
        assert!(pay.failed_share < 1.0);
        assert!(pay.clean_bonus > 1.0);
        assert!(pay.clean_threshold > pay.success_threshold);
    }

    #[test]
    fn walking_late_is_worth_more_than_walking_early() {
        // The curve that makes the standing order a judgement rather than a
        // switch: a tighter order is safer and poorer, and the player is
        // choosing where on that line to sit.
        let data = GameData::load().unwrap();
        let target = data.targets.get("velvet_room").unwrap().clone();
        let worth = target.potential_payout as f32;
        let pay = &data.config.payout;

        let quoted = |cleared: usize, total: usize| {
            (worth * (cleared as f32 / total as f32) * pay.walked_share) as i64
        };

        assert!(quoted(2, 3) > quoted(1, 3));
        assert!(quoted(1, 3) > quoted(0, 3));
        // And never as much as finishing it: the forfeited score is the price.
        assert!(quoted(3, 3) < (worth * 1.0) as i64);
    }

    #[test]
    fn getting_in_teaches_the_city_more_than_being_beaten_does() {
        // Otherwise the pressure would land hardest on the crew already having
        // the worst week, which is the wrong shape for a cost the player is
        // meant to spend down deliberately.
        let tuning = &GameData::load().unwrap().config.scrutiny;
        assert!(tuning.per_door_cleared > tuning.per_door_failed);
        assert!(tuning.per_door_failed > 0, "a failed door taught nothing");
    }

    #[test]
    fn a_file_the_city_already_has_shows_up_on_the_next_job_by_name() {
        // The whole contract: what the settlement writes has to be readable on
        // the planning screen before the next commit (pillar 2).
        let (data, mut session) = setup(1_706);
        let target = first_target(&data, &session).clone();
        let door = data.encounters_for(&target)[0].clone();
        let trade = door.primary_skill;

        session
            .scrutiny
            .note(trade, data.config.scrutiny.ceiling(), &data.config.scrutiny);
        let check = crate::sim::plan::candidate_check(
            &session,
            &data,
            &target,
            &door,
            &session.crew[0],
            &[],
        );

        let entry = check
            .entries
            .iter()
            .find(|entry| entry.label == Scrutiny::label(trade))
            .expect("the city's file is charged by name");
        assert_eq!(entry.value, -data.config.scrutiny.max_penalty);
    }
}
