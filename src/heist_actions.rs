//! The dispatcher. Every intent the UI returns becomes a change here, and
//! nowhere else.

use crate::data::GameData;
use crate::sim::{self, JobReport};
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

    if report.success {
        notifications.success(format!(
            "{} — {}/{} doors, {} net",
            report.target_name,
            report.doors_passed(),
            report.doors.len(),
            format_money(report.payout)
        ));
    } else {
        notifications.danger(format!(
            "{} went wrong — {}/{} doors",
            report.target_name,
            report.doors_passed(),
            report.doors.len()
        ));
    }

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
