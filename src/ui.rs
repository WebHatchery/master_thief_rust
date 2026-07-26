//! The view layer. It reads state and returns intents; it never mutates the
//! campaign. `heist_actions.rs` is the only place an intent becomes a change.

pub mod board;
pub mod chrome;
pub mod crew;
pub mod floorplan;
pub mod hiring;
pub mod planning;
pub mod records;
pub mod results;
pub mod run;
pub mod shop;

use crate::data::GameData;
use crate::game::playback::RunPlayback;
use crate::sim::{JobReport, PlanDraft};
use crate::state::GameSession;
use macroquad::prelude::*;
use macroquad_toolkit::ui::VirtualUi;

pub const LOGICAL_WIDTH: f32 = 1280.0;
pub const LOGICAL_HEIGHT: f32 = 720.0;

/// Which board the fixer is standing at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Crew,
    Board,
    Shop,
    Records,
    /// Reached from a mark, not from the tab bar — it needs a job to plan.
    Planning,
    /// The committed job, resolving door by door.
    Run,
    Results,
}

impl Screen {
    /// The screens the tab bar offers. Planning and the run are deliberately
    /// not among them — each needs a job to exist first.
    pub const TABS: [Screen; 5] = [
        Screen::Crew,
        Screen::Board,
        Screen::Shop,
        Screen::Results,
        Screen::Records,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Screen::Crew => "Crew",
            Screen::Board => "The Board",
            Screen::Shop => "Outfitter",
            Screen::Records => "Records",
            Screen::Planning => "Planning",
            Screen::Run => "The Run",
            Screen::Results => "Last Job",
        }
    }
}

/// Everything the player can ask for. The UI produces these; it never acts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    NewGame,
    Save,
    Load,
    DeleteSave,
    ShowScreen(Screen),
    SelectMember(String),
    SelectTarget(String),
    /// Switch the crew screen's left panel between payroll and applicants.
    ShowHiring(bool),
    HireRecruit(String),
    BuyItem(String),
    EquipItem {
        member_id: String,
        item_id: String,
    },
    UnequipSlot {
        member_id: String,
        slot: crate::model::EquipmentSlot,
    },
    SpendAttribute {
        member_id: String,
        kind: crate::model::AttributeKind,
    },
    SpendSkill {
        member_id: String,
        skill: crate::model::Skill,
    },
    CaseTarget(String),
    /// Open the planning screen on a mark.
    PlanJob(String),
    /// Open one door's candidate list.
    FocusDoor(usize),
    AssignDoor {
        door: usize,
        member_id: String,
    },
    ClearDoor(usize),
    /// Fill the draft with the crew's own best guess.
    AutoFillPlan,
    /// Commit the plan and run the job.
    CommitPlan,
    AbandonPlan,
    /// Stop watching and jump to the end of the run.
    SkipRun,
    /// Leave the run screen for the results.
    FinishRun,
    /// Hand the job to the crew's own judgement and run it (GDD 5.3).
    DelegateJob(String),
    AdvanceWeek,
}

pub struct UiContext<'a> {
    pub data: &'a GameData,
    pub session: &'a GameSession,
    pub screen: Screen,
    pub selected_member: Option<&'a str>,
    pub selected_target: Option<&'a str>,
    /// True while the crew screen is showing applicants rather than payroll.
    pub hiring: bool,
    pub draft: Option<&'a PlanDraft>,
    pub playback: Option<&'a RunPlayback>,
    pub last_report: Option<&'a JobReport>,
    pub save_exists: bool,
    pub ui: &'a VirtualUi,
}

impl UiContext<'_> {
    pub fn mouse(&self) -> Vec2 {
        self.ui.mouse_position()
    }
}

pub fn content_rect() -> Rect {
    Rect::new(18.0, 142.0, LOGICAL_WIDTH - 36.0, 496.0)
}

/// The panel a list of things lives in, on the left of every screen.
pub fn list_rect() -> Rect {
    let content = content_rect();
    Rect::new(content.x, content.y, 372.0, content.h)
}

/// The panel the selected thing is detailed in.
pub fn detail_rect() -> Rect {
    let content = content_rect();
    let list = list_rect();
    Rect::new(
        list.x + list.w + 14.0,
        content.y,
        content.w - list.w - 14.0,
        content.h,
    )
}

pub fn draw_game_ui(ctx: UiContext<'_>) -> Vec<UiAction> {
    let mut actions = Vec::new();

    chrome::draw_header(&ctx);
    chrome::draw_tabs(&ctx, &mut actions);

    match ctx.screen {
        Screen::Crew => crew::draw(&ctx, &mut actions),
        Screen::Board => board::draw(&ctx, &mut actions),
        Screen::Shop => shop::draw(&ctx, &mut actions),
        Screen::Records => records::draw(&ctx, &mut actions),
        Screen::Planning => planning::draw(&ctx, &mut actions),
        Screen::Run => run::draw(&ctx, &mut actions),
        Screen::Results => results::draw(&ctx),
    }

    chrome::draw_footer(&ctx, &mut actions);

    actions
}
