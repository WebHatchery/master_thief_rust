use super::{new_seed, Game};
use crate::audio::Sfx;
use crate::data::GameConfig;
use crate::heist_actions::Selection;
use crate::state::{migrate_save_value, GameSession, SaveData};
use macroquad_toolkit::persistence::{
    delete_slot, load_from_slot_with_migration, save_to_slot_with_version, slot_exists,
};

impl Game {
    pub(super) fn save_preferences(&mut self) {
        self.prefs.sanitize();
        self.sound.set_volume(self.prefs.effective_volume());
        let _ = self.prefs.save(&self.data.config.game_name);
        self.sound.play(Sfx::Click);
    }

    pub(super) fn new_campaign(&mut self) {
        let seed = new_seed();
        self.session = GameSession::new(&self.data.config, &self.data, seed);
        self.selection = Selection::default();
        self.playback = None;
        self.floats.clear();
        self.notifications
            .info(format!("New campaign opened on seed {}", seed));
    }

    pub(super) fn save_game(&mut self) {
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

    pub(super) fn load_game(&mut self) {
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
                crate::state::adopt_current_definitions(&mut self.session, &self.data);
                self.selection = Selection::default();
                self.playback = None;
                self.floats.clear();
                self.notifications.success("Campaign picked back up");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.warning(format!("Load failed: {}", err)),
        }
    }

    pub(super) fn delete_save(&mut self) {
        match delete_slot(&self.data.config.game_name, &self.data.config.save_slot) {
            Ok(()) => {
                self.notifications.info("Save slot burned");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.danger(format!("Delete failed: {}", err)),
        }
    }

    pub(super) fn refresh_save_state(&mut self) {
        self.save_exists = slot_exists(&self.data.config.game_name, &self.data.config.save_slot);
    }
}
