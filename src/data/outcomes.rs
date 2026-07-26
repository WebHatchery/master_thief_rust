//! The narrative tables laid over the dice.
//!
//! Keyed band -> skill, with a `generic` list every band carries as a floor so
//! a new skill can never leave the run screen mute.

use crate::model::Skill;
use crate::rules::Outcome;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const GENERIC_KEY: &str = "generic";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutcomeTables {
    #[serde(flatten)]
    bands: HashMap<String, HashMap<String, Vec<String>>>,
}

impl OutcomeTables {
    /// Lines available for a band and skill: the skill's own list if it has
    /// one, otherwise the band's generic list.
    pub fn lines(&self, outcome: Outcome, skill: Skill) -> &[String] {
        let Some(band) = self.bands.get(outcome.key()) else {
            return &[];
        };

        band.get(skill.key())
            .filter(|lines| !lines.is_empty())
            .or_else(|| band.get(GENERIC_KEY))
            .map(|lines| lines.as_slice())
            .unwrap_or(&[])
    }

    /// Pick a line by index, wrapping. The draw belongs to the run's seeded
    /// RNG; this only does the lookup.
    pub fn line(&self, outcome: Outcome, skill: Skill, index: usize) -> Option<&str> {
        let lines = self.lines(outcome, skill);
        if lines.is_empty() {
            None
        } else {
            Some(lines[index % lines.len()].as_str())
        }
    }

    /// Total authored lines, for the content inventory check.
    pub fn total_lines(&self) -> usize {
        self.bands
            .values()
            .flat_map(|band| band.values())
            .map(|lines| lines.len())
            .sum()
    }
}

#[cfg(test)]
mod tests {
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
}
