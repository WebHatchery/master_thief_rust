use super::*;

#[test]
fn a_new_player_watches_the_dice_with_the_sound_on() {
    let prefs = Preferences::default();
    assert_eq!(prefs.pacing, RunPacing::Full);
    assert!(prefs.sound);
    assert!(prefs.hints, "hints are on until they are turned off");
    assert!(prefs.effective_volume() > 0.0);
}

#[test]
fn turning_the_sound_off_is_silence_not_a_quiet_floor() {
    let prefs = Preferences {
        sound: false,
        volume: 1.0,
        ..Preferences::default()
    };
    assert_eq!(prefs.effective_volume(), 0.0);
}

#[test]
fn a_volume_from_an_edited_save_is_brought_back_into_range() {
    let mut prefs = Preferences {
        volume: 40.0,
        ..Preferences::default()
    };
    prefs.sanitize();
    assert_eq!(prefs.volume, 1.0);
}

#[test]
fn pacing_cycles_through_every_option_and_returns() {
    let mut pacing = RunPacing::Full;
    for _ in 0..RunPacing::ALL.len() {
        pacing = pacing.next();
    }
    assert_eq!(pacing, RunPacing::Full);
}

#[test]
fn only_the_instant_setting_skips_the_watching() {
    assert!(RunPacing::Instant.skips_the_run());
    assert!(!RunPacing::Full.skips_the_run());
    assert!(!RunPacing::Brisk.skips_the_run());
    assert!(RunPacing::Brisk.speed() > RunPacing::Full.speed());
}
