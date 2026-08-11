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
        // Drilling a hand costs an hour of the same weekly attention scouting
        // spends, so the refusal has to say which of the two ran out.
        UiAction::SpendAttribute { member_id, kind } => {
            if session.spend_attribute_point(&data.config, &member_id, kind) {
                notifications.info(format!("{} raised", kind.short_label()));
            } else if session.attention_left_this_week(&data.config) == 0 {
                notifications.warning("The crew has no time left for training this week");
            }
        }
        UiAction::SpendSkill { member_id, skill } => {
            if session.spend_skill_point(&data.config, &member_id, skill) {
                notifications.info(format!("{} trained", skill.label()));
            } else if session.attention_left_this_week(&data.config) == 0 {
                notifications.warning("The crew has no time left for training this week");
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
        UiAction::CycleNerve => {
            if let Some(draft) = selection.draft.as_mut() {
                draft.cycle_nerve();
            }
        }
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
mod tests;
