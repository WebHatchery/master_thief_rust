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
mod tests;
