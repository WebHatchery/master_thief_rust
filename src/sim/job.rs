//! Running a job: door by door, in order, visibly.

mod settle;

use crate::data::GameData;
use crate::model::crew::Injury;
use crate::model::{Encounter, HeistTarget, RunEffect, Skill};
use crate::rules::attributes::{award_experience, work_the_trade};
use crate::rules::encounter::{build_check, resolve_with_effects, CheckInputs, EncounterResult};
use crate::rules::outcome::Outcome;
use crate::sim::plan::situational_modifiers;
use crate::state::GameSession;
use settle::{empty_report, settle};

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
    /// Trades this job put on the city's file, and by how much. Worst first
    /// (GDD 5.4).
    pub trades_noticed: Vec<(Skill, i32)>,
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

        for member in session.available_crew(&data.config.condition) {
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
        let Some(member_id) = assigned_member(plan, &encounter_id, session, &data.config.condition)
        else {
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
            &data.trait_rates,
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
    rates: &crate::rules::chemistry::TraitRates,
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

    crate::rules::chemistry::record_outcome(
        &mut session.chemistry,
        actor,
        &watchers,
        outcome,
        rates,
    );
}

fn assigned_member(
    plan: &JobPlan,
    encounter_id: &str,
    session: &GameSession,
    tuning: &crate::rules::ConditionTuning,
) -> Option<String> {
    plan.assignments
        .iter()
        .find(|assignment| assignment.encounter_id == encounter_id)
        .map(|assignment| assignment.member_id.clone())
        // A complication has no assignment of its own; the crew's steadiest
        // available hand takes it.
        .or_else(|| {
            session
                .available_crew(tuning)
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
    // Load-bearing, not tidiness: `DataRegistry` is a `HashMap`, so `iter()`
    // yields a different order for every load of the content. Drawing from it
    // unsorted would make the next line's RNG draw depend on hash order, and a
    // seed would stop reproducing a campaign between runs (GDD 5.7).
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

    // A hand the fixer sent out tired is likelier to come back hurt, and that
    // is priced here rather than on the die: the draw itself happens at the
    // same point in the RNG order it always did, so the odds change and the
    // replay does not (GDD 5.7).
    let tuning = &data.config.condition;
    let spent = session
        .member(member_id)
        .is_some_and(|member| tuning.is_spent(member.condition.fatigue));
    let hurt = session.rng.next_f32() < tuning.injury_chance(result.outcome, spent);
    let injury = hurt.then(|| match result.outcome {
        Outcome::CriticalFailure => Injury::major(format!("Hurt at {}", encounter.name)),
        _ => Injury::minor(format!("Strained at {}", encounter.name)),
    });

    if let Some(member) = session.member_mut(member_id) {
        member.condition.add_fatigue(result.stress_inflicted);
        member.condition.worked_this_week = true;
        // Being sent out past the point of usefulness is remembered whatever
        // happened at the door.
        if spent {
            member.condition.adjust_loyalty(-tuning.spent_loyalty_cost);
        }
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
    fn a_crew_sent_out_spent_can_still_go_and_pays_for_it() {
        // The move the week never used to argue with was "rest until everybody
        // is fresh", because a tired hand simply could not be assigned. They
        // can now — and over two hundred jobs the difference is doors lost,
        // people hurt, and goodwill spent (GDD 5.6).
        let data = GameData::load().unwrap();

        let run = |fatigue: i32| {
            let mut hurt = 0usize;
            let mut passed = 0usize;
            let mut doors = 0usize;
            let mut goodwill = 0i32;

            for seed in 0..200u64 {
                let mut session = GameSession::new(&data.config, &data, seed);
                for member in &mut session.crew {
                    member.condition.fatigue = fatigue;
                }
                let before: i32 = session.crew.iter().map(|m| m.condition.loyalty).sum();

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

                hurt += report.doors.iter().filter(|d| d.injury.is_some()).count();
                passed += report.doors_passed();
                doors += report.doors.len();
                goodwill += session
                    .crew
                    .iter()
                    .map(|m| m.condition.loyalty)
                    .sum::<i32>()
                    - before;
            }
            (hurt, passed as f32 / doors.max(1) as f32, goodwill)
        };

        let threshold = data.config.condition.fatigue_work_threshold;
        let (fresh_hurt, fresh_rate, fresh_goodwill) = run(0);
        let (spent_hurt, spent_rate, spent_goodwill) = run(threshold + 10);

        assert!(
            spent_rate < fresh_rate,
            "a spent crew cleared {:.0}% against a fresh crew's {:.0}%",
            spent_rate * 100.0,
            fresh_rate * 100.0
        );
        assert!(
            spent_hurt > fresh_hurt,
            "{} hurt working spent against {} working fresh",
            spent_hurt,
            fresh_hurt
        );
        assert!(
            spent_goodwill < fresh_goodwill,
            "being sent out on empty cost nothing in goodwill"
        );
    }

    #[test]
    fn the_same_seed_replays_a_spent_crew_identically() {
        // The odds move; the order of the draws does not (GDD 5.7).
        let data = GameData::load().unwrap();
        let build = || {
            let mut session = GameSession::new(&data.config, &data, 5_150);
            for member in &mut session.crew {
                member.condition.fatigue = data.config.condition.fatigue_work_threshold + 10;
            }
            session
        };
        let (mut a, mut b) = (build(), build());
        let target = first_target(&data, &a).clone();
        let plan = auto_assign(&a, &data, &target);

        let left = run_job(&mut a, &data, &plan);
        let right = run_job(&mut b, &data, &plan);

        let rolls = |report: &JobReport| -> Vec<i32> {
            report.doors.iter().map(|d| d.result.roll).collect()
        };
        assert_eq!(rolls(&left), rolls(&right));
        assert_eq!(
            left.doors.iter().filter(|d| d.injury.is_some()).count(),
            right.doors.iter().filter(|d| d.injury.is_some()).count()
        );
        assert_eq!(a.budget, b.budget);
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
