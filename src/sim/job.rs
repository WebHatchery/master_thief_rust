//! Running a job: door by door, in order, visibly.

use crate::data::GameData;
use crate::model::crew::Injury;
use crate::model::{Encounter, HeistTarget, RunEffect};
use crate::rules::attributes::{award_experience, work_the_trade};
use crate::rules::encounter::{build_check, resolve_with_effects, CheckInputs, EncounterResult};
use crate::rules::outcome::Outcome;
use crate::sim::plan::situational_modifiers;
use crate::state::GameSession;

/// Who the fixer put on which door.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    pub encounter_id: String,
    pub member_id: String,
}

/// A plan, ready to commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobPlan {
    pub target_id: String,
    pub assignments: Vec<Assignment>,
    /// True when the auto-assigner built this rather than the player.
    pub delegated: bool,
}

impl JobPlan {
    /// Everybody down for this job, each named once.
    pub fn crew_on_job(&self) -> Vec<String> {
        let mut crew: Vec<String> = self
            .assignments
            .iter()
            .map(|assignment| assignment.member_id.clone())
            .collect();
        crew.sort();
        crew.dedup();
        crew
    }
}

/// One resolved door.
#[derive(Debug, Clone)]
pub struct DoorOutcome {
    pub encounter_name: String,
    pub result: EncounterResult,
    pub narrative: String,
    pub injury: Option<Injury>,
    /// A door the run inserted after a critical failure.
    pub was_complication: bool,
    /// What the critical actually did, in the encounter's own words. GDD 5.2
    /// says a critical "fires the encounter's `critical_failure_effect` /
    /// `critical_success_reward`" — fifty-six of them are authored, and until
    /// now none of them reached the player.
    pub critical_effect: Option<String>,
}

impl DoorOutcome {
    /// How the run itself was rewritten, if it was. A skipped door and an extra
    /// one are the two things a critical can do to a plan, and the player is
    /// entitled to be told which happened (GDD 5.2, pillar 2).
    pub fn structural_note(&self) -> Option<&'static str> {
        match self.result.run_effect {
            RunEffect::SkipNext => Some("The next door opens with it."),
            RunEffect::AddComplication => Some("Something else is waiting now."),
            RunEffect::None => None,
        }
    }
}

/// What the job did to the campaign.
#[derive(Debug, Clone)]
pub struct JobReport {
    pub target_id: String,
    pub target_name: String,
    pub doors: Vec<DoorOutcome>,
    pub success: bool,
    /// Equipment ids carried out along with the money.
    pub loot: Vec<String>,
    /// Doors a better hand was free for. Empty for a hand-made plan, which is
    /// the player's own business (GDD 5.3).
    pub delegation_misses: Vec<super::delegation::DelegationMiss>,
    /// The take before the crew were paid for the job.
    pub gross: i64,
    /// What they wanted for it, and why.
    pub cut: super::payroll::CrewCut,
    /// What reached the outfit after the crew took theirs.
    pub payout: i64,
    pub notoriety_gained: i32,
    pub reputation_gained: i32,
    pub heat_gained: i32,
    pub delegated: bool,
}

impl JobReport {
    pub fn doors_passed(&self) -> usize {
        self.doors
            .iter()
            .filter(|door| door.result.passed())
            .count()
    }

    pub fn success_rate(&self) -> f32 {
        if self.doors.is_empty() {
            0.0
        } else {
            self.doors_passed() as f32 / self.doors.len() as f32
        }
    }
}

/// Greedy best-fit: put the highest total on each door in order, and never the
/// same hand twice while somebody else is standing idle. This is the same rule
/// the original used for automation — delegation runs the engine with a worse
/// assignment, never a kinder one (GDD 5.3).
pub fn auto_assign(session: &GameSession, data: &GameData, target: &HeistTarget) -> JobPlan {
    let mut used: Vec<String> = Vec::new();
    let mut assignments = Vec::new();

    for encounter in data.encounters_for(target) {
        let mut best: Option<(i32, String)> = None;

        for member in session.available_crew() {
            // A pair that refuses to work together is not a choice the
            // auto-assigner gets to make either (GDD 5.5).
            if used
                .iter()
                .any(|other| session.chemistry.refuses(&member.id, other))
            {
                continue;
            }

            let loadout = session.loadout(member, data);
            let check = build_check(CheckInputs {
                member,
                loadout: &loadout,
                encounter,
                tuning: &data.config.condition,
                extra: &situational_modifiers(data, target, encounter, session, &member.id, &used),
            });
            let mut score = check.bonus();
            if used.contains(&member.id) {
                score -= 4;
            }

            if best.as_ref().is_none_or(|(top, _)| score > *top) {
                best = Some((score, member.id.clone()));
            }
        }

        if let Some((_, member_id)) = best {
            if !used.contains(&member_id) {
                used.push(member_id.clone());
            }
            assignments.push(Assignment {
                encounter_id: encounter.id.clone(),
                member_id,
            });
        }
    }

    JobPlan {
        target_id: target.id.clone(),
        assignments,
        delegated: true,
    }
}

/// Commit. Resolves each door in order, applies everything the job costs, and
/// returns the report the results screen reads.
pub fn run_job(session: &mut GameSession, data: &GameData, plan: &JobPlan) -> JobReport {
    let Some(target) = data.targets.get(&plan.target_id).cloned() else {
        return empty_report(plan);
    };

    // Audited before a single die is thrown, while the crew is still in the
    // state the plan was made against.
    let delegation_misses = if plan.delegated {
        super::delegation::audit(session, data, &target, plan)
    } else {
        Vec::new()
    };

    let crew_on_job = plan.crew_on_job();
    let mut doors = Vec::new();
    let mut queue: Vec<(String, bool)> = plan
        .assignments
        .iter()
        .map(|assignment| (assignment.encounter_id.clone(), false))
        .collect();
    let mut index = 0;

    while index < queue.len() {
        let (encounter_id, was_complication) = queue[index].clone();
        index += 1;

        let Some(encounter) = data.encounters.get(&encounter_id).cloned() else {
            continue;
        };
        let Some(member_id) = assigned_member(plan, &encounter_id, session) else {
            continue;
        };

        let outcome = resolve_door(session, data, &target, &encounter, &member_id, &crew_on_job);

        match outcome.result.run_effect {
            RunEffect::SkipNext => {
                if index < queue.len() {
                    index += 1;
                }
            }
            RunEffect::AddComplication => {
                if let Some(complication) = draw_complication(session, data) {
                    queue.insert(index, (complication, true));
                }
            }
            RunEffect::None => {}
        }

        let _ = was_complication;
        record_chemistry(
            session,
            &outcome.result.check.member_id,
            &crew_on_job,
            outcome.result.outcome,
        );
        doors.push(outcome);
    }

    settle(session, data, &target, plan, doors, delegation_misses)
}

/// Everybody else on the job watched that door. What they made of it depends
/// on who they are (GDD 5.5).
fn record_chemistry(
    session: &mut GameSession,
    actor: &str,
    crew_on_job: &[String],
    outcome: Outcome,
) {
    let watchers: Vec<(String, Vec<String>)> = crew_on_job
        .iter()
        .filter(|id| id.as_str() != actor)
        .filter_map(|id| {
            session
                .member(id)
                .map(|member| (member.id.clone(), member.personality_traits.clone()))
        })
        .collect();

    crate::rules::chemistry::record_outcome(&mut session.chemistry, actor, &watchers, outcome);
}

fn assigned_member(plan: &JobPlan, encounter_id: &str, session: &GameSession) -> Option<String> {
    plan.assignments
        .iter()
        .find(|assignment| assignment.encounter_id == encounter_id)
        .map(|assignment| assignment.member_id.clone())
        // A complication has no assignment of its own; the crew's steadiest
        // available hand takes it.
        .or_else(|| {
            session
                .available_crew()
                .next()
                .map(|member| member.id.clone())
        })
}

fn draw_complication(session: &mut GameSession, data: &GameData) -> Option<String> {
    let mut pool: Vec<&String> = data
        .encounters
        .iter()
        .filter(|(_, encounter)| encounter.complication_only)
        .map(|(id, _)| id)
        .collect();
    pool.sort();

    if pool.is_empty() {
        return None;
    }
    let index = session.rng.below(pool.len());
    Some(pool[index].clone())
}

fn resolve_door(
    session: &mut GameSession,
    data: &GameData,
    target: &HeistTarget,
    encounter: &Encounter,
    member_id: &str,
    crew_on_job: &[String],
) -> DoorOutcome {
    let extras = {
        if session.member(member_id).is_none() {
            return missed_door(encounter);
        }
        situational_modifiers(data, target, encounter, session, member_id, crew_on_job)
    };

    let check = {
        let member = session.member(member_id).expect("member checked above");
        let loadout = session.loadout(member, data);
        build_check(CheckInputs {
            member,
            loadout: &loadout,
            encounter,
            tuning: &data.config.condition,
            extra: &extras,
        })
    };

    let roll = (session.rng.below(20) + 1) as i32;
    let result = resolve_with_effects(
        check,
        roll,
        encounter.critical_success_run_effect,
        encounter.critical_failure_run_effect,
    );

    let line_index = session.rng.below(64);
    let narrative = data
        .outcomes
        .line(result.outcome, encounter.primary_skill, line_index)
        .unwrap_or(&encounter.failure_consequence)
        .to_owned();

    let hurt = session.rng.next_f32() < result.outcome.injury_chance();
    let injury = hurt.then(|| match result.outcome {
        Outcome::CriticalFailure => Injury::major(format!("Hurt at {}", encounter.name)),
        _ => Injury::minor(format!("Strained at {}", encounter.name)),
    });

    if let Some(member) = session.member_mut(member_id) {
        member.condition.add_fatigue(result.stress_inflicted);
        member.condition.worked_this_week = true;
        award_experience(&mut member.progression, result.experience_gained);
        member.progression.jobs_completed += 1;
        if result.passed() {
            member.progression.jobs_succeeded += 1;
            member.condition.adjust_loyalty(1);
            // A trade is learned at its own doors, cleared. Somebody else's
            // door teaches nothing, and neither does a failed one.
            if encounter.primary_skill == member.specialty_skill {
                work_the_trade(&mut member.progression, &data.config.mastery);
            }
        } else {
            member.condition.adjust_loyalty(-2);
        }
        if let Some(injury) = injury.clone() {
            member.condition.injuries.push(injury);
        }
    }

    // The encounter's own account of what a critical did. Only a critical
    // fires one, which is what makes it worth reading (GDD 5.2).
    let critical_effect = match result.outcome {
        Outcome::CriticalSuccess => encounter.critical_success_reward.clone(),
        Outcome::CriticalFailure => encounter.critical_failure_effect.clone(),
        _ => None,
    };

    DoorOutcome {
        encounter_name: encounter.name.clone(),
        result,
        narrative,
        injury,
        was_complication: encounter.complication_only,
        critical_effect,
    }
}

fn missed_door(encounter: &Encounter) -> DoorOutcome {
    DoorOutcome {
        encounter_name: encounter.name.clone(),
        result: EncounterResult {
            check: crate::rules::encounter::CheckBreakdown {
                member_id: String::new(),
                member_name: "Nobody".to_owned(),
                encounter_id: encounter.id.clone(),
                skill: encounter.primary_skill,
                dc: encounter.difficulty,
                entries: Vec::new(),
            },
            roll: 1,
            total: 1,
            outcome: Outcome::CriticalFailure,
            experience_gained: 0,
            stress_inflicted: 0,
            run_effect: RunEffect::None,
        },
        narrative: format!("Nobody was on the door at {}.", encounter.name),
        injury: None,
        was_complication: false,
        critical_effect: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn settle(
    session: &mut GameSession,
    data: &GameData,
    target: &HeistTarget,
    plan: &JobPlan,
    doors: Vec<DoorOutcome>,
    delegation_misses: Vec<super::delegation::DelegationMiss>,
) -> JobReport {
    let passed = doors.iter().filter(|door| door.result.passed()).count();
    let rate = if doors.is_empty() {
        0.0
    } else {
        passed as f32 / doors.len() as f32
    };
    let success = rate >= 0.5;

    // What the mark is worth today, not what it was worth when it appeared:
    // every week the crew left it sitting added to the take (GDD 5.4).
    let worth = session
        .board_entry(&target.id)
        .map(|entry| entry.ripened_payout(target.potential_payout, &data.config.board))
        .unwrap_or(target.potential_payout);

    let payout = if success {
        let bonus = if rate > 0.8 { 1.2 } else { 1.0 };
        (worth as f32 * rate * bonus) as i64
    } else {
        (worth as f32 * 0.15) as i64
    };
    // What the hands who worked it want for having worked it, negotiated
    // against who they are and how they feel about the outfit (GDD 5.5).
    let cut = super::payroll::crew_cut(session, &data.config, &plan.crew_on_job());
    let net = cut.net_of(payout);

    let notoriety = target.notoriety
        + if success {
            0
        } else {
            data.config.failure_notoriety
        };
    let reputation = if success {
        data.config.reputation_per_job * difficulty_weight(target)
    } else {
        0
    };

    // What the crew carried out besides the money (GDD 0, "Build").
    let outcomes: Vec<Outcome> = doors.iter().map(|door| door.result.outcome).collect();
    let loot = super::loot::roll_loot(&mut session.rng, data, target, &outcomes, success);
    session.inventory.extend(loot.iter().cloned());

    let was_cased = session
        .board_entry(&target.id)
        .map(|entry| !entry.is_blind())
        .unwrap_or(false);

    session.budget += net;
    session.notoriety += notoriety;
    session.heat += notoriety;
    session.reputation += reputation;
    session.board.retain(|entry| entry.target_id != target.id);

    record_job(
        session, target, plan, &doors, success, net, &loot, was_cased,
    );

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
        delegated: plan.delegated,
    }
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

fn empty_report(plan: &JobPlan) -> JobReport {
    JobReport {
        target_id: plan.target_id.clone(),
        target_name: plan.target_id.clone(),
        doors: Vec::new(),
        success: false,
        loot: Vec::new(),
        delegation_misses: Vec::new(),
        gross: 0,
        cut: super::payroll::CrewCut::default(),
        payout: 0,
        notoriety_gained: 0,
        reputation_gained: 0,
        heat_gained: 0,
        delegated: plan.delegated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(seed: u64) -> (GameData, GameSession) {
        let data = GameData::load().unwrap();
        let session = GameSession::new(&data.config, &data, seed);
        (data, session)
    }

    fn first_target<'a>(data: &'a GameData, session: &GameSession) -> &'a HeistTarget {
        data.targets.get(&session.board[0].target_id).unwrap()
    }

    #[test]
    fn auto_assignment_covers_every_door() {
        let (data, session) = setup(4242);
        let target = first_target(&data, &session);
        let plan = auto_assign(&session, &data, target);

        assert_eq!(plan.assignments.len(), target.encounters.len());
        for assignment in &plan.assignments {
            assert!(session.member(&assignment.member_id).is_some());
        }
    }

    #[test]
    fn auto_assignment_spreads_the_work_when_it_can() {
        let (data, session) = setup(77);
        let target = first_target(&data, &session);
        let plan = auto_assign(&session, &data, target);

        let distinct: std::collections::HashSet<&str> = plan
            .assignments
            .iter()
            .map(|a| a.member_id.as_str())
            .collect();
        assert!(distinct.len() > 1, "one hand took every door");
    }

    #[test]
    fn a_job_resolves_every_door_and_pays_out() {
        let (data, mut session) = setup(9001);
        let target = first_target(&data, &session).clone();
        let plan = auto_assign(&session, &data, &target);
        let budget = session.budget;

        let report = run_job(&mut session, &data, &plan);

        assert!(!report.doors.is_empty());
        assert!(session.budget > budget);
        assert!(report.notoriety_gained > 0);
    }

    #[test]
    fn the_same_seed_and_plan_replay_identically() {
        let (data, mut a) = setup(31337);
        let (_, mut b) = setup(31337);
        let target = first_target(&data, &a).clone();
        let plan = auto_assign(&a, &data, &target);

        let report_a = run_job(&mut a, &data, &plan);
        let report_b = run_job(&mut b, &data, &plan);

        let rolls_a: Vec<i32> = report_a.doors.iter().map(|d| d.result.roll).collect();
        let rolls_b: Vec<i32> = report_b.doors.iter().map(|d| d.result.roll).collect();
        assert_eq!(rolls_a, rolls_b);
        assert_eq!(a.budget, b.budget);
    }

    #[test]
    fn a_job_leaves_its_mark_on_the_crew_that_worked_it() {
        let (data, mut session) = setup(555);
        let target = first_target(&data, &session).clone();
        let plan = auto_assign(&session, &data, &target);
        let worked = plan.assignments[0].member_id.clone();

        let report = run_job(&mut session, &data, &plan);
        assert!(session.member(&worked).unwrap().progression.jobs_completed > 0);

        // Only a flawless run costs nothing: a critical success inflicts no
        // fatigue at all, so the assertion has to allow for one.
        let tired: i32 = session.crew.iter().map(|m| m.condition.fatigue).sum();
        let flawless = report
            .doors
            .iter()
            .all(|door| door.result.outcome == Outcome::CriticalSuccess);
        assert!(
            tired > 0 || flawless,
            "a whole job and nobody broke a sweat"
        );
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
    fn a_critical_reports_what_it_did_and_nothing_else_does() {
        // Fifty-six critical effects are authored, counted toward the GDD 8
        // content target, and were read by nothing at all. A door that crits
        // must now carry its own account of it; a door that does not, must not.
        let data = GameData::load().unwrap();
        let mut seen_effect = false;
        let mut checked = 0;

        for seed in 0..120u64 {
            let mut session = GameSession::new(&data.config, &data, seed);
            let Some(entry) = session.board.first().cloned() else {
                continue;
            };
            let Some(target) = data.targets.get(&entry.target_id).cloned() else {
                continue;
            };
            let plan = auto_assign(&session, &data, &target);
            if plan.assignments.is_empty() {
                continue;
            }
            let report = run_job(&mut session, &data, &plan);

            for door in &report.doors {
                checked += 1;
                let crit = matches!(
                    door.result.outcome,
                    Outcome::CriticalSuccess | Outcome::CriticalFailure
                );
                if !crit {
                    assert!(
                        door.critical_effect.is_none(),
                        "{} reported a critical effect without a critical",
                        door.encounter_name
                    );
                }
                seen_effect |= door.critical_effect.is_some();
            }
        }

        assert!(checked > 0);
        assert!(
            seen_effect,
            "a hundred and twenty jobs and not one critical said what it did"
        );
    }

    #[test]
    fn the_run_says_when_a_critical_rewrote_the_plan() {
        // Skipping a door and gaining one are the two things a critical can do
        // to a plan, and both used to happen silently.
        let data = GameData::load().unwrap();
        let mut notes = 0;

        for seed in 0..120u64 {
            let mut session = GameSession::new(&data.config, &data, seed);
            let Some(entry) = session.board.first().cloned() else {
                continue;
            };
            let Some(target) = data.targets.get(&entry.target_id).cloned() else {
                continue;
            };
            let plan = auto_assign(&session, &data, &target);
            if plan.assignments.is_empty() {
                continue;
            }
            for door in run_job(&mut session, &data, &plan).doors {
                if door.structural_note().is_some() {
                    notes += 1;
                    assert_ne!(door.result.run_effect, RunEffect::None);
                }
            }
        }

        assert!(
            notes > 0,
            "no run was ever rewritten in a hundred and twenty jobs"
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
    fn every_door_reports_a_narrative_line() {
        let (data, mut session) = setup(112233);
        let target = first_target(&data, &session).clone();
        let plan = auto_assign(&session, &data, &target);

        let report = run_job(&mut session, &data, &plan);
        for door in &report.doors {
            assert!(
                !door.narrative.is_empty(),
                "{} was mute",
                door.encounter_name
            );
        }
    }

    #[test]
    fn a_soak_of_jobs_keeps_the_difficulty_bands_apart() {
        // The soak's job is catching DC drift as content lands. An easy mark
        // worked by the right specialist should be reliable; the same crew
        // walking into an extreme one should not be. If those two numbers ever
        // converge, the bands have stopped meaning anything.
        let data = GameData::load().unwrap();
        let easy = door_pass_rate(&data, "velvet_room", 200);
        let extreme = door_pass_rate(&data, "harbour_vault", 200);

        assert!(
            (0.75..=0.95).contains(&easy),
            "an easy mark clears {:.0}% of its doors",
            easy * 100.0
        );
        assert!(
            (0.20..=0.70).contains(&extreme),
            "an extreme mark clears {:.0}% of its doors",
            extreme * 100.0
        );
        assert!(
            easy - extreme > 0.2,
            "easy {:.0}% and extreme {:.0}% are too close to be different jobs",
            easy * 100.0,
            extreme * 100.0
        );
    }

    /// Run one mark many times with a fresh starting crew and report the share
    /// of doors they got through.
    fn door_pass_rate(data: &GameData, target_id: &str, runs: u64) -> f32 {
        let target = data.targets.get(target_id).expect("known mark").clone();
        let mut passed = 0usize;
        let mut total = 0usize;

        for seed in 0..runs {
            let mut session = GameSession::new(&data.config, data, seed);
            let plan = auto_assign(&session, data, &target);
            if plan.assignments.is_empty() {
                continue;
            }
            let report = run_job(&mut session, data, &plan);
            passed += report.doors_passed();
            total += report.doors.len();
        }

        passed as f32 / total.max(1) as f32
    }
}
