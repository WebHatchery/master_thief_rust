use super::{Game, CAPTURE_SEED};
use crate::audio::SoundBank;
use crate::heist_actions::Selection;
use crate::playback::RunPlayback;
use crate::sim;
use crate::state::GameSession;
use crate::ui::{self, Screen};

impl Game {
    /// Boot straight into a named screen for the headless capture harness. The
    /// campaign is reseeded to a fixed value so every capture is the same
    /// campaign, and the results scene runs a job so it has something to show.
    pub fn set_capture_scene(&mut self, scene: &str) {
        self.session = GameSession::new(&self.data.config, &self.data, CAPTURE_SEED);
        self.selection = Selection::default();
        // A screenshot has nothing to listen with.
        self.sound = SoundBank::muted();

        self.selection.screen = match scene {
            "board" | "targets" => {
                // Age the board without working it, so the capture shows marks
                // that have ripened rather than five identical fresh ones. Done
                // directly rather than by advancing weeks: a screenshot has no
                // business running the payroll.
                for _ in 0..2 {
                    self.session.age_board();
                }
                // And put part of the first mark on the file, so the shot shows
                // what half-scouted looks like rather than all-or-nothing.
                if let Some(entry) = self.session.board.first_mut() {
                    entry.casing = 2;
                }
                self.session.attention_spent_this_week = 2;
                // And a trade already on the city's file, so the shot carries
                // the watched-door line the board exists to warn with.
                self.note_capture_method(0);
                self.selection.target = self
                    .session
                    .board
                    .first()
                    .map(|entry| entry.target_id.clone());
                Screen::Board
            }
            "settings" => {
                self.selection.settings_open = true;
                Screen::Crew
            }
            "shop" | "outfitter" => {
                // A few weeks of loot, so the shelf has spares on it and the
                // fence has something to quote against. An empty lockup shows
                // half the screen.
                crate::sim::play(&mut self.session, &self.data, 8);
                Screen::Shop
            }
            "records" => {
                crate::sim::play(&mut self.session, &self.data, 12);
                crate::sim::award(&mut self.session, &self.data.config, &self.data.awards);
                Screen::Records
            }
            "hiring" => {
                // A few weeks in, so the notoriety premium is on the asking
                // prices rather than an unknown outfit paying list.
                crate::sim::play(&mut self.session, &self.data, 10);
                self.selection.crew_tab = ui::CrewTab::ForHire;
                Screen::Crew
            }
            "outfit" | "payroll" => {
                // Fourteen weeks in, every line of the books has something on
                // it: wages paid, heat carried, whoever is threatening to walk,
                // and a trade the city has finally seen enough of. At twelve
                // the method file is real but still a point under the first
                // band, so the row it exists to show photographs as empty.
                crate::sim::play(&mut self.session, &self.data, 14);
                self.selection.crew_tab = ui::CrewTab::Outfit;
                // Show a dossier with something on it: whoever is hurt, so the
                // shot carries the treat quote rather than an idle column.
                self.selection.member = self
                    .session
                    .crew
                    .iter()
                    .find(|member| !member.condition.injuries.is_empty())
                    .map(|member| member.id.clone());
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
            _ => {
                // A roster where everybody reads "Ready" photographs none of
                // the states the screen exists to distinguish, and a dossier
                // with nothing banked photographs none of the decision.
                self.wear_out_a_hand();
                self.bank_a_levels_worth();
                Screen::Crew
            }
        };
    }

    /// Put a level's worth of unspent points on the dossier the crew screen
    /// opens on, and half the week's hours behind them, so the shot carries
    /// what training now costs rather than an empty column.
    fn bank_a_levels_worth(&mut self) {
        if let Some(member) = self.session.crew.first_mut() {
            member.progression.attribute_points = 1;
            member.progression.skill_points = 2;
        }
        self.session.attention_spent_this_week = 1;
    }

    /// Run one hand past the working threshold. They stay selectable — that is
    /// the whole point of the state — so the capture shows a crew the fixer can
    /// still send and should not (GDD 5.6).
    fn wear_out_a_hand(&mut self) {
        let threshold = self.data.config.condition.fatigue_work_threshold;
        if let Some(member) = self.session.crew.last_mut() {
            member.condition.fatigue = threshold + 12;
        }
    }

    /// Put one of the first mark's trades on the city's file, at the top of the
    /// curve. Done directly rather than by working twelve weeks of jobs: the
    /// capture wants the modifier on screen, not a campaign behind it. The door
    /// index is the caller's, because each scene frames a different one and a
    /// penalty on a door nobody is looking at photographs as nothing.
    fn note_capture_method(&mut self, door_index: usize) {
        let Some(target) = self
            .session
            .board
            .first()
            .and_then(|entry| self.data.targets.get(&entry.target_id))
        else {
            return;
        };
        let doors = self.data.encounters_for(target);
        let Some(door) = doors.get(door_index).copied() else {
            return;
        };
        let tuning = self.data.config.scrutiny;
        self.session
            .scrutiny
            .note(door.primary_skill, tuning.ceiling(), &tuning);
    }

    /// A cased mark with the crew's own picks already in, so the capture shows
    /// difficulties, assignments, and odds rather than an empty draft.
    fn open_capture_plan(&mut self) {
        self.wear_out_a_hand();
        let Some(entry) = self.session.board.first_mut() else {
            return;
        };
        let target_id = entry.target_id.clone();
        entry.casing = 99;

        let Some(target) = self.data.targets.get(&target_id).cloned() else {
            return;
        };
        // Open on a door that can rewrite the run where the mark has one, so
        // the capture carries the telegraph rather than a door that is only
        // ever itself. Its trade is the one the city is watching, for the same
        // reason: a shot of the breakdown wants the lines actually on it.
        let focus = self
            .data
            .encounters_for(&target)
            .iter()
            .position(|encounter| encounter.can_rewrite_the_run())
            .unwrap_or(target.encounters.len().saturating_sub(1));
        self.note_capture_method(focus);
        let mut draft = sim::PlanDraft::from_auto(&self.session, &self.data, &target);
        draft.clear(focus);
        draft.focus_on(focus);
        // A standing order actually set, so the footer photographs the decision
        // rather than its default.
        draft.cycle_nerve();

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
}
