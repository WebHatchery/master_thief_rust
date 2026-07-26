//! The dispatcher. Every intent the UI returns becomes a change here, and
//! nowhere else.

use crate::data::GameData;
use crate::sim::{self, JobReport, PlanDraft};
use crate::state::GameSession;
use crate::ui::{Screen, UiAction};
use macroquad_toolkit::notifications::NotificationManager;
use macroquad_toolkit::ui::format_money;

/// What the player is currently looking at. Not part of the save.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    pub screen: Screen,
    pub member: Option<String>,
    pub target: Option<String>,
    /// The plan under construction, if the fixer is at the planning table.
    pub draft: Option<PlanDraft>,
    pub last_report: Option<JobReport>,
}

/// Persistence work the dispatcher cannot do itself, handed back to `Game`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveCommand {
    NewCampaign,
    Save,
    Load,
    Delete,
}

pub struct Dispatch<'a> {
    pub data: &'a GameData,
    pub session: &'a mut GameSession,
    pub selection: &'a mut Selection,
    pub notifications: &'a mut NotificationManager,
}

pub fn apply(action: UiAction, dispatch: Dispatch<'_>) -> Option<SaveCommand> {
    let Dispatch {
        data,
        session,
        selection,
        notifications,
    } = dispatch;

    match action {
        UiAction::NewGame => return Some(SaveCommand::NewCampaign),
        UiAction::Save => return Some(SaveCommand::Save),
        UiAction::Load => return Some(SaveCommand::Load),
        UiAction::DeleteSave => return Some(SaveCommand::Delete),

        UiAction::ShowScreen(screen) => selection.screen = screen,
        UiAction::SelectMember(id) => selection.member = Some(id),
        UiAction::SelectTarget(id) => selection.target = Some(id),

        UiAction::CaseTarget(id) => case_target(data, session, notifications, &id),

        UiAction::PlanJob(id) => open_plan(data, session, selection, notifications, &id),
        UiAction::FocusDoor(door) => {
            if let Some(draft) = selection.draft.as_mut() {
                draft.focus_on(door);
            }
        }
        UiAction::AssignDoor { door, member_id } => {
            if let Some(draft) = selection.draft.as_mut() {
                draft.assign(door, member_id);
            }
        }
        UiAction::ClearDoor(door) => {
            if let Some(draft) = selection.draft.as_mut() {
                draft.clear(door);
            }
        }
        UiAction::AutoFillPlan => auto_fill_plan(data, session, selection, notifications),
        UiAction::CommitPlan => commit_plan(data, session, selection, notifications),
        UiAction::AbandonPlan => {
            selection.draft = None;
            selection.screen = Screen::Board;
        }

        UiAction::DelegateJob(id) => delegate_job(data, session, selection, notifications, &id),
        UiAction::AdvanceWeek => advance_week(data, session, notifications),
    }

    None
}

/// Spend the week's attention on a mark: its DCs and conditions become visible
/// on the board and, later, on the planning screen.
fn case_target(
    data: &GameData,
    session: &mut GameSession,
    notifications: &mut NotificationManager,
    target_id: &str,
) {
    let cost = data.config.casing_cost;
    let name = data
        .targets
        .get(target_id)
        .map(|target| target.name.clone())
        .unwrap_or_else(|| target_id.to_owned());

    let Some(entry) = session
        .board
        .iter_mut()
        .find(|entry| entry.target_id == target_id)
    else {
        notifications.warning(format!("{} is no longer on the board", name));
        return;
    };

    if entry.cased {
        notifications.info(format!("{} is already cased", name));
        return;
    }

    if session.budget < cost {
        notifications.warning(format!("Casing {} costs {}", name, format_money(cost)));
        return;
    }

    entry.cased = true;
    session.budget -= cost;
    notifications.success(format!("{} cased — every door is now on the file", name));
}

/// Open the planning table on a mark. The draft starts empty: the point of the
/// screen is the choosing.
fn open_plan(
    data: &GameData,
    session: &GameSession,
    selection: &mut Selection,
    notifications: &mut NotificationManager,
    target_id: &str,
) {
    let Some(target) = data.targets.get(target_id) else {
        notifications.warning("That mark is no longer on the board");
        return;
    };

    if session.available_crew().count() == 0 {
        notifications.warning("Nobody on the payroll is fit to work");
        return;
    }

    selection.draft = Some(PlanDraft::new(target, data));
    selection.target = Some(target_id.to_owned());
    selection.screen = Screen::Planning;

    if session
        .board_entry(target_id)
        .is_some_and(|entry| !entry.cased)
    {
        notifications.info(format!(
            "{} is uncased — the crew goes in without the difficulties",
            target.name
        ));
    }
}

fn auto_fill_plan(
    data: &GameData,
    session: &GameSession,
    selection: &mut Selection,
    notifications: &mut NotificationManager,
) {
    let Some(target) = selection
        .draft
        .as_ref()
        .and_then(|draft| data.targets.get(&draft.target_id))
    else {
        return;
    };

    selection.draft = Some(PlanDraft::from_auto(session, data, target));
    notifications.info("The crew picked their own doors - argue with it");
}

/// Commit a hand-made plan. Same engine, same dice, better assignments than
/// delegation if the fixer earned them.
fn commit_plan(
    data: &GameData,
    session: &mut GameSession,
    selection: &mut Selection,
    notifications: &mut NotificationManager,
) {
    let Some(plan) = selection
        .draft
        .as_ref()
        .and_then(|draft| draft.to_job_plan())
    else {
        notifications.warning("Every door needs somebody on it before you commit");
        return;
    };

    let report = sim::run_job(session, data, &plan);
    announce(notifications, &report);

    selection.draft = None;
    selection.last_report = Some(report);
    selection.screen = Screen::Results;
    selection.target = None;
}

fn announce(notifications: &mut NotificationManager, report: &JobReport) {
    if report.success {
        notifications.success(format!(
            "{} - {}/{} doors, {} net",
            report.target_name,
            report.doors_passed(),
            report.doors.len(),
            format_money(report.payout)
        ));
    } else {
        notifications.danger(format!(
            "{} went wrong - {}/{} doors",
            report.target_name,
            report.doors_passed(),
            report.doors.len()
        ));
    }
}

/// Hand the job to the crew and run it through the same engine a hand-made
/// plan would use — with a worse assignment, never a kinder rule (GDD 5.3).
fn delegate_job(
    data: &GameData,
    session: &mut GameSession,
    selection: &mut Selection,
    notifications: &mut NotificationManager,
    target_id: &str,
) {
    let Some(target) = data.targets.get(target_id).cloned() else {
        notifications.warning("That mark is no longer on the board");
        return;
    };

    if session.available_crew().count() == 0 {
        notifications.warning("Nobody on the payroll is fit to work");
        return;
    }

    let plan = sim::auto_assign(session, data, &target);
    let report = sim::run_job(session, data, &plan);
    announce(notifications, &report);

    selection.draft = None;
    selection.last_report = Some(report);
    selection.screen = Screen::Results;
    selection.target = None;
}

fn advance_week(
    data: &GameData,
    session: &mut GameSession,
    notifications: &mut NotificationManager,
) {
    let summary = sim::advance_week(session, data);
    notifications.info(format!(
        "Week {} — {} fatigue shed, {} healed, heat down {}, {} new marks",
        summary.week,
        summary.fatigue_shed,
        summary.injuries_healed,
        summary.heat_shed,
        summary.new_marks
    ));
}

#[cfg(test)]
mod tests {
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
        Dispatch {
            data,
            session,
            selection,
            notifications,
        }
    }

    #[test]
    fn casing_a_mark_costs_the_fee_and_opens_the_file() {
        let (data, mut session) = setup();
        let mut selection = Selection::default();
        let mut notifications = NotificationManager::new();
        let target_id = session.board[0].target_id.clone();
        let budget = session.budget;

        apply(
            UiAction::CaseTarget(target_id.clone()),
            dispatch(&data, &mut session, &mut selection, &mut notifications),
        );

        assert!(session.board_entry(&target_id).unwrap().cased);
        assert_eq!(session.budget, budget - data.config.casing_cost);
    }

    #[test]
    fn casing_twice_does_not_charge_twice() {
        let (data, mut session) = setup();
        let mut selection = Selection::default();
        let mut notifications = NotificationManager::new();
        let target_id = session.board[0].target_id.clone();

        for _ in 0..2 {
            apply(
                UiAction::CaseTarget(target_id.clone()),
                dispatch(&data, &mut session, &mut selection, &mut notifications),
            );
        }

        assert_eq!(
            session.budget,
            data.config.starting_budget - data.config.casing_cost
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

        assert!(!session.board_entry(&target_id).unwrap().cased);
        assert_eq!(session.budget, 0);
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
        assert!(selection.last_report.is_none());
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
        apply(
            UiAction::CommitPlan,
            dispatch(&data, &mut session, &mut selection, &mut notifications),
        );

        let report = selection.last_report.as_ref().expect("the job ran");
        assert_eq!(selection.screen, Screen::Results);
        assert!(selection.draft.is_none());
        assert!(!report.delegated, "a hand-made plan is not delegated");
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
    fn persistence_intents_are_handed_back_rather_than_acted_on() {
        let (data, mut session) = setup();
        let mut selection = Selection::default();
        let mut notifications = NotificationManager::new();

        let command = apply(
            UiAction::Save,
            dispatch(&data, &mut session, &mut selection, &mut notifications),
        );
        assert_eq!(command, Some(SaveCommand::Save));
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
}
