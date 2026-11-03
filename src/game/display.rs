//! The client's display preference and its timed fullscreen confirmation.

#[cfg(not(target_arch = "wasm32"))]
use macroquad::prelude::set_fullscreen;
use macroquad_toolkit::settings::{DisplayPreview, GameSettings};

struct ActivePreview {
    candidate: GameSettings,
    preview: DisplayPreview,
}

pub(super) struct DisplaySettings {
    game_name: String,
    committed: GameSettings,
    active_preview: Option<ActivePreview>,
}

impl DisplaySettings {
    pub(super) fn load(game_name: &str, capture_mode: bool) -> Self {
        let mut committed = GameSettings::load(game_name);
        if capture_mode {
            committed.fullscreen = false;
        }
        #[cfg(target_arch = "wasm32")]
        {
            // The browser can enter fullscreen only after a user gesture.
            committed.fullscreen = false;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if committed.fullscreen && !capture_mode {
            set_fullscreen(true);
        }

        Self {
            game_name: game_name.to_owned(),
            committed,
            active_preview: None,
        }
    }

    pub(super) fn fullscreen(&self) -> bool {
        self.active_preview
            .as_ref()
            .map(|active| active.candidate.fullscreen)
            .unwrap_or(self.committed.fullscreen)
    }

    pub(super) fn is_previewing(&self) -> bool {
        self.active_preview.is_some()
    }

    pub(super) fn preview_remaining(&self) -> Option<f32> {
        self.active_preview
            .as_ref()
            .map(|active| active.preview.remaining())
    }

    pub(super) fn begin_preview(&mut self) {
        if self.active_preview.is_some() {
            return;
        }
        let mut candidate = self.committed.clone();
        candidate.fullscreen = !candidate.fullscreen;
        let preview = DisplayPreview::begin(&self.committed, &candidate);
        self.active_preview = Some(ActivePreview { candidate, preview });
    }

    /// Returns true after the original display mode has been restored.
    pub(super) fn update(&mut self, real_dt: f32) -> bool {
        let expired = self
            .active_preview
            .as_mut()
            .map(|active| active.preview.update(real_dt))
            .unwrap_or(false);
        if expired {
            self.active_preview = None;
        }
        expired
    }

    pub(super) fn keep_preview(&mut self) -> Result<(), String> {
        let Some(mut active) = self.active_preview.take() else {
            return Ok(());
        };
        if let Err(error) = active.candidate.save(&self.game_name) {
            active.preview.revert();
            return Err(error);
        }
        active.preview.confirm();
        self.committed = active.candidate;
        Ok(())
    }

    pub(super) fn revert_preview(&mut self) {
        if let Some(mut active) = self.active_preview.take() {
            active.preview.revert();
        }
    }
}
