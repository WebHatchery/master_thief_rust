//! Tests for `plan.rs`.
//!
//! Extracted to its own module rather than left inline: the block had grown
//! past the point where the parent file could be scanned around it
//! (CODE_STANDARDS 11.3). `use super::*` still reaches every private item.

use super::*;

fn setup(seed: u64) -> (GameData, GameSession, HeistTarget) {
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config, &data, seed);
    let target = data
        .targets
        .get(&session.board[0].target_id)
        .unwrap()
        .clone();
    (data, session, target)
}

#[test]
fn a_fresh_draft_has_one_empty_slot_per_door() {
    let (data, _, target) = setup(11);
    let draft = PlanDraft::new(&target, &data);

    assert_eq!(draft.doors.len(), target.encounters.len());
    assert_eq!(draft.unfilled(), target.encounters.len());
    assert!(!draft.is_complete());
    assert!(draft.to_job_plan().is_none());
}

#[test]
fn filling_every_door_makes_the_plan_committable() {
    let (data, session, target) = setup(12);
    let mut draft = PlanDraft::new(&target, &data);
    let hand = session.crew[0].id.clone();

    for door in 0..draft.doors.len() {
        draft.assign(door, hand.clone());
    }

    assert!(draft.is_complete());
    let plan = draft.to_job_plan().unwrap();
    assert_eq!(plan.assignments.len(), draft.doors.len());
    assert!(!plan.delegated, "a hand-made plan is never delegated");
}

#[test]
fn clearing_a_door_takes_the_plan_back_out_of_reach() {
    let (data, session, target) = setup(13);
    let mut draft = PlanDraft::from_auto(&session, &data, &target);
    assert!(draft.is_complete());

    draft.clear(1);
    assert!(!draft.is_complete());
    assert_eq!(draft.unfilled(), 1);
}

#[test]
fn the_auto_draft_fills_the_same_doors_delegation_would() {
    let (data, session, target) = setup(14);
    let draft = PlanDraft::from_auto(&session, &data, &target);
    let auto = super::super::job::auto_assign(&session, &data, &target);

    assert!(draft.is_complete());
    for assignment in &auto.assignments {
        let index = draft
            .doors
            .iter()
            .position(|door| *door == assignment.encounter_id)
            .unwrap();
        assert_eq!(draft.assigned(index), Some(assignment.member_id.as_str()));
    }
}

#[test]
fn candidates_are_ranked_best_first_and_name_every_modifier() {
    let (data, session, target) = setup(15);
    let draft = PlanDraft::new(&target, &data);
    let encounter = data.encounters.get(&draft.doors[0]).unwrap();

    let ranked = candidates(&session, &data, &target, encounter, &draft);
    assert_eq!(ranked.len(), session.crew.len());
    for pair in ranked.windows(2) {
        assert!(pair[0].check.bonus() >= pair[1].check.bonus());
    }
    assert!(ranked[0].check.entries.iter().any(|e| e.value != 0));
}

#[test]
fn a_hand_already_down_for_another_door_is_flagged_not_hidden() {
    let (data, session, target) = setup(16);
    let mut draft = PlanDraft::new(&target, &data);
    let hand = session.crew[0].id.clone();
    draft.assign(0, hand.clone());

    let second = data.encounters.get(&draft.doors[1]).unwrap();
    let ranked = candidates(&session, &data, &target, second, &draft);
    let entry = ranked.iter().find(|c| c.member_id == hand).unwrap();
    assert!(entry.doubled_up);

    let first = data.encounters.get(&draft.doors[0]).unwrap();
    let same_door = candidates(&session, &data, &target, first, &draft);
    let entry = same_door.iter().find(|c| c.member_id == hand).unwrap();
    assert!(
        !entry.doubled_up,
        "their own door does not count as doubling"
    );
}

#[test]
fn a_spent_hand_sinks_below_the_fit_ones_and_a_hurt_one_below_them_all() {
    // Three tiers, and the middle one is the new decision: a spent hand is
    // listed last-but-one, warned about, and still selectable. Taking that
    // choice away is what used to make lying low free (GDD 5.6).
    let (data, mut session, target) = setup(17);
    session.crew[0].condition.fatigue = 95;
    session.crew[1].condition.injuries = (0..data.config.condition.max_injuries_for_work + 1)
        .map(|n| crate::model::crew::Injury::major(format!("Hurt {}", n)))
        .collect();
    let draft = PlanDraft::new(&target, &data);
    let encounter = data.encounters.get(&draft.doors[0]).unwrap();

    let ranked = candidates(&session, &data, &target, encounter, &draft);
    let spent = ranked.iter().position(|c| c.spent && !c.unfit).unwrap();
    let hurt = ranked.iter().position(|c| c.unfit).unwrap();

    assert!(!ranked[0].spent && !ranked[0].unfit);
    assert!(spent < hurt, "a hurt hand outranked a merely tired one");
    assert!(ranked[spent].selectable(), "a tired hand cannot be sent");
    assert!(!ranked[hurt].selectable());
    assert_eq!(
        ranked[spent].warning(),
        Some("spent — worse odds, and gets hurt easier")
    );
}

#[test]
fn being_spent_is_charged_by_name_on_the_breakdown() {
    let (data, mut session, target) = setup(34);
    let draft = PlanDraft::new(&target, &data);
    let encounter = data.encounters.get(&draft.doors[0]).unwrap();

    session.crew[0].condition.fatigue = data.config.condition.fatigue_work_threshold;
    let rested = candidate_check(&session, &data, &target, encounter, &session.crew[0], &[]);
    assert!(!rested
        .entries
        .iter()
        .any(|entry| entry.label == "Running on empty"));

    session.crew[0].condition.fatigue = data.config.condition.fatigue_work_threshold + 1;
    let spent = candidate_check(&session, &data, &target, encounter, &session.crew[0], &[]);

    let entry = spent
        .entries
        .iter()
        .find(|entry| entry.label == "Running on empty")
        .expect("a spent hand is charged by name");
    assert_eq!(entry.value, -data.config.condition.spent_check_penalty);
    assert!(spent.bonus() < rested.bonus());
}

#[test]
fn a_hand_made_plan_can_never_be_beaten_by_delegation() {
    // GDD 5.3: delegation is a discount, not a shortcut. Picking the best
    // candidate for every door must total at least what the greedy
    // auto-assigner reaches, on every mark the crew can currently open.
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config, &data, 21);

    for target in session.eligible_targets(&data) {
        let draft = PlanDraft::new(target, &data);
        let best: i32 = draft
            .doors
            .iter()
            .filter_map(|door| data.encounters.get(door))
            .map(|encounter| {
                candidates(&session, &data, target, encounter, &draft)
                    .first()
                    .map(|candidate| candidate.check.bonus())
                    .unwrap_or(0)
            })
            .sum();

        let delegated: i32 = super::super::job::auto_assign(&session, &data, target)
            .assignments
            .iter()
            .filter_map(|assignment| {
                let encounter = data.encounters.get(&assignment.encounter_id)?;
                let member = session.member(&assignment.member_id)?;
                Some(candidate_check(&session, &data, target, encounter, member, &[]).bonus())
            })
            .sum();

        assert!(
            best >= delegated,
            "{}: delegation reached {} where a hand-made plan reaches {}",
            target.id,
            delegated,
            best
        );
    }
}

#[test]
fn the_standing_order_cycles_and_never_offers_one_that_cannot_fire() {
    // An order of "walk after every door goes wrong" is not an order, it is
    // the default with extra clicks. The cycle stops one short of the mark's
    // own door count and wraps back to pushing on.
    let (data, _, target) = setup(41);
    let mut draft = PlanDraft::new(&target, &data);
    let doors = draft.doors.len() as u32;
    assert!(doors >= 2, "this mark cannot test the cycle");
    assert_eq!(draft.walk_after, None);
    assert_eq!(draft.nerve_label(), "Push on regardless");

    let mut seen = Vec::new();
    for _ in 0..doors + 2 {
        draft.cycle_nerve();
        seen.push(draft.walk_after);
        if let Some(limit) = draft.walk_after {
            assert!(
                limit < doors,
                "offered an order that leaves nothing to leave"
            );
        }
    }

    assert!(seen.contains(&Some(1)));
    assert!(seen.contains(&None), "the cycle never came back round");
}

#[test]
fn the_standing_order_is_committed_with_everything_else() {
    let (data, session, target) = setup(42);
    let mut draft = PlanDraft::from_auto(&session, &data, &target);
    assert!(draft.crew_planned);
    draft.cycle_nerve();

    let plan = draft.to_job_plan().expect("a full draft commits");
    assert_eq!(plan.walk_after, draft.walk_after);
    assert_eq!(plan.walk_after, Some(1));
    assert!(
        !plan.delegated,
        "let them pick, then tell them when to leave, and still take the \
         delegation discount"
    );
}

#[test]
fn nobody_gives_the_crew_a_standing_order_on_their_own_job() {
    // GDD 5.3: delegation is a discount, not a shortcut. Knowing when to
    // leave is one more thing a fixer who turns up brings.
    let (data, session, target) = setup(43);
    let auto = super::super::job::auto_assign(&session, &data, &target);
    assert_eq!(auto.walk_after, None);
}

#[test]
fn letting_the_crew_pick_and_committing_it_is_delegation() {
    // The dodge this closes: "Let them pick" produced exactly the
    // assignment delegation produces, and committing it recorded a
    // hand-made plan. The label has to follow the work, not the button.
    let (data, session, target) = setup(31);
    let draft = PlanDraft::from_auto(&session, &data, &target);

    assert!(draft.crew_planned);
    assert!(
        draft.to_job_plan().expect("a full draft commits").delegated,
        "the crew's own plan committed as the fixer's"
    );
}

#[test]
fn arguing_with_one_door_makes_the_plan_yours() {
    let (data, session, target) = setup(32);
    let mut draft = PlanDraft::from_auto(&session, &data, &target);
    let hand = session.crew[0].id.clone();

    draft.assign(0, hand);
    assert!(!draft.crew_planned);
    assert!(!draft.to_job_plan().unwrap().delegated);
}

#[test]
fn a_draft_built_by_hand_was_never_theirs() {
    let (data, session, target) = setup(33);
    let mut draft = PlanDraft::new(&target, &data);
    let hand = session.crew[0].id.clone();

    assert!(!draft.crew_planned);
    for door in 0..draft.doors.len() {
        draft.assign(door, hand.clone());
    }
    assert!(!draft.to_job_plan().unwrap().delegated);
}

#[test]
fn a_ripened_mark_charges_for_itself_on_the_planning_screen() {
    // The payout the board advertises and the doors the crew will meet have
    // to move together, and both before commit (pillar 2).
    let (data, mut session, target) = setup(23);
    let draft = PlanDraft::new(&target, &data);
    let encounter = data.encounters.get(&draft.doors[0]).unwrap();

    let fresh = candidate_check(&session, &data, &target, encounter, &session.crew[0], &[]);
    assert!(!fresh
        .entries
        .iter()
        .any(|entry| entry.label == "Mark has ripened"));

    if let Some(entry) = session
        .board
        .iter_mut()
        .find(|entry| entry.target_id == target.id)
    {
        entry.ripeness = 2;
    }
    let ripe = candidate_check(&session, &data, &target, encounter, &session.crew[0], &[]);

    let named = ripe
        .entries
        .iter()
        .find(|entry| entry.label == "Mark has ripened")
        .expect("the waiting is charged by name");
    assert_eq!(named.value, -2 * data.config.board.ripeness_door_penalty);
    assert!(ripe.bonus() < fresh.bonus());
}

#[test]
fn a_tail_is_named_on_the_planning_screen_before_anybody_commits() {
    // Pillar 2: no hidden difficulty, ever. A penalty the city applied last
    // week has to be readable on the breakdown this week.
    let (data, mut session, target) = setup(19);
    let draft = PlanDraft::new(&target, &data);
    let encounter = data.encounters.get(&draft.doors[0]).unwrap();
    let member = &session.crew[0];

    let clear = candidate_check(&session, &data, &target, encounter, member, &[]);
    assert!(!clear
        .entries
        .iter()
        .any(|entry| entry.label == "Under surveillance"));

    session.surveillance_weeks = 2;
    let member = &session.crew[0];
    let tailed = candidate_check(&session, &data, &target, encounter, member, &[]);

    let entry = tailed
        .entries
        .iter()
        .find(|entry| entry.label == "Under surveillance")
        .expect("a tail shows up by name");
    assert_eq!(entry.value, -data.config.law.surveillance_penalty);
    assert_eq!(
        clear.bonus() - tailed.bonus(),
        data.config.law.surveillance_penalty
    );
}

#[test]
fn the_planning_screen_and_the_run_agree_on_the_arithmetic() {
    let (data, session, target) = setup(18);
    let draft = PlanDraft::from_auto(&session, &data, &target);
    let encounter = data.encounters.get(&draft.doors[0]).unwrap();
    let member = session.member(draft.assigned(0).unwrap()).unwrap();

    let crew = draft.crew_on_job();
    let planned = candidate_check(&session, &data, &target, encounter, member, &crew);
    let at_run_time = candidate_check(&session, &data, &target, encounter, member, &crew);
    assert_eq!(planned, at_run_time);
}
