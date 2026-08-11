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
///
/// Not `Eq`: the price of working somebody who should be resting is partly a
/// chance, and pretending two floats compare exactly would be a lie about what
/// comparing them means.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConditionTuning {
    /// Fatigue below this costs nothing.
    pub fatigue_free_threshold: i32,
    /// Every this much fatigue above the threshold costs one point.
    pub fatigue_step: i32,
    /// Fatigue above which a hand is **spent**: still able to work, and no
    /// longer able to do it well. This used to sit at the top of
    /// `game_config.json` where nothing read it, while the real bar was the
    /// number 80 written into `Condition::is_fit_for_work`.
    pub fatigue_work_threshold: i32,
    /// What being spent costs on the die, on top of the graded fatigue penalty
    /// the hand is already carrying.
    pub spent_check_penalty: i32,
    /// Added to a door's chance of hurting somebody when the hand working it is
    /// spent. Tired people get hurt; this is the half of the price that is not
    /// on the breakdown, so the planning screen says it in words instead.
    pub spent_injury_chance: f32,
    /// Loyalty a hand loses per door worked while spent. Being sent out tired
    /// is remembered.
    pub spent_loyalty_cost: i32,
    /// Injuries at which a hand genuinely cannot go. This is the hard bar, and
    /// the reason there is one: fatigue is a decision the fixer gets to make
    /// badly, and a third broken bone is not.
    pub max_injuries_for_work: usize,
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
            fatigue_work_threshold: 80,
            spent_check_penalty: 2,
            spent_injury_chance: 0.15,
            spent_loyalty_cost: 2,
            max_injuries_for_work: 2,
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

    /// Is this hand past the point where the fixer should be sending them out?
    pub fn is_spent(&self, fatigue: i32) -> bool {
        fatigue > self.fatigue_work_threshold
    }

    /// Can this hand go at all? Injuries only — the hard bar, and the only one.
    /// Fatigue used to be the other half of this test, against a hardcoded 80,
    /// which made "rest until everybody is fresh" the one move the week never
    /// argued with. Being tired is now a price, not a locked door.
    pub fn can_work(&self, condition: &crate::model::crew::Condition) -> bool {
        condition.injuries.len() <= self.max_injuries_for_work
    }

    /// Chance a door hurts the hand working it, given what state they are in.
    /// The draw itself belongs to the run's seeded RNG, not to this function.
    ///
    /// A spent hand is at risk on a door they *passed*, which is the whole
    /// point — the cost of working somebody tired is not only that they are
    /// worse at it. The one exception is the flawless door, because a critical
    /// success costing nothing at all is a rule the rest of the game already
    /// keeps.
    pub fn injury_chance(&self, outcome: super::outcome::Outcome, spent: bool) -> f32 {
        let base = outcome.injury_chance();
        if spent && outcome != super::outcome::Outcome::CriticalSuccess {
            (base + self.spent_injury_chance).min(1.0)
        } else {
            base
        }
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

    // Past the working threshold the graded penalty stops being the whole
    // story. Named separately from `Fatigue` on purpose: it is the line that
    // tells the fixer they are choosing to send somebody who should be in bed,
    // and it has to be legible as a decision rather than as more of the same
    // curve (pillar 2).
    if tuning.is_spent(condition.fatigue) && tuning.spent_check_penalty != 0 {
        entries.push(ModifierEntry::new(
            "Running on empty",
            -tuning.spent_check_penalty,
        ));
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
mod tests;
