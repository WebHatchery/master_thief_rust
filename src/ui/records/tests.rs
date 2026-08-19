use super::*;

#[test]
fn achievement_batches_cover_the_current_four_shelves() {
    let mut counts = [0usize; 4];
    for index in 0..82 {
        counts[achievement_batch(index)] += 1;
    }
    assert_eq!(counts, [21, 21, 21, 19]);
    assert_eq!(achievement_batch(81), 3);
}

#[test]
fn badge_renderer_states_are_explicit() {
    assert_ne!(AchievementBadgeState::Locked, AchievementBadgeState::Earned);
    assert_ne!(
        AchievementBadgeState::Earned,
        AchievementBadgeState::Notable
    );
}
