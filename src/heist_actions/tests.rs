use super::*;

fn setup() -> (GameData, GameSession) {
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config, &data, 1234);
    (data, session)
}

fn dispatch<'a>(
    data: &'a GameData,
    session: &'a mut GameSession,
    selection: &'a mut Selection,
    notifications: &'a mut NotificationManager,
) -> Dispatch<'a> {
    // The tests never assert on preferences, so they share one throwaway.
    Dispatch {
        data,
        session,
        selection,
        prefs: Box::leak(Box::new(Preferences::default())),
        notifications,
    }
}

#[test]
fn casing_buys_one_door_at_a_time() {
    let (data, mut session) = setup();
    let mut selection = Selection::default();
    let mut notifications = NotificationManager::new();
    let target_id = session.board[0].target_id.clone();
    let budget = session.budget;

    apply(
        UiAction::CaseTarget(target_id.clone()),
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );

    let entry = session.board_entry(&target_id).unwrap();
    assert_eq!(entry.casing, 1, "one look bought the whole building");
    assert!(entry.knows_door(0) && !entry.knows_door(1));
    assert_eq!(session.budget, budget - data.config.casing_cost);
    assert_eq!(session.attention_spent_this_week, 1);
}

#[test]
fn each_door_deeper_into_a_building_costs_more_than_the_last() {
    let (data, mut session) = setup();
    let mut selection = Selection::default();
    let mut notifications = NotificationManager::new();
    let target_id = session.board[0].target_id.clone();

    let first = session
        .board_entry(&target_id)
        .unwrap()
        .next_casing_cost(&data.config);
    apply(
        UiAction::CaseTarget(target_id.clone()),
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );
    let second = session
        .board_entry(&target_id)
        .unwrap()
        .next_casing_cost(&data.config);

    assert!(
        second > first,
        "the vault scouted as cheap as the front door"
    );
}

#[test]
fn the_crew_only_has_so_many_looks_in_a_week() {
    // The half of casing money cannot buy. Attention spent on one mark is
    // attention not spent on any other (GDD 12, open question 2).
    let (data, mut session) = setup();
    let mut selection = Selection::default();
    let mut notifications = NotificationManager::new();
    let target_id = session.board[0].target_id.clone();
    session.budget = 10_000_000;

    for _ in 0..(data.config.attention_per_week + 3) {
        apply(
            UiAction::CaseTarget(target_id.clone()),
            dispatch(&data, &mut session, &mut selection, &mut notifications),
        );
    }

    let doors = data.targets.get(&target_id).unwrap().encounters.len() as u32;
    let spent = session.attention_spent_this_week;
    assert!(
        spent <= data.config.attention_per_week,
        "{} looks in a week of {}",
        spent,
        data.config.attention_per_week
    );
    assert!(session.board_entry(&target_id).unwrap().casing <= doors);
}

#[test]
fn drilling_a_hand_spends_the_same_week_scouting_does() {
    // Banked points used to be a button that was always right to press the
    // moment it lit up, which is not a decision. They now compete with the
    // board for the one thing the week rations.
    let (data, mut session) = setup();
    let id = session.crew[0].id.clone();
    session.crew[0].progression.skill_points = 5;
    let before = session.attention_left_this_week(&data.config);
    assert!(before > 0);

    assert!(session.spend_skill_point(&data.config, &id, crate::model::Skill::Stealth));
    assert_eq!(session.attention_left_this_week(&data.config), before - 1);
    assert_eq!(session.member(&id).unwrap().progression.skill_points, 4);
}

#[test]
fn a_week_spent_on_the_board_is_a_week_nobody_gets_trained() {
    let (data, mut session) = setup();
    let id = session.crew[0].id.clone();
    session.crew[0].progression.skill_points = 5;
    session.crew[0].progression.attribute_points = 5;
    session.attention_spent_this_week = data.config.attention_per_week;

    assert!(!session.spend_skill_point(&data.config, &id, crate::model::Skill::Stealth));
    assert!(!session.spend_attribute_point(
        &data.config,
        &id,
        crate::model::AttributeKind::Dexterity
    ));
    assert_eq!(
        session.member(&id).unwrap().progression.skill_points,
        5,
        "a point was spent with no week to spend it in"
    );
}

#[test]
fn points_keep_until_there_is_a_week_to_spare_for_them() {
    // The cost is pacing, not forfeiture: nothing earned is ever lost, it
    // just waits behind whatever else the week wanted.
    let (data, mut session) = setup();
    let id = session.crew[0].id.clone();
    session.crew[0].progression.skill_points = 2;
    session.attention_spent_this_week = data.config.attention_per_week;

    crate::sim::advance_week(&mut session, &data);

    assert_eq!(session.member(&id).unwrap().progression.skill_points, 2);
    assert!(session.spend_skill_point(&data.config, &id, crate::model::Skill::Stealth));
}

#[test]
fn a_new_week_gives_the_crew_their_eyes_back() {
    let (data, mut session) = setup();
    session.attention_spent_this_week = data.config.attention_per_week;
    assert_eq!(session.attention_left_this_week(&data.config), 0);

    crate::sim::advance_week(&mut session, &data);
    assert_eq!(
        session.attention_left_this_week(&data.config),
        data.config.attention_per_week
    );
}

#[test]
fn a_broke_fixer_cases_nothing() {
    let (data, mut session) = setup();
    let mut selection = Selection::default();
    let mut notifications = NotificationManager::new();
    let target_id = session.board[0].target_id.clone();
    session.budget = 0;

    apply(
        UiAction::CaseTarget(target_id.clone()),
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );

    assert!(session.board_entry(&target_id).unwrap().is_blind());
    assert_eq!(session.budget, 0);
}

#[test]
fn treating_a_hurt_hand_buys_the_week_back() {
    let (data, mut session) = setup();
    let mut selection = Selection::default();
    let mut notifications = NotificationManager::new();
    let id = session.crew[0].id.clone();
    session.crew[0]
        .condition
        .injuries
        .push(crate::model::crew::Injury::major("Torn shoulder"));
    session.budget = 5_000_000;
    let budget = session.budget;

    apply(
        UiAction::TreatInjuries(id.clone()),
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );

    assert!(session.member(&id).unwrap().condition.injuries.is_empty());
    assert!(session.budget < budget, "the doctor worked for free");
    assert!(session.tally.injuries_treated > 0);
}

#[test]
fn planning_a_mark_opens_an_empty_draft_at_the_planning_table() {
    let (data, mut session) = setup();
    let mut selection = Selection::default();
    let mut notifications = NotificationManager::new();
    let target_id = session.board[0].target_id.clone();

    apply(
        UiAction::PlanJob(target_id.clone()),
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );

    let draft = selection.draft.as_ref().expect("a draft was opened");
    assert_eq!(selection.screen, Screen::Planning);
    assert_eq!(draft.target_id, target_id);
    assert_eq!(draft.unfilled(), draft.doors.len());
}

#[test]
fn an_incomplete_plan_refuses_to_commit() {
    let (data, mut session) = setup();
    let mut selection = Selection::default();
    let mut notifications = NotificationManager::new();
    let target_id = session.board[0].target_id.clone();
    let budget = session.budget;

    apply(
        UiAction::PlanJob(target_id),
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );
    apply(
        UiAction::CommitPlan,
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );

    assert_eq!(selection.screen, Screen::Planning);
    assert!(
        selection.draft.is_some(),
        "the draft survives a refused commit"
    );
    assert_eq!(session.budget, budget);
}

#[test]
fn a_plan_can_be_built_door_by_door_and_committed() {
    let (data, mut session) = setup();
    let mut selection = Selection::default();
    let mut notifications = NotificationManager::new();
    let target_id = session.board[0].target_id.clone();
    let hand = session.crew[0].id.clone();

    apply(
        UiAction::PlanJob(target_id.clone()),
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );
    let doors = selection.draft.as_ref().unwrap().doors.len();
    for door in 0..doors {
        apply(
            UiAction::FocusDoor(door),
            dispatch(&data, &mut session, &mut selection, &mut notifications),
        );
        apply(
            UiAction::AssignDoor {
                door,
                member_id: hand.clone(),
            },
            dispatch(&data, &mut session, &mut selection, &mut notifications),
        );
    }
    let command = apply(
        UiAction::CommitPlan,
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );

    // Committing resolves the job and hands the report to `Game`, which
    // owns the watching. The dispatcher never opens the run screen itself.
    let Some(GameCommand::StartRun(report)) = command else {
        panic!("committing a full plan must start a run");
    };
    assert!(!report.delegated, "a hand-made plan is not delegated");
    assert!(!report.doors.is_empty());
    assert!(selection.draft.is_none());
    assert!(session.board_entry(&target_id).is_none());
}

#[test]
fn letting_the_crew_pick_fills_every_door_without_committing() {
    let (data, mut session) = setup();
    let mut selection = Selection::default();
    let mut notifications = NotificationManager::new();
    let target_id = session.board[0].target_id.clone();

    apply(
        UiAction::PlanJob(target_id),
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );
    apply(
        UiAction::AutoFillPlan,
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );

    assert!(selection.draft.as_ref().unwrap().is_complete());
    assert_eq!(selection.screen, Screen::Planning);
    assert!(selection.last_report.is_none());
}

#[test]
fn walking_away_from_the_table_leaves_the_mark_untouched() {
    let (data, mut session) = setup();
    let mut selection = Selection::default();
    let mut notifications = NotificationManager::new();
    let target_id = session.board[0].target_id.clone();

    apply(
        UiAction::PlanJob(target_id.clone()),
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );
    apply(
        UiAction::AbandonPlan,
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );

    assert!(selection.draft.is_none());
    assert_eq!(selection.screen, Screen::Board);
    assert!(session.board_entry(&target_id).is_some());
}

#[test]
fn a_retired_outfit_takes_no_more_weeks_and_runs_no_more_jobs() {
    let (data, mut session) = setup();
    let mut selection = Selection::default();
    let mut notifications = NotificationManager::new();
    let target_id = session.board[0].target_id.clone();

    apply(
        UiAction::Retire,
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );
    assert!(session.is_retired());
    let week = session.week;
    let board = session.board.len();

    apply(
        UiAction::AdvanceWeek,
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );
    assert_eq!(session.week, week, "a finished campaign moved on");

    let command = apply(
        UiAction::DelegateJob(target_id),
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );
    assert!(command.is_none(), "a finished campaign started a job");
    assert_eq!(session.board.len(), board);
}

#[test]
fn persistence_intents_are_handed_back_rather_than_acted_on() {
    let (data, mut session) = setup();
    let mut selection = Selection::default();
    let mut notifications = NotificationManager::new();

    let command = apply(
        UiAction::Save,
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );
    assert!(matches!(command, Some(GameCommand::Save)));
}

#[test]
fn selection_intents_only_move_the_view() {
    let (data, mut session) = setup();
    let mut selection = Selection::default();
    let mut notifications = NotificationManager::new();
    let before = serde_json::to_value(&session).unwrap();

    apply(
        UiAction::ShowScreen(Screen::Board),
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );
    apply(
        UiAction::SelectMember("vera_sloan".to_owned()),
        dispatch(&data, &mut session, &mut selection, &mut notifications),
    );

    assert_eq!(selection.screen, Screen::Board);
    assert_eq!(selection.member.as_deref(), Some("vera_sloan"));
    assert_eq!(serde_json::to_value(&session).unwrap(), before);
}
