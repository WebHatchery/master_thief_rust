//! Outcome bands and the named modifiers that produced them.

use serde::{Deserialize, Serialize};

/// One line of a check's arithmetic, named so the player can read it.
/// Pillar 2: no hidden difficulty, ever.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifierEntry {
    pub label: String,
    pub value: i32,
}

impl ModifierEntry {
    pub fn new(label: impl Into<String>, value: i32) -> Self {
        Self {
            label: label.into(),
            value,
        }
    }

    /// Entries worth showing: a zero contributes nothing and only crowds the
    /// planning screen.
    pub fn is_significant(&self) -> bool {
        self.value != 0
    }

    pub fn signed(&self) -> String {
        if self.value >= 0 {
            format!("+{}", self.value)
        } else {
            self.value.to_string()
        }
    }
}

/// The five bands a check can land in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    CriticalFailure,
    Failure,
    Neutral,
    Success,
    CriticalSuccess,
}

impl Outcome {
    pub const ALL: [Outcome; 5] = [
        Outcome::CriticalFailure,
        Outcome::Failure,
        Outcome::Neutral,
        Outcome::Success,
        Outcome::CriticalSuccess,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Outcome::CriticalFailure => "Critical Failure",
            Outcome::Failure => "Failure",
            Outcome::Neutral => "Scraped Through",
            Outcome::Success => "Success",
            Outcome::CriticalSuccess => "Critical Success",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Outcome::CriticalFailure => "critical_failure",
            Outcome::Failure => "failure",
            Outcome::Neutral => "neutral",
            Outcome::Success => "success",
            Outcome::CriticalSuccess => "critical_success",
        }
    }

    /// Did the crew get through the door at all?
    pub fn passed(self) -> bool {
        matches!(
            self,
            Outcome::Neutral | Outcome::Success | Outcome::CriticalSuccess
        )
    }

    /// Experience the door is worth at a given difficulty class.
    pub fn experience_for(self, dc: i32) -> i32 {
        match self {
            Outcome::CriticalSuccess => dc * 3,
            Outcome::Success => dc * 2,
            Outcome::Neutral => dc,
            Outcome::Failure => dc / 2,
            Outcome::CriticalFailure => dc / 4,
        }
    }

    /// Fatigue the door inflicts at a given difficulty class.
    pub fn stress_for(self, dc: i32) -> i32 {
        match self {
            Outcome::CriticalSuccess => 0,
            Outcome::Success => dc / 2,
            Outcome::Neutral => dc,
            Outcome::Failure => dc * 2,
            Outcome::CriticalFailure => dc * 3,
        }
    }

    /// Chance the member walks away hurt. The draw itself belongs to the run's
    /// seeded RNG, not to this function.
    pub fn injury_chance(self) -> f32 {
        match self {
            Outcome::CriticalFailure => 0.4,
            Outcome::Failure => 0.2,
            _ => 0.0,
        }
    }

    /// Classify a total against a difficulty class. Natural 1 and 20 override
    /// the margins entirely, exactly as the original engine did.
    pub fn classify(roll: i32, total: i32, dc: i32) -> Outcome {
        if roll == 1 {
            Outcome::CriticalFailure
        } else if roll == 20 || total >= dc + 10 {
            Outcome::CriticalSuccess
        } else if total >= dc + 5 {
            Outcome::Success
        } else if total >= dc {
            Outcome::Neutral
        } else if total >= dc - 5 {
            Outcome::Failure
        } else {
            Outcome::CriticalFailure
        }
    }
}

#[cfg(test)]
mod tests;
