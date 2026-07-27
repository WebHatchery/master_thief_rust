//! What the state a hand is in is worth on the die.
//!
//! Fatigue, loyalty, and injuries are the three things a campaign does to a
//! person, and each of them reads as its own named line on the check (pillar
//! 2). The curves are authored in `game_config.json` under `condition`: their
//! *shape* is a rule and lives here, but every number on them is balance and
//! does not.

use super::outcome::ModifierEntry;
use crate::model::CrewMember;
use serde::{Deserialize, Serialize};

/// Where fatigue starts costing dice, and how loyalty grades into a modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionTuning {
    /// Fatigue below this costs nothing.
    pub fatigue_free_threshold: i32,
    /// Every this much fatigue above the threshold costs one point.
    pub fatigue_step: i32,
    /// The loyalty at which a hand is neither steadied nor shaken.
    pub loyalty_neutral: i32,
    /// Every this much loyalty either side of neutral is worth one point.
    pub loyalty_step: i32,
    pub loyalty_max_bonus: i32,
    pub loyalty_max_penalty: i32,
}

impl Default for ConditionTuning {
    fn default() -> Self {
        Self {
            fatigue_free_threshold: 50,
            fatigue_step: 10,
            loyalty_neutral: 60,
            loyalty_step: 20,
            loyalty_max_bonus: 2,
            loyalty_max_penalty: 3,
        }
    }
}

impl ConditionTuning {
    /// What a hand's loyalty is worth on the die, graded rather than banded: a
    /// crew drifting from steady to sullen loses ground the whole way down,
    /// so goodwill the outfit burns shows up immediately instead of at a cliff.
    pub fn loyalty_modifier(&self, loyalty: i32) -> i32 {
        if self.loyalty_step <= 0 {
            return 0;
        }
        (loyalty - self.loyalty_neutral)
            .div_euclid(self.loyalty_step)
            .clamp(-self.loyalty_max_penalty, self.loyalty_max_bonus)
    }

    pub fn fatigue_modifier(&self, fatigue: i32) -> i32 {
        if self.fatigue_step <= 0 || fatigue <= self.fatigue_free_threshold {
            return 0;
        }
        -((fatigue - self.fatigue_free_threshold) / self.fatigue_step)
    }
}

/// Fatigue, loyalty, and injuries, each named separately so a player can see
/// which one is costing them the job.
pub fn condition_entries(member: &CrewMember, tuning: &ConditionTuning) -> Vec<ModifierEntry> {
    let condition = &member.condition;
    let mut entries = Vec::new();

    let fatigue = tuning.fatigue_modifier(condition.fatigue);
    if fatigue != 0 {
        entries.push(ModifierEntry::new("Fatigue", fatigue));
    }

    // A hand who has given notice is already half out of the door, and it shows
    // on every one they open (GDD 5.6).
    let loyalty = tuning.loyalty_modifier(condition.loyalty);
    match loyalty {
        0 => {}
        value if value > 0 => entries.push(ModifierEntry::new("Loyalty", value)),
        value => entries.push(ModifierEntry::new(
            if condition.notice_given {
                "Working their notice"
            } else {
                "Wavering loyalty"
            },
            value,
        )),
    }

    let injury_penalty: i32 = condition
        .injuries
        .iter()
        .map(|injury| injury.severity.check_penalty())
        .sum();
    if injury_penalty != 0 {
        entries.push(ModifierEntry::new(
            format!("Injuries ({})", condition.injuries.len()),
            injury_penalty,
        ));
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loyalty_grades_all_the_way_down_rather_than_falling_off_a_cliff() {
        // Two flat bands made loyalty a switch. Graded, every point of goodwill
        // the outfit burns is worth something on the die.
        let tuning = ConditionTuning::default();
        let steps: Vec<i32> = [100, 80, 60, 40, 20, 0]
            .into_iter()
            .map(|loyalty| tuning.loyalty_modifier(loyalty))
            .collect();

        assert_eq!(steps, vec![2, 1, 0, -1, -2, -3]);
        for pair in steps.windows(2) {
            assert!(pair[0] >= pair[1], "the curve doubles back: {:?}", steps);
        }
    }

    #[test]
    fn the_grading_never_runs_past_its_own_bounds() {
        let tuning = ConditionTuning::default();
        assert_eq!(tuning.loyalty_modifier(1_000), tuning.loyalty_max_bonus);
        assert_eq!(tuning.loyalty_modifier(-1_000), -tuning.loyalty_max_penalty);
        assert_eq!(tuning.fatigue_modifier(0), 0);
        assert_eq!(tuning.fatigue_modifier(tuning.fatigue_free_threshold), 0);
    }

    #[test]
    fn a_tuning_with_no_step_in_it_refuses_to_divide_by_nothing() {
        let flat = ConditionTuning {
            loyalty_step: 0,
            fatigue_step: 0,
            ..ConditionTuning::default()
        };
        assert_eq!(flat.loyalty_modifier(0), 0);
        assert_eq!(flat.fatigue_modifier(100), 0);
    }
}
