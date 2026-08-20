//! The application shell: load the authored world and compose the runtime services.
//!
//! Frame-loop, capture, and persistence behavior live in child modules so this
//! type remains the composition root rather than another gameplay system.

mod capture;
mod persistence;
mod runtime;

/// Compatibility facade for callers that used the old `game::playback` path.
/// The implementation now lives beside the other shared presentation types.
pub use crate::playback;

use crate::artwork::Artwork;
use crate::audio::SoundBank;
use crate::data::GameData;
use crate::heist_actions::Selection;
use crate::prefs::Preferences;
use crate::state::GameSession;
use crate::ui::UiAction;
use macroquad_toolkit::events::EventBus;
use macroquad_toolkit::fx::FloatingTextLayer;
use macroquad_toolkit::notifications::NotificationManager;
use macroquad_toolkit::rng::random_u64;

pub struct Game {
    data: GameData,
    session: GameSession,
    selection: Selection,
    /// The job currently being watched, if any.
    playback: Option<crate::playback::RunPlayback>,
    /// Crit punctuation, hung over the room it happened in.
    floats: FloatingTextLayer,
    prefs: Preferences,
    sound: SoundBank,
    artwork: Artwork,
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

        let prefs = Preferences::load(&data.config.game_name);
        let sound = SoundBank::load(prefs.effective_volume()).await;
        let artwork = Artwork::load(&data).await;
        let session = GameSession::new(&data.config, &data, new_seed());
        let mut game = Self {
            data,
            session,
            selection: Selection::default(),
            playback: None,
            floats: FloatingTextLayer::new(),
            prefs,
            sound,
            artwork,
            notifications,
            events: EventBus::new(),
            save_exists: false,
        };
        game.refresh_save_state();
        game
    }
}

/// Fixed seed for the screenshot harness, so captures are comparable run to run.
const CAPTURE_SEED: u64 = 20_260_726;

fn new_seed() -> u64 {
    random_u64()
}
