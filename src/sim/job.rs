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
    /// How many doors can go wrong before the crew are told to leave. `None` is
    /// the old behaviour and still the default: push on whatever happens.
    ///
    /// Decided *before* the dice, not during. That is the point rather than a
    /// limitation — the plan is the game (GDD 1), and a fixer who could call it
    /// off after seeing the roll would be playing a different one. It also
    /// keeps the run a replay of a resolved report rather than a second roll.
    pub walk_after: Option<u32>,
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
    /// Set when the crew walked out on the fixer's standing order, carrying the
    /// number of doors that were still standing when they did.
    pub called_off_with: Option<usize>,
    pub delegated: bool,
}

impl JobReport {
    /// Did the crew leave doors unopened because they were told to?
    pub fn was_called_off(&self) -> bool {
        self.called_off_with.is_some()
    }

    pub fn doors_passed(&self) -> usize {
        self.doors
            .iter()
            .filter(|door| door.result.passed())
            .count()
    }

    /// Doors the job had, including the ones nobody opened because the crew
    /// were told to leave. Scoring a walked job against what it attempted would
    /// read two-of-three-and-out as a clean sweep.
    pub fn doors_total(&self) -> usize {
        self.doors.len() + self.called_off_with.unwrap_or(0)
    }

    pub fn success_rate(&self) -> f32 {
        let total = self.doors_total();
        if total == 0 {
            0.0
        } else {
            self.doors_passed() as f32 / total as f32
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
        // Nobody told them when to leave, so they do not. Delegation is a
        // discount, not a shortcut: the standing order to walk is one more
        // thing a fixer who turns up gets and one the crew are not given
        // (GDD 5.3).
        walk_after: None,
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
    let mut gone_wrong = 0u32;
    let mut called_off_with = None;

    while index < queue.len() {
        // The standing order, checked between doors rather than during one: the
        // crew finish what they are holding and then leave.
        if plan
            .walk_after
            .is_some_and(|limit| limit > 0 && gone_wrong >= limit)
        {
            called_off_with = Some(queue.len() - index);
            break;
        }

        let (encounter_id, was_complication) = queue[index].clone();
        index += 1;

        let Some(encounter) = data.encounters.get(&encounter_id).cloned() else {
            continue;
        };
        let Some(member_id) =
            assigned_member(plan, &encounter, session, data, &target, &crew_on_job)
        else {
            continue;
        };

        let mut outcome =
            resolve_door(session, data, &target, &encounter, &member_id, &crew_on_job);
        // Read off the queue rather than off the encounter: it is the run that
        // knows whether this door was in the plan or arrived during it.
        outcome.was_complication = was_complication;

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

        if !outcome.result.passed() {
            gone_wrong += 1;
        }

        record_chemistry(
            session,
            &outcome.result.check.member_id,
            &crew_on_job,
            outcome.result.outcome,
            &data.trait_rates,
        );
        doors.push(outcome);
    }

    settle(
        session,
        data,
        &target,
        plan,
        doors,
        delegation_misses,
        called_off_with,
    )
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

/// Who opens this door. Assigned doors go to the hand the fixer put on them;
/// a complication has no assignment of its own and falls to whoever is best
/// placed to deal with it **out of the people already in the building**.
///
/// It used to fall to the first fit name on the whole payroll, which was both a
/// fiction break — somebody who was not on the job answering a door in it — and
/// arbitrary, since roster order is not a measure of anything. Picking the best
/// hand present is what makes a complication a hazard the fixer can staff
/// against rather than a dice roll on the roster: a second capable body on the
/// job is cover, and cover is a roster decision with a price (GDD 12, q3).
fn assigned_member(
    plan: &JobPlan,
    encounter: &Encounter,
    session: &GameSession,
    data: &GameData,
    target: &HeistTarget,
    crew_on_job: &[String],
) -> Option<String> {
    if let Some(assignment) = plan
        .assignments
        .iter()
        .find(|assignment| assignment.encounter_id == encounter.id)
    {
        return Some(assignment.member_id.clone());
    }

    let best = crew_on_job
        .iter()
        .filter_map(|id| session.member(id))
        .filter(|member| data.config.condition.can_work(&member.condition))
        .max_by_key(|member| {
            let loadout = session.loadout(member, data);
            build_check(CheckInputs {
                member,
                loadout: &loadout,
                encounter,
                tuning: &data.config.condition,
                extra: &situational_modifiers(
                    data,
                    target,
                    encounter,
                    session,
                    &member.id,
                    crew_on_job,
                ),
            })
            .bonus()
        })
        .map(|member| member.id.clone());

    // Only if the job somehow has nobody in it at all.
    best.or_else(|| {
        session
            .available_crew(&data.config.condition)
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

    if spent {
        session.tally.doors_worked_spent += 1;
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
        // Overwritten by the run loop, which is the only thing that knows
        // whether this door was planned or arrived mid-job.
        was_complication: false,
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
mod tests;
