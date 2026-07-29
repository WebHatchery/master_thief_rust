//! The dispatcher. Every intent the UI returns becomes a change here, and
//! nowhere else.

mod jobs;

use crate::data::GameData;
use crate::prefs::Preferences;
use crate::sim::{self, JobReport, PlanDraft};
use crate::state::GameSession;
use crate::ui::{CrewTab, Screen, UiAction};
use jobs::{
    advance_week, auto_fill_plan, case_target, check_awards, commit_plan, delegate_job, open_plan,
};
use macroquad_toolkit::notifications::NotificationManager;

/// What the player is currently looking at. Not part of the save.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    pub screen: Screen,
    pub member: Option<String>,
    pub target: Option<String>,
    /// Which panel the crew screen's left column is showing.
    pub crew_tab: CrewTab,
    pub settings_open: bool,
    /// The plan under construction, if the fixer is at the planning table.
    pub draft: Option<PlanDraft>,
    pub last_report: Option<JobReport>,
}

/// Work the dispatcher cannot do itself — persistence, and anything that needs
/// the frame loop — handed back to `Game`.
#[derive(Debug, Clone)]
pub enum GameCommand {
    NewCampaign,
    Save,
    Load,
    Delete,
    /// A job has been committed; start watching it happen.
    StartRun(Box<JobReport>),
    /// Stop watching and jump to the end.
    SkipRun,
    /// The run is over; move to the results.
    FinishRun,
    /// A preference changed; write it out and follow it.
    SavePreferences,
}

pub struct Dispatch<'a> {
    pub data: &'a GameData,
    pub session: &'a mut GameSession,
    pub selection: &'a mut Selection,
    pub prefs: &'a mut Preferences,
    pub notifications: &'a mut NotificationManager,
}

pub fn apply(action: UiAction, dispatch: Dispatch<'_>) -> Option<GameCommand> {
    let Dispatch {
        data,
        session,
        selection,
        prefs,
        notifications,
    } = dispatch;

    match action {
        UiAction::NewGame => return Some(GameCommand::NewCampaign),
        UiAction::Save => return Some(GameCommand::Save),
        UiAction::Load => return Some(GameCommand::Load),
        UiAction::DeleteSave => return Some(GameCommand::Delete),
        UiAction::SkipRun => return Some(GameCommand::SkipRun),
        UiAction::FinishRun => return Some(GameCommand::FinishRun),

        UiAction::ShowScreen(screen) => selection.screen = screen,
        UiAction::SelectMember(id) => selection.member = Some(id),
        UiAction::SelectTarget(id) => selection.target = Some(id),

        UiAction::CaseTarget(id) => case_target(data, session, notifications, &id),

        UiAction::ShowCrewTab(tab) => selection.crew_tab = tab,

        UiAction::OpenSettings => selection.settings_open = true,
        UiAction::CloseSettings => {
            selection.settings_open = false;
            return Some(GameCommand::SavePreferences);
        }
        UiAction::SetPacing(pacing) => {
            prefs.pacing = pacing;
            return Some(GameCommand::SavePreferences);
        }
        UiAction::SetSound(on) => {
            prefs.sound = on;
            return Some(GameCommand::SavePreferences);
        }
        UiAction::SetVolume(volume) => {
            prefs.volume = volume.clamp(0.0, 1.0);
            return Some(GameCommand::SavePreferences);
        }
        UiAction::ShowHints(shown) => {
            prefs.hints = shown;
            return Some(GameCommand::SavePreferences);
        }
        UiAction::HireRecruit(id) => {
            let hired = session.hire(data, &id);
            if hired.is_ok() {
                session.tally.hires += 1;
            }
            report(
                notifications,
                hired.map(|name| format!("{} is on the payroll", name)),
            );
            check_awards(data, session, notifications);
        }
        UiAction::BuyItem(id) => report(
            notifications,
            session
                .buy(data, &id)
                .map(|name| format!("{} bought", name)),
        ),
        UiAction::EquipItem { member_id, item_id } => report(
            notifications,
            session
                .equip(data, &member_id, &item_id)
                .map(|()| String::new()),
        ),
        UiAction::UnequipSlot { member_id, slot } => session.unequip(&member_id, slot),
        UiAction::SpendAttribute { member_id, kind } => {
            if session.spend_attribute_point(&member_id, kind) {
                notifications.info(format!("{} raised", kind.short_label()));
            }
        }
        UiAction::SpendSkill { member_id, skill } => {
            if session.spend_skill_point(&member_id, skill) {
                notifications.info(format!("{} trained", skill.label()));
            }
        }

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
        UiAction::CommitPlan => return commit_plan(data, session, selection, notifications),
        UiAction::AbandonPlan => {
            selection.draft = None;
            selection.screen = Screen::Board;
        }

        UiAction::DelegateJob(id) => {
            return delegate_job(data, session, selection, notifications, &id)
        }
        UiAction::AdvanceWeek => advance_week(data, session, notifications),

        UiAction::PayBonus(id) => {
            report(notifications, sim::pay_bonus(session, &data.config, &id));
            check_awards(data, session, notifications);
        }
        UiAction::GreasePalms => {
            report(notifications, sim::grease_palms(session, &data.config));
            check_awards(data, session, notifications);
        }
        UiAction::PostBail(id) => {
            report(notifications, sim::post_bail(session, &data.config, &id));
            check_awards(data, session, notifications);
        }
        UiAction::TreatInjuries(id) => {
            report(notifications, sim::treat(session, &data.config, &id));
            check_awards(data, session, notifications);
        }
        UiAction::RefitKit(id) => {
            report(notifications, sim::refit(session, &data.config, &id));
            check_awards(data, session, notifications);
        }
        UiAction::SellItem(id) => {
            report(notifications, sim::sell(session, data, &data.config, &id));
            check_awards(data, session, notifications);
        }
        UiAction::Retire => {
            report(
                notifications,
                sim::retire(session, data, &data.config).map(|line| format!("{}.", line)),
            );
            check_awards(data, session, notifications);
            selection.screen = Screen::Records;
        }
        UiAction::DismissMember(id) => {
            if selection.member.as_deref() == Some(id.as_str()) {
                selection.member = None;
            }
            report(notifications, sim::dismiss(session, &data.config, &id));
            check_awards(data, session, notifications);
        }
    }

    None
}

/// Turn a session result into a notification. An empty message means the change
/// speaks for itself on screen.
fn report(notifications: &mut NotificationManager, outcome: Result<String, String>) {
    match outcome {
        Ok(message) if !message.is_empty() => notifications.success(message),
        Ok(_) => {}
        Err(problem) => notifications.warning(problem),
    }
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
        assert_eq!(session.casing_this_week, 1);
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

        for _ in 0..(data.config.casing_steps_per_week + 3) {
            apply(
                UiAction::CaseTarget(target_id.clone()),
                dispatch(&data, &mut session, &mut selection, &mut notifications),
            );
        }

        let doors = data.targets.get(&target_id).unwrap().encounters.len() as u32;
        let spent = session.casing_this_week;
        assert!(
            spent <= data.config.casing_steps_per_week,
            "{} looks in a week of {}",
            spent,
            data.config.casing_steps_per_week
        );
        assert!(session.board_entry(&target_id).unwrap().casing <= doors);
    }

    #[test]
    fn a_new_week_gives_the_crew_their_eyes_back() {
        let (data, mut session) = setup();
        session.casing_this_week = data.config.casing_steps_per_week;
        assert_eq!(session.casing_left_this_week(&data.config), 0);

        crate::sim::advance_week(&mut session, &data);
        assert_eq!(
            session.casing_left_this_week(&data.config),
            data.config.casing_steps_per_week
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
}
