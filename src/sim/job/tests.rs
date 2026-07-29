//! Tests for `job.rs`.
//!
//! Extracted to its own module rather than left inline: the block had grown
//! past the point where the parent file could be scanned around it
//! (CODE_STANDARDS 11.3). `use super::*` still reaches every private item.

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

    let rolls =
        |report: &JobReport| -> Vec<i32> { report.doors.iter().map(|d| d.result.roll).collect() };
    assert_eq!(rolls(&left), rolls(&right));
    assert_eq!(
        left.doors.iter().filter(|d| d.injury.is_some()).count(),
        right.doors.iter().filter(|d| d.injury.is_some()).count()
    );
    assert_eq!(a.budget, b.budget);
}

/// A seed on which a standing order of one actually fires, with the plan
/// that fires it. Scanned rather than hardcoded so re-balancing content
/// cannot quietly turn these tests into assertions about nothing.
fn a_job_that_goes_wrong(data: &GameData) -> (u64, JobPlan) {
    for seed in 0..400u64 {
        let mut session = GameSession::new(&data.config, data, seed);
        let Some(entry) = session.board.first().cloned() else {
            continue;
        };
        let Some(target) = data.targets.get(&entry.target_id).cloned() else {
            continue;
        };
        let mut plan = auto_assign(&session, data, &target);
        if plan.assignments.len() < 3 {
            continue;
        }
        plan.walk_after = Some(1);
        plan.delegated = false;

        if run_job(&mut session, data, &plan).was_called_off() {
            return (seed, plan);
        }
    }
    panic!("four hundred seeds and no job ever went wrong on the first door");
}

#[test]
fn a_standing_order_leaves_doors_unopened() {
    let data = GameData::load().unwrap();
    let (seed, plan) = a_job_that_goes_wrong(&data);
    let mut session = GameSession::new(&data.config, &data, seed);

    let report = run_job(&mut session, &data, &plan);
    let left = report.called_off_with.expect("the order fired");

    assert!(left > 0, "called off with nothing left to call off");
    assert_eq!(report.doors.len() + left, plan.assignments.len());
    assert!(!report.success, "walking out counted as a win");
    assert_eq!(session.tally.jobs_called_off, 1);
}

#[test]
fn walking_out_costs_the_score_and_saves_the_crew() {
    // The bet the standing order is: the same seed, the same plan, the same
    // dice up to the point the order fires. One crew pushes on; one leaves.
    let data = GameData::load().unwrap();
    let (seed, walked_plan) = a_job_that_goes_wrong(&data);
    let pushed_plan = JobPlan {
        walk_after: None,
        ..walked_plan.clone()
    };

    let run = |plan: &JobPlan| {
        let mut session = GameSession::new(&data.config, &data, seed);
        let report = run_job(&mut session, &data, plan);
        let hurt = report.doors.iter().filter(|d| d.injury.is_some()).count();
        (report, session, hurt)
    };

    let (walked, walked_session, walked_hurt) = run(&walked_plan);
    let (pushed, pushed_session, pushed_hurt) = run(&pushed_plan);

    assert!(walked.doors.len() < pushed.doors.len());
    assert!(
        walked.notoriety_gained < pushed.notoriety_gained,
        "leaving early made exactly as much noise as finishing"
    );
    assert!(walked_session.heat < pushed_session.heat);
    assert!(walked_hurt <= pushed_hurt, "walking got more people hurt");
    assert!(
        walked_session
            .scrutiny
            .get(walked.doors[0].result.check.skill)
            <= pushed_session
                .scrutiny
                .get(walked.doors[0].result.check.skill),
        "the city learned as much from a job the crew abandoned"
    );

    // And the price. A job the crew walked out of never pays what finishing
    // it would have — that is the whole cost of the nerve.
    assert!(
        walked.gross < pushed.gross || !pushed.success,
        "walking out cost nothing: {} against {}",
        walked.gross,
        pushed.gross
    );
}

#[test]
fn no_standing_order_runs_exactly_the_job_it_always_did() {
    // The default has to be the old behaviour, door for door and coin for
    // coin, or every balance number in the game just moved.
    let data = GameData::load().unwrap();
    for seed in [11u64, 404, 2_026] {
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
        assert!(
            plan.walk_after.is_none(),
            "delegation gave a standing order"
        );
        assert!(!report.was_called_off());
        assert!(report.doors.len() >= plan.assignments.len());
    }
}

#[test]
fn the_same_seed_and_the_same_order_replay_identically() {
    let data = GameData::load().unwrap();
    let (seed, plan) = a_job_that_goes_wrong(&data);
    let mut a = GameSession::new(&data.config, &data, seed);
    let mut b = GameSession::new(&data.config, &data, seed);

    let left = run_job(&mut a, &data, &plan);
    let right = run_job(&mut b, &data, &plan);

    assert_eq!(left.called_off_with, right.called_off_with);
    let rolls =
        |report: &JobReport| -> Vec<i32> { report.doors.iter().map(|d| d.result.roll).collect() };
    assert_eq!(rolls(&left), rolls(&right));
    assert_eq!(a.budget, b.budget);
    assert_eq!(a.notoriety, b.notoriety);
}

#[test]
fn a_complication_is_answered_by_somebody_who_is_actually_in_the_building() {
    // It used to fall to the first fit name on the whole payroll, whether or
    // not they were on the job. A door in a building can only be opened by
    // somebody standing in it.
    let data = GameData::load().unwrap();
    let mut seen = 0;

    for seed in 0..250u64 {
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
        let on_the_job = plan.crew_on_job();
        let report = run_job(&mut session, &data, &plan);

        for door in report.doors.iter().filter(|door| door.was_complication) {
            seen += 1;
            assert!(
                on_the_job.contains(&door.result.check.member_id),
                "{} answered a complication without being on the job",
                door.result.check.member_name
            );
        }
    }

    assert!(seen > 0, "two hundred and fifty jobs and no complication");
}

#[test]
fn a_complication_goes_to_the_best_hand_present_not_the_first_one() {
    // The reason this matters: a second capable body on the job is *cover*,
    // and cover is a roster decision with a price. If the complication went to
    // whoever sorted first, bringing cover would buy nothing.
    let data = GameData::load().unwrap();
    let complication = data
        .encounters
        .iter()
        .filter(|(_, encounter)| encounter.complication_only)
        .map(|(_, encounter)| encounter.clone())
        .min_by(|a, b| a.id.cmp(&b.id))
        .expect("the content has complications in it");

    let session = GameSession::new(&data.config, &data, 77);
    let target = data
        .targets
        .get(&session.board[0].target_id)
        .unwrap()
        .clone();
    let crew = session.crew_ids();
    let plan = JobPlan {
        target_id: target.id.clone(),
        assignments: Vec::new(),
        delegated: false,
        walk_after: None,
    };

    let chosen = assigned_member(&plan, &complication, &session, &data, &target, &crew)
        .expect("somebody takes it");

    let best = crew
        .iter()
        .filter_map(|id| session.member(id))
        .map(|member| {
            let loadout = session.loadout(member, &data);
            let bonus = build_check(CheckInputs {
                member,
                loadout: &loadout,
                encounter: &complication,
                tuning: &data.config.condition,
                extra: &situational_modifiers(
                    &data,
                    &target,
                    &complication,
                    &session,
                    &member.id,
                    &crew,
                ),
            })
            .bonus();
            (bonus, member.id.clone())
        })
        .max()
        .expect("a crew");

    assert_eq!(chosen, best.1);
}

#[test]
fn a_door_that_can_rewrite_the_run_says_so_once_it_is_on_the_file() {
    // GDD 12, open question 3: adding an encounter is only unfair while it
    // arrives unannounced. Every door capable of it has to carry the warning,
    // and every door not capable of it must not — or the warning means nothing.
    let data = GameData::load().unwrap();
    let mut warned = 0;
    let mut silent = 0;

    for (_, encounter) in data.encounters.iter() {
        let telegraphs = encounter.run_effect_telegraphs();
        assert_eq!(
            encounter.can_rewrite_the_run(),
            !telegraphs.is_empty(),
            "{} disagrees with itself about rewriting the run",
            encounter.id
        );
        if telegraphs.is_empty() {
            silent += 1;
        } else {
            warned += 1;
            assert!(telegraphs.len() <= 2);
            for line in telegraphs {
                assert!(!line.is_empty());
            }
        }
    }

    assert!(warned > 0, "no door in the game can rewrite a run");
    assert!(silent > warned, "every door became a special case");
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
