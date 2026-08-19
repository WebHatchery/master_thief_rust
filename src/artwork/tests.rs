use super::PortraitState;
use crate::model::CharacterClass;

#[test]
fn portrait_state_keys_cover_every_visual_state() {
    let states = [
        PortraitState::Neutral,
        PortraitState::Speaking,
        PortraitState::Pleased,
        PortraitState::Worried,
        PortraitState::Injured,
        PortraitState::Exhausted,
        PortraitState::Arrested,
        PortraitState::Unavailable,
        PortraitState::Unknown,
        PortraitState::Locked,
    ];
    let keys: Vec<_> = states.into_iter().map(PortraitState::key).collect();
    assert_eq!(keys.len(), 10);
    assert!(keys.iter().all(|key| !key.is_empty()));
}

#[test]
fn class_badges_have_one_key_per_current_class() {
    let classes = [
        CharacterClass::Infiltrator,
        CharacterClass::Tech,
        CharacterClass::Face,
        CharacterClass::Muscle,
        CharacterClass::Acrobat,
        CharacterClass::Mastermind,
        CharacterClass::Wildcard,
    ];
    let labels: Vec<_> = classes.into_iter().map(CharacterClass::label).collect();
    assert_eq!(labels.len(), 7);
    assert_eq!(
        labels
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        7
    );
}
