//! Sound, generated rather than sampled.
//!
//! Every effect is a handful of `synth` voices rendered to WAV at boot, so the
//! game ships no audio files at all — which is the same argument the floorplan
//! makes about art. The set is small on purpose: a run is mostly quiet, and the
//! dice have to be the loudest thing in it.

use macroquad::audio::PlaySoundParams;
use macroquad_toolkit::audio::SoundManager;
use macroquad_toolkit::synth::{render_wav, SynthConfig, Voice, Wave};

/// The whole vocabulary of the game's noise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sfx {
    /// A button, a tab, an assignment.
    Click,
    /// The die leaving the hand.
    DiceThrow,
    /// The die landing on a number.
    DiceLand,
    /// A door cleared.
    DoorPassed,
    /// A door blown.
    DoorFailed,
    /// A natural twenty.
    Critical,
    /// A natural one.
    Disaster,
    /// The take, counted.
    Payout,
    /// A week turning over.
    Week,
}

impl Sfx {
    pub const ALL: [Sfx; 9] = [
        Sfx::Click,
        Sfx::DiceThrow,
        Sfx::DiceLand,
        Sfx::DoorPassed,
        Sfx::DoorFailed,
        Sfx::Critical,
        Sfx::Disaster,
        Sfx::Payout,
        Sfx::Week,
    ];
}

fn config() -> SynthConfig {
    SynthConfig::default()
}

/// The voices behind each effect. Kept deliberately terse — a heist planner
/// wants a click and a clatter, not a fanfare.
fn voices_for(sfx: Sfx) -> Vec<Voice> {
    match sfx {
        Sfx::Click => vec![Voice::tone(0.0, 0.045, 880.0, 0.18)
            .wave(Wave::Square)
            .attack(0.004)],

        // A wooden rattle: three short knocks falling in pitch.
        Sfx::DiceThrow => vec![
            Voice::tone(0.0, 0.05, 420.0, 0.16).wave(Wave::Noise),
            Voice::tone(0.06, 0.05, 360.0, 0.14).wave(Wave::Noise),
            Voice::tone(0.13, 0.06, 300.0, 0.12).wave(Wave::Noise),
        ],

        Sfx::DiceLand => vec![
            Voice::tone(0.0, 0.08, 240.0, 0.26)
                .wave(Wave::Noise)
                .attack(0.002),
            Voice::tone(0.0, 0.16, 180.0, 0.16).wave(Wave::Sine),
        ],

        Sfx::DoorPassed => vec![
            Voice::tone(0.0, 0.10, 523.0, 0.20).wave(Wave::Triangle),
            Voice::tone(0.08, 0.14, 784.0, 0.16).wave(Wave::Triangle),
        ],

        Sfx::DoorFailed => vec![Voice::tone(0.0, 0.22, 220.0, 0.22)
            .glide(150.0)
            .wave(Wave::Square)],

        Sfx::Critical => vec![
            Voice::tone(0.0, 0.10, 659.0, 0.20).wave(Wave::Triangle),
            Voice::tone(0.09, 0.10, 880.0, 0.20).wave(Wave::Triangle),
            Voice::tone(0.18, 0.22, 1319.0, 0.18).wave(Wave::Sine),
        ],

        Sfx::Disaster => vec![
            Voice::tone(0.0, 0.30, 196.0, 0.24)
                .glide(98.0)
                .wave(Wave::Square),
            Voice::tone(0.05, 0.25, 92.0, 0.18).wave(Wave::Square),
        ],

        // Coins, counted quickly.
        Sfx::Payout => vec![
            Voice::tone(0.0, 0.06, 1046.0, 0.14).wave(Wave::Triangle),
            Voice::tone(0.05, 0.06, 1318.0, 0.14).wave(Wave::Triangle),
            Voice::tone(0.10, 0.06, 1568.0, 0.13).wave(Wave::Triangle),
            Voice::tone(0.15, 0.18, 2093.0, 0.11).wave(Wave::Sine),
        ],

        Sfx::Week => vec![
            Voice::tone(0.0, 0.20, 294.0, 0.16).wave(Wave::Sine),
            Voice::tone(0.12, 0.26, 220.0, 0.14).wave(Wave::Sine),
        ],
    }
}

/// The rendered set, plus the volume the player asked for.
pub struct SoundBank {
    sounds: SoundManager<Sfx>,
    volume: f32,
    muted: bool,
}

impl SoundBank {
    /// A bank that plays nothing. The screenshot harness uses this, and so does
    /// any platform where the audio device refuses to open.
    pub fn muted() -> Self {
        Self {
            sounds: SoundManager::new(),
            volume: 0.0,
            muted: true,
        }
    }

    pub async fn load(volume: f32) -> Self {
        let mut sounds = SoundManager::new();

        for (index, sfx) in Sfx::ALL.into_iter().enumerate() {
            let bytes = render_wav(&voices_for(sfx), &config(), 0x51F_7000_u64 + index as u64);
            // One effect failing to decode is not worth losing the rest.
            let _ = sounds.load_sound_bytes(sfx, &bytes).await;
        }

        Self {
            sounds,
            volume: volume.clamp(0.0, 1.0),
            muted: false,
        }
    }

    pub fn len(&self) -> usize {
        self.sounds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }

    /// Zero is true silence, not a near-inaudible floor.
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    pub fn play(&self, sfx: Sfx) {
        self.play_at(sfx, 1.0);
    }

    pub fn play_at(&self, sfx: Sfx, gain: f32) {
        if self.muted || self.volume <= 0.0 {
            return;
        }
        self.sounds.play_raw(
            sfx,
            PlaySoundParams {
                looped: false,
                volume: (self.volume * gain).clamp(0.0, 1.0),
            },
        );
    }
}

#[cfg(test)]
mod tests;
