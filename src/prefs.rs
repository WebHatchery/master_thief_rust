//! What the player wants, as opposed to what the campaign is.
//!
//! The toolkit's `GameSettings` covers volume, display, and text scale. These
//! are the ones only this game has: how fast the dice are allowed to take, and
//! whether to watch the run at all. Both are accessibility settings as much as
//! preferences — GDD 9 asks for a skippable dice presentation, and a player who
//! cannot comfortably watch forty jobs resolve should not have to.

use macroquad_toolkit::persistence::{load_json_key, save_json_key};
use serde::{Deserialize, Serialize};

const PREFS_KEY: &str = "preferences";

/// How much of the run the player wants to sit through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunPacing {
    /// Every beat, at the pace it was designed for.
    #[default]
    Full,
    /// The same beats, twice as fast.
    Brisk,
    /// Skip straight to the results.
    Instant,
}

impl RunPacing {
    pub const ALL: [RunPacing; 3] = [RunPacing::Full, RunPacing::Brisk, RunPacing::Instant];

    pub fn label(self) -> &'static str {
        match self {
            RunPacing::Full => "Watch it",
            RunPacing::Brisk => "Brisk",
            RunPacing::Instant => "Straight to results",
        }
    }

    /// The multiplier applied to the playback clock.
    pub fn speed(self) -> f32 {
        match self {
            RunPacing::Full => 1.0,
            RunPacing::Brisk => 2.2,
            RunPacing::Instant => 1.0,
        }
    }

    pub fn skips_the_run(self) -> bool {
        matches!(self, RunPacing::Instant)
    }

    pub fn next(self) -> Self {
        match self {
            RunPacing::Full => RunPacing::Brisk,
            RunPacing::Brisk => RunPacing::Instant,
            RunPacing::Instant => RunPacing::Full,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub pacing: RunPacing,
    /// Sound on or off, independent of the volume slider.
    pub sound: bool,
    /// Effect volume, 0..1.
    pub volume: f32,
    /// Whether the tutorial hints are still wanted.
    pub hints: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            pacing: RunPacing::default(),
            sound: true,
            volume: 0.6,
            hints: true,
        }
    }
}

impl Preferences {
    pub fn load(game_name: &str) -> Self {
        let mut prefs: Self = load_json_key(game_name, PREFS_KEY).unwrap_or_default();
        prefs.sanitize();
        prefs
    }

    pub fn save(&self, game_name: &str) -> Result<(), String> {
        save_json_key(game_name, PREFS_KEY, self)
    }

    pub fn sanitize(&mut self) {
        self.volume = self.volume.clamp(0.0, 1.0);
    }

    /// The volume the sound bank should actually use.
    pub fn effective_volume(&self) -> f32 {
        if self.sound {
            self.volume.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests;
