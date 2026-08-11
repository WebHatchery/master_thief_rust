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
mod tests;
