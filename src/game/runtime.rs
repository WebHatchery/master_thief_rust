use super::Game;
use crate::audio::Sfx;
use crate::heist_actions::{self, Dispatch, GameCommand};
use crate::playback::RunPlayback;
use crate::rules::Outcome;
use crate::ui::{self, Screen, UiAction, UiContext};
use macroquad::prelude::*;
use macroquad_toolkit::notifications::{NotificationAnchor, NotificationRenderConfig};
use macroquad_toolkit::prelude::{begin_virtual_ui_frame, dark, end_virtual_ui_frame};

impl Game {
    pub fn update(&mut self, dt: f32) {
        self.notifications.update(dt);
        self.floats.update(dt);
        self.advance_run(dt);

        if is_key_pressed(KeyCode::Tab) {
            let next = match self.selection.screen {
                Screen::Crew => Screen::Board,
                Screen::Board => Screen::Shop,
                Screen::Shop => Screen::Results,
                Screen::Records => Screen::Crew,
                // Tab never walks out of a plan under construction, and never
                // out of a run in progress.
                Screen::Planning => Screen::Planning,
                Screen::Run => Screen::Run,
                Screen::Results => Screen::Records,
            };
            self.events.push(UiAction::ShowScreen(next));
        }
        if is_key_pressed(KeyCode::Escape) {
            self.events.push(if self.selection.settings_open {
                UiAction::CloseSettings
            } else {
                UiAction::OpenSettings
            });
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
            crew_tab: self.selection.crew_tab,
            draft: self.selection.draft.as_ref(),
            playback: self.playback.as_ref(),
            last_report: self.selection.last_report.as_ref(),
            prefs: &self.prefs,
            artwork: &self.artwork,
            settings_open: self.selection.settings_open,
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
                prefs: &mut self.prefs,
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
            Some(GameCommand::SavePreferences) => self.save_preferences(),
            None => {}
        }
    }

    fn start_run(&mut self, report: crate::sim::JobReport) {
        self.floats.clear();
        let playback = RunPlayback::new(report);

        // A player who has asked not to watch is taken at their word: the dice
        // were cast at commit either way (GDD 9, and accessibility).
        if self.prefs.pacing.skips_the_run() {
            self.selection.last_report = Some(playback.into_report());
            self.selection.screen = Screen::Results;
            self.sound.play(Sfx::Payout);
            return;
        }

        self.playback = Some(playback);
        self.selection.screen = Screen::Run;
        self.sound.play(Sfx::DiceThrow);
    }

    /// Walk the run forward and punctuate each critical as its verdict lands.
    fn advance_run(&mut self, dt: f32) {
        let Some(playback) = self.playback.as_mut() else {
            return;
        };

        let was_phase = playback.phase();
        let was_door = playback.door_index();
        let was_landed = playback.roll_landed();
        let fast_forward = is_key_down(KeyCode::Space);
        playback.update(dt * self.prefs.pacing.speed(), fast_forward);

        if !was_landed && playback.roll_landed() {
            self.sound.play(Sfx::DiceLand);
        }

        if !playback.verdict_just_landed(was_phase, was_door) {
            return;
        }
        let _ = was_phase;

        let index = playback.door_index();
        let doors = playback.report().doors.len();
        let target_id = playback.report().target_id.clone();
        let Some(door) = playback.current_door() else {
            return;
        };
        let outcome = door.result.outcome;
        let (text, color) = match outcome {
            Outcome::CriticalSuccess => ("CRITICAL", Color::new(0.46, 0.88, 0.56, 1.0)),
            Outcome::CriticalFailure => ("DISASTER", Color::new(0.92, 0.36, 0.34, 1.0)),
            _ => {
                self.sound.play(if outcome.passed() {
                    Sfx::DoorPassed
                } else {
                    Sfx::DoorFailed
                });
                return;
            }
        };

        self.sound.play(if outcome == Outcome::CriticalSuccess {
            Sfx::Critical
        } else {
            Sfx::Disaster
        });
        if let Some(center) = ui::run::room_center(&target_id, index, doors) {
            self.floats.spawn(text, center, color);
        }
    }

    fn finish_run(&mut self) {
        let Some(playback) = self.playback.take() else {
            return;
        };
        self.floats.clear();
        let report = playback.into_report();
        self.sound.play(if report.success {
            Sfx::Payout
        } else {
            Sfx::DoorFailed
        });
        self.selection.last_report = Some(report);
        self.selection.screen = Screen::Results;
    }
}
