use super::*;
use macroquad_toolkit::synth::render_waveform;

#[test]
fn every_effect_has_voices_behind_it() {
    for sfx in Sfx::ALL {
        assert!(!voices_for(sfx).is_empty(), "{:?} is silent", sfx);
    }
}

#[test]
fn every_effect_renders_to_audible_samples() {
    for (index, sfx) in Sfx::ALL.into_iter().enumerate() {
        let samples = render_waveform(&voices_for(sfx), &config(), index as u64);
        assert!(!samples.is_empty(), "{:?} rendered nothing", sfx);

        let peak = samples.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
        assert!(peak > 0.01, "{:?} is inaudible ({peak})", sfx);
        assert!(peak <= 1.0, "{:?} clips at {peak}", sfx);
    }
}

#[test]
fn nothing_outstays_its_welcome() {
    // A planner's sounds punctuate; they do not perform.
    for sfx in Sfx::ALL {
        let end = voices_for(sfx)
            .iter()
            .map(|voice| voice.start() + voice.duration())
            .fold(0.0f32, f32::max);
        assert!(end <= 0.45, "{:?} runs for {end}s", sfx);
    }
}

#[test]
fn a_muted_bank_plays_nothing_and_says_so() {
    let bank = SoundBank::muted();
    assert!(bank.is_empty());
    bank.play(Sfx::Critical);
}

#[test]
fn volume_is_clamped_to_something_a_speaker_can_hold() {
    let mut bank = SoundBank::muted();
    bank.set_volume(4.0);
    assert_eq!(bank.volume, 1.0);
    bank.set_volume(-1.0);
    assert_eq!(bank.volume, 0.0);
}
