use super::*;

fn tables(json: &str) -> OutcomeTables {
    serde_json::from_str(json).unwrap()
}

#[test]
fn a_skill_specific_list_beats_the_generic_one() {
    let t = tables(r#"{"success":{"generic":["Clean."],"stealth":["Nobody hears a thing."]}}"#);
    assert_eq!(
        t.lines(Outcome::Success, Skill::Stealth),
        ["Nobody hears a thing."]
    );
    assert_eq!(t.lines(Outcome::Success, Skill::Combat), ["Clean."]);
}

#[test]
fn an_unknown_band_reports_nothing_rather_than_panicking() {
    let t = tables(r#"{"success":{"generic":["Clean."]}}"#);
    assert!(t.lines(Outcome::CriticalFailure, Skill::Social).is_empty());
    assert!(t.line(Outcome::CriticalFailure, Skill::Social, 0).is_none());
}

#[test]
fn line_indexes_wrap() {
    let t = tables(r#"{"success":{"generic":["A","B"]}}"#);
    assert_eq!(t.line(Outcome::Success, Skill::Social, 0), Some("A"));
    assert_eq!(t.line(Outcome::Success, Skill::Social, 3), Some("B"));
}
