//! The frame loop: draw the screen, collect intents, hand them to the
//! dispatcher, and own the save slot.

pub mod playback;

use crate::data::{GameConfig, GameData};
use crate::game::playback::RunPlayback;
use crate::heist_actions::{self, Dispatch, GameCommand, Selection};
use crate::rules::Outcome;
use crate::sim;
use crate::state::{migrate_save_value, GameSession, SaveData};
use crate::ui::{self, Screen, UiAction, UiContext};
use macroquad::prelude::*;
use macroquad_toolkit::events::EventBus;
use macroquad_toolkit::fx::FloatingTextLayer;
use macroquad_toolkit::notifications::{
    NotificationAnchor, NotificationManager, NotificationRenderConfig,
};
use macroquad_toolkit::persistence::{
    delete_slot, load_from_slot_with_migration, save_to_slot_with_version, slot_exists,
};
use macroquad_toolkit::prelude::{begin_virtual_ui_frame, dark, end_virtual_ui_frame};
use macroquad_toolkit::rng::random_u64;

pub struct Game {
    data: GameData,
    session: GameSession,
    selection: Selection,
    /// The job currently being watched, if any.
    playback: Option<RunPlayback>,
    /// Crit punctuation, hung over the room it happened in.
    floats: FloatingTextLayer,
    notifications: NotificationManager,
    events: EventBus<UiAction>,
    save_exists: bool,
}

impl Game {
    pub async fn new() -> Self {
        let data = GameData::load().unwrap_or_else(|err| {
            panic!("Master Thief data failed to load: {}", err);
        });

        let mut notifications = NotificationManager::new();
        notifications.info(format!(
            "{} marks on file, {} hands for hire",
            data.targets.len(),
            data.crew_pool.len()
        ));

        let session = GameSession::new(&data.config, &data, new_seed());
        let mut game = Self {
            data,
            session,
            selection: Selection::default(),
            playback: None,
            floats: FloatingTextLayer::new(),
            notifications,
            events: EventBus::new(),
            save_exists: false,
        };
        game.refresh_save_state();
        game
    }

    /// Boot straight into a named screen for the headless capture harness. The
    /// campaign is reseeded to a fixed value so every capture is the same
    /// campaign, and the results scene runs a job so it has something to show.
    pub fn set_capture_scene(&mut self, scene: &str) {
        self.session = GameSession::new(&self.data.config, &self.data, CAPTURE_SEED);
        self.selection = Selection::default();

        self.selection.screen = match scene {
            "board" | "targets" => {
                self.selection.target = self
                    .session
                    .board
                    .first()
                    .map(|entry| entry.target_id.clone());
                Screen::Board
            }
            "shop" | "outfitter" => Screen::Shop,
            "hiring" => {
                self.selection.hiring = true;
                Screen::Crew
            }
            "planning" | "plan" => {
                self.open_capture_plan();
                Screen::Planning
            }
            "run" => {
                self.run_capture_job();
                if let Some(report) = self.selection.last_report.take() {
                    self.playback = Some(RunPlayback::new(report));
                    // Park the capture mid-job: first door read out, second in
                    // the air, so the shot shows the plan lighting up.
                    if let Some(playback) = self.playback.as_mut() {
                        playback.update(4.0, false);
                    }
                }
                Screen::Run
            }
            "results" => {
                self.run_capture_job();
                Screen::Results
            }
            _ => Screen::Crew,
        };
    }

    /// A cased mark with the crew's own picks already in, so the capture shows
    /// difficulties, assignments, and odds rather than an empty draft.
    fn open_capture_plan(&mut self) {
        let Some(entry) = self.session.board.first_mut() else {
            return;
        };
        entry.cased = true;
        let target_id = entry.target_id.clone();

        let Some(target) = self.data.targets.get(&target_id).cloned() else {
            return;
        };
        let mut draft = sim::PlanDraft::from_auto(&self.session, &self.data, &target);
        draft.clear(draft.doors.len().saturating_sub(1));
        draft.focus_on(draft.doors.len().saturating_sub(1));

        self.selection.target = Some(target_id);
        self.selection.draft = Some(draft);
    }

    fn run_capture_job(&mut self) {
        let Some(entry) = self.session.board.first().cloned() else {
            return;
        };
        let Some(target) = self.data.targets.get(&entry.target_id).cloned() else {
            return;
        };
        let plan = sim::auto_assign(&self.session, &self.data, &target);
        self.selection.last_report = Some(sim::run_job(&mut self.session, &self.data, &plan));
    }

    pub fn update(&mut self, dt: f32) {
        self.notifications.update(dt);
        self.floats.update(dt);
        self.advance_run(dt);

        if is_key_pressed(KeyCode::Tab) {
            let next = match self.selection.screen {
                Screen::Crew => Screen::Board,
                Screen::Board => Screen::Shop,
                Screen::Shop => Screen::Results,
                // Tab never walks out of a plan under construction, and never
                // out of a run in progress.
                Screen::Planning => Screen::Planning,
                Screen::Run => Screen::Run,
                Screen::Results => Screen::Crew,
            };
            self.events.push(UiAction::ShowScreen(next));
        }
        if is_key_pressed(KeyCode::S) {
            self.events.push(UiAction::Save);
        }
        if is_key_pressed(KeyCode::L) && self.save_exists {
            self.events.push(UiAction::Load);
        }

        let actions: Vec<UiAction> = self.events.drain().collect();
        for action in actions {
            self.apply_action(action);
        }
    }

    pub fn draw(&mut self) {
        clear_background(dark::BACKGROUND);

        let virtual_ui = begin_virtual_ui_frame(ui::LOGICAL_WIDTH, ui::LOGICAL_HEIGHT);
        let actions = ui::draw_game_ui(UiContext {
            data: &self.data,
            session: &self.session,
            screen: self.selection.screen,
            selected_member: self.selection.member.as_deref(),
            selected_target: self.selection.target.as_deref(),
            hiring: self.selection.hiring,
            draft: self.selection.draft.as_ref(),
            playback: self.playback.as_ref(),
            last_report: self.selection.last_report.as_ref(),
            save_exists: self.save_exists,
            ui: &virtual_ui,
        });
        self.floats.draw();
        end_virtual_ui_frame();

        for action in actions {
            self.events.push(action);
        }

        self.notifications
            .draw_with_config(&NotificationRenderConfig {
                anchor: NotificationAnchor::BottomRight,
                ..Default::default()
            });
    }

    fn apply_action(&mut self, action: UiAction) {
        let command = heist_actions::apply(
            action,
            Dispatch {
                data: &self.data,
                session: &mut self.session,
                selection: &mut self.selection,
                notifications: &mut self.notifications,
            },
        );

        match command {
            Some(GameCommand::NewCampaign) => self.new_campaign(),
            Some(GameCommand::Save) => self.save_game(),
            Some(GameCommand::Load) => self.load_game(),
            Some(GameCommand::Delete) => self.delete_save(),
            Some(GameCommand::StartRun(report)) => self.start_run(*report),
            Some(GameCommand::SkipRun) => {
                if let Some(playback) = self.playback.as_mut() {
                    playback.skip_to_end();
                }
            }
            Some(GameCommand::FinishRun) => self.finish_run(),
            None => {}
        }
    }

    fn start_run(&mut self, report: crate::sim::JobReport) {
        self.floats.clear();
        self.playback = Some(RunPlayback::new(report));
        self.selection.screen = Screen::Run;
    }

    /// Walk the run forward and punctuate each critical as its verdict lands.
    fn advance_run(&mut self, dt: f32) {
        let Some(playback) = self.playback.as_mut() else {
            return;
        };

        let was_phase = playback.phase();
        let was_door = playback.door_index();
        let fast_forward = is_key_down(KeyCode::Space);
        playback.update(dt, fast_forward);

        if !playback.verdict_just_landed(was_phase, was_door) {
            return;
        }
        let _ = was_phase;

        let index = playback.door_index();
        let doors = playback.report().doors.len();
        let Some(door) = playback.current_door() else {
            return;
        };
        let (text, color) = match door.result.outcome {
            Outcome::CriticalSuccess => ("CRITICAL", Color::new(0.46, 0.88, 0.56, 1.0)),
            Outcome::CriticalFailure => ("DISASTER", Color::new(0.92, 0.36, 0.34, 1.0)),
            _ => return,
        };

        if let Some(center) = ui::run::room_center(index, doors) {
            self.floats.spawn(text, center, color);
        }
    }

    fn finish_run(&mut self) {
        let Some(playback) = self.playback.take() else {
            return;
        };
        self.floats.clear();
        self.selection.last_report = Some(playback.into_report());
        self.selection.screen = Screen::Results;
    }

    fn new_campaign(&mut self) {
        let seed = new_seed();
        self.session = GameSession::new(&self.data.config, &self.data, seed);
        self.selection = Selection::default();
        self.playback = None;
        self.floats.clear();
        self.notifications
            .info(format!("New campaign opened on seed {}", seed));
    }

    fn save_game(&mut self) {
        let config: &GameConfig = &self.data.config;
        let save = self.session.to_save(&config.version);
        match save_to_slot_with_version(
            &config.game_name,
            &config.save_slot,
            &save,
            &config.version,
        ) {
            Ok(()) => {
                self.notifications.success("Campaign filed away");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.danger(format!("Save failed: {}", err)),
        }
    }

    fn load_game(&mut self) {
        let config = self.data.config.clone();
        let loaded: Result<SaveData, String> = load_from_slot_with_migration(
            &config.game_name,
            &config.save_slot,
            &config.version,
            |version, value| migrate_save_value(version, value, &config),
        );

        match loaded {
            Ok(save) => {
                self.session = GameSession::from_save(save);
                self.selection = Selection::default();
                self.playback = None;
                self.floats.clear();
                self.notifications.success("Campaign picked back up");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.warning(format!("Load failed: {}", err)),
        }
    }

    fn delete_save(&mut self) {
        match delete_slot(&self.data.config.game_name, &self.data.config.save_slot) {
            Ok(()) => {
                self.notifications.info("Save slot burned");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.danger(format!("Delete failed: {}", err)),
        }
    }

    fn refresh_save_state(&mut self) {
        self.save_exists = slot_exists(&self.data.config.game_name, &self.data.config.save_slot);
    }
}

/// Fixed seed for the screenshot harness, so captures are comparable run to run.
const CAPTURE_SEED: u64 = 20_260_726;

fn new_seed() -> u64 {
    random_u64()
}
