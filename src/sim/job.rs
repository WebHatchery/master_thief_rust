//! Running a job: door by door, in order, visibly.

use crate::data::GameData;
use crate::model::crew::Injury;
use crate::model::{Encounter, HeistTarget, RunEffect};
use crate::rules::attributes::award_experience;
use crate::rules::encounter::{build_check, resolve_with_effects, CheckInputs, EncounterResult};
use crate::rules::environment::environment_entries;
use crate::rules::outcome::{ModifierEntry, Outcome};
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

/// One resolved door.
#[derive(Debug, Clone)]
pub struct DoorOutcome {
    pub encounter_name: String,
    pub result: EncounterResult,
    pub narrative: String,
    pub injury: Option<Injury>,
    /// A door the run inserted after a critical failure.
    pub was_complication: bool,
}

/// What the job did to the campaign.
#[derive(Debug, Clone)]
pub struct JobReport {
    pub target_id: String,
    pub target_name: String,
    pub doors: Vec<DoorOutcome>,
    pub success: bool,
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
            let loadout = session.loadout(member, data);
            let check = build_check(CheckInputs {
                member,
                loadout: &loadout,
                encounter,
                extra: &environment_extras(data, target, encounter, session),
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
            used.push(member_id.clone());
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

fn environment_extras(
    data: &GameData,
    target: &HeistTarget,
    encounter: &Encounter,
    session: &GameSession,
) -> Vec<ModifierEntry> {
    let mut extras = environment_entries(&target.environment, encounter.primary_skill, |id| {
        data.environment.get(id)
    });

    let heat = session.heat_dc_penalty(&data.config);
    if heat > 0 {
        extras.push(ModifierEntry::new("City heat", -heat));
    }
    extras
}

/// Commit. Resolves each door in order, applies everything the job costs, and
/// returns the report the results screen reads.
pub fn run_job(session: &mut GameSession, data: &GameData, plan: &JobPlan) -> JobReport {
    let Some(target) = data.targets.get(&plan.target_id).cloned() else {
        return empty_report(plan);
    };

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

        let outcome = resolve_door(session, data, &target, &encounter, &member_id);

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
        doors.push(outcome);
    }

    settle(session, data, &target, plan, doors)
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
) -> DoorOutcome {
    let extras = {
        let Some(member) = session.member(member_id) else {
            return missed_door(encounter);
        };
        let _ = member;
        environment_extras(data, target, encounter, session)
    };

    let check = {
        let member = session.member(member_id).expect("member checked above");
        let loadout = session.loadout(member, data);
        build_check(CheckInputs {
            member,
            loadout: &loadout,
            encounter,
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
        award_experience(&mut member.progression, result.experience_gained);
        member.progression.jobs_completed += 1;
        if result.passed() {
            member.progression.jobs_succeeded += 1;
            member.condition.adjust_loyalty(1);
        } else {
            member.condition.adjust_loyalty(-2);
        }
        if let Some(injury) = injury.clone() {
            member.condition.injuries.push(injury);
        }
    }

    DoorOutcome {
        encounter_name: encounter.name.clone(),
        result,
        narrative,
        injury,
        was_complication: encounter.complication_only,
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
    }
}

fn settle(
    session: &mut GameSession,
    data: &GameData,
    target: &HeistTarget,
    plan: &JobPlan,
    doors: Vec<DoorOutcome>,
) -> JobReport {
    let passed = doors.iter().filter(|door| door.result.passed()).count();
    let rate = if doors.is_empty() {
        0.0
    } else {
        passed as f32 / doors.len() as f32
    };
    let success = rate >= 0.5;

    let payout = if success {
        let bonus = if rate > 0.8 { 1.2 } else { 1.0 };
        (target.potential_payout as f32 * rate * bonus) as i64
    } else {
        (target.potential_payout as f32 * 0.15) as i64
    };
    let crew_cut = (payout as f32 * data.config.crew_cut) as i64;
    let net = payout - crew_cut;

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

    session.budget += net;
    session.notoriety += notoriety;
    session.heat += notoriety;
    session.reputation += reputation;
    session.board.retain(|entry| entry.target_id != target.id);

    JobReport {
        target_id: target.id.clone(),
        target_name: target.name.clone(),
        doors,
        success,
        payout: net,
        notoriety_gained: notoriety,
        reputation_gained: reputation,
        heat_gained: notoriety,
        delegated: plan.delegated,
    }
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
    fn a_job_tires_the_crew_that_worked_it() {
        let (data, mut session) = setup(555);
        let target = first_target(&data, &session).clone();
        let plan = auto_assign(&session, &data, &target);
        let worked = plan.assignments[0].member_id.clone();

        run_job(&mut session, &data, &plan);
        assert!(session.member(&worked).unwrap().condition.fatigue > 0);
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
    fn a_soak_of_jobs_keeps_outcomes_inside_designed_bands() {
        let data = GameData::load().unwrap();
        let mut passed = 0usize;
        let mut total = 0usize;

        for seed in 0..200u64 {
            let mut session = GameSession::new(&data.config, &data, seed);
            let Some(entry) = session.board.first().cloned() else {
                continue;
            };
            let target = data.targets.get(&entry.target_id).unwrap().clone();
            let plan = auto_assign(&session, &data, &target);
            let report = run_job(&mut session, &data, &plan);

            passed += report.doors_passed();
            total += report.doors.len();
        }

        let rate = passed as f32 / total as f32;
        assert!(
            (0.55..=0.90).contains(&rate),
            "starting crew clears {:.0}% of early doors",
            rate * 100.0
        );
    }
}
