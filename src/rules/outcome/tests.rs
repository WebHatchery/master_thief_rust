use super::*;

#[test]
fn natural_one_and_twenty_override_the_margins() {
    assert_eq!(Outcome::classify(1, 99, 10), Outcome::CriticalFailure);
    assert_eq!(Outcome::classify(20, -99, 10), Outcome::CriticalSuccess);
}

#[test]
fn margins_pick_the_band() {
    assert_eq!(Outcome::classify(10, 25, 15), Outcome::CriticalSuccess);
    assert_eq!(Outcome::classify(10, 20, 15), Outcome::Success);
    assert_eq!(Outcome::classify(10, 15, 15), Outcome::Neutral);
    assert_eq!(Outcome::classify(10, 11, 15), Outcome::Failure);
    assert_eq!(Outcome::classify(10, 9, 15), Outcome::CriticalFailure);
}

#[test]
fn success_grants_more_experience_and_less_stress_than_failure() {
    let dc = 12;
    assert!(Outcome::Success.experience_for(dc) > Outcome::Failure.experience_for(dc));
    assert!(Outcome::Success.stress_for(dc) < Outcome::Failure.stress_for(dc));
    assert_eq!(Outcome::CriticalSuccess.stress_for(dc), 0);
}
