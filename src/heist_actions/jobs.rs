//! The half of the dispatcher that runs the week: casing a mark, building and
//! committing a plan, handing one to the crew, and turning the week over.
//!
//! Split out of `heist_actions.rs` on its own responsibility — routing every
//! intent and carrying out the ones that move the campaign are two jobs, and
//! the file was over the line carrying both. The rule that matters is unchanged
//! and holds across both halves: an intent becomes a change here and nowhere
//! else.

use super::{GameCommand, Selection};
use crate::data::GameData;
use crate::sim::{self, JobReport, PlanDraft};
use crate::state::GameSession;
use crate::ui::Screen;
use macroquad_toolkit::notifications::NotificationManager;
use macroquad_toolkit::ui::format_money;

/// Put one more of a mark's doors on the file. Casing is bought a door at a
/// time against two budgets — cash, and the week's attention — so scouting one
/// building deeply is scouting every other one not at all
/// (GDD 12, open question 2).
pub(super) fn case_target(
    data: &GameData,
    session: &mut GameSession,
    notifications: &mut NotificationManager,
    target_id: &str,
) {
    let Some(target) = data.targets.get(target_id) else {
        notifications.warning("That mark is no longer on the board");
        return;
    };
    let name = target.name.clone();
    let doors = target.encounters.len();
    let left = session.attention_left_this_week(&data.config);

    let Some(entry) = session.board_entry(target_id) else {
        notifications.warning(format!("{} is no longer on the board", name));
        return;
    };

    if entry.is_fully_cased(doors) {
        notifications.info(format!("{} is on the file, door to door", name));
        return;
    }
    if left == 0 {
        notifications
            .warning("The crew has no hours left this week — scouting and training share them");
        return;
    }

    let cost = entry.next_casing_cost(&data.config);
    if session.budget < cost {
        notifications.warning(format!(
            "The next door of {} costs {}",
            name,
            format_money(cost)
        ));
        return;
    }

    session.budget -= cost;
    session.attention_spent_this_week += 1;
    let Some(entry) = session
        .board
        .iter_mut()
        .find(|entry| entry.target_id == target_id)
    else {
        return;
    };
    entry.casing += 1;
    let known = entry.casing as usize;

    if known >= doors {
        notifications.success(format!("{} is on the file, door to door", name));
    } else {
        notifications.success(format!(
            "{} — {}/{} doors on the file, {} more looks this week",
            name,
            known,
            doors,
            left - 1
        ));
    }
}

/// Open the planning table on a mark. The draft starts empty: the point of the
/// screen is the choosing.
pub(super) fn open_plan(
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

    if session.is_retired() {
        notifications.info("The outfit is out. Start a new campaign to work again");
        return;
    }
    if session.available_crew(&data.config.condition).count() == 0 {
        notifications.warning("Everybody on the payroll is too hurt to work");
        return;
    }

    selection.draft = Some(PlanDraft::new(target, data));
    selection.target = Some(target_id.to_owned());
    selection.screen = Screen::Planning;

    let doors = target.encounters.len();
    if let Some(entry) = session.board_entry(target_id) {
        if entry.is_blind() {
            notifications.info(format!(
                "{} is unscouted — the crew goes in without a single difficulty",
                target.name
            ));
        } else if !entry.is_fully_cased(doors) {
            notifications.info(format!(
                "{} — {} of {} doors on the file; the rest is guesswork",
                target.name, entry.casing, doors
            ));
        }
    }
}

pub(super) fn auto_fill_plan(
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
    notifications.info("The crew picked their own doors — argue with it");
}

/// Commit a hand-made plan. Same engine, same dice, better assignments than
/// delegation if the fixer earned them.
pub(super) fn commit_plan(
    data: &GameData,
    session: &mut GameSession,
    selection: &mut Selection,
    notifications: &mut NotificationManager,
) -> Option<GameCommand> {
    let Some(plan) = selection
        .draft
        .as_ref()
        .and_then(|draft| draft.to_job_plan())
    else {
        notifications.warning("Every door needs somebody on it before you commit");
        return None;
    };

    // The dice are cast here, all of them, from the run's seeded RNG. What
    // follows on the run screen is a replay, not a second roll.
    let report = sim::run_job(session, data, &plan);
    announce(notifications, &report);
    check_awards(data, session, notifications);

    selection.draft = None;
    selection.target = None;
    Some(GameCommand::StartRun(Box::new(report)))
}

/// Check the achievement list after anything that could have earned something,
/// and say so when it has.
pub(super) fn check_awards(
    data: &GameData,
    session: &mut GameSession,
    notifications: &mut NotificationManager,
) {
    for name in sim::award(session, &data.config, &data.awards) {
        notifications.success(format!("Achievement: {}", name));
    }
}

fn announce(notifications: &mut NotificationManager, report: &JobReport) {
    if let Some(left) = report.called_off_with {
        notifications.warning(format!(
            "{} — pulled out with {} doors to go, {} banked",
            report.target_name,
            left,
            format_money(report.payout)
        ));
        return;
    }
    if report.success {
        notifications.success(format!(
            "{} - {}/{} doors, {} net",
            report.target_name,
            report.doors_passed(),
            report.doors_total(),
            format_money(report.payout)
        ));
        if !report.loot.is_empty() {
            notifications.info(format!("{} carried out as well", report.loot.len()));
        }
    } else {
        notifications.danger(format!(
            "{} went wrong - {}/{} doors",
            report.target_name,
            report.doors_passed(),
            report.doors_total()
        ));
    }
}

/// Hand the job to the crew and run it through the same engine a hand-made
/// plan would use — with a worse assignment, never a kinder rule (GDD 5.3).
pub(super) fn delegate_job(
    data: &GameData,
    session: &mut GameSession,
    selection: &mut Selection,
    notifications: &mut NotificationManager,
    target_id: &str,
) -> Option<GameCommand> {
    let Some(target) = data.targets.get(target_id).cloned() else {
        notifications.warning("That mark is no longer on the board");
        return None;
    };

    if session.is_retired() {
        notifications.info("The outfit is out. Start a new campaign to work again");
        return None;
    }
    if session.available_crew(&data.config.condition).count() == 0 {
        notifications.warning("Everybody on the payroll is too hurt to work");
        return None;
    }

    let plan = sim::auto_assign(session, data, &target);
    // The planning screen warns per candidate before anybody commits, and a
    // delegated job skips it entirely. Somebody still has to say it: handing
    // the work to a crew who should be resting is a decision, and the fixer
    // making it without looking is exactly who this is for (pillar 2).
    let spent: Vec<String> = plan
        .crew_on_job()
        .iter()
        .filter_map(|id| session.member(id))
        .filter(|member| data.config.condition.is_spent(member.condition.fatigue))
        .map(|member| member.name.clone())
        .collect();
    if !spent.is_empty() {
        notifications.warning(format!("{} went out on empty", spent.join(", ")));
    }

    let report = sim::run_job(session, data, &plan);
    announce(notifications, &report);
    check_awards(data, session, notifications);

    selection.draft = None;
    selection.target = None;
    Some(GameCommand::StartRun(Box::new(report)))
}

/// Turn the week over and tell the player everything it cost them. The ledger
/// line always shows; the alerts only exist when the week did something.
pub(super) fn advance_week(
    data: &GameData,
    session: &mut GameSession,
    notifications: &mut NotificationManager,
) {
    if session.is_retired() {
        notifications.info("The outfit is out. Start a new campaign to work again");
        return;
    }
    let summary = sim::advance_week(session, data);
    check_awards(data, session, notifications);

    if summary.payroll.was_short() {
        notifications.danger(summary.ledger_line());
    } else {
        notifications.info(summary.ledger_line());
    }

    for warning in summary.warnings() {
        notifications.warning(warning);
    }
    for note in summary.notes() {
        notifications.info(note);
    }
}
