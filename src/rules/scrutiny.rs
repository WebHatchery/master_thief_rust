//! What the city learns from watching an outfit work.
//!
//! Heat is *how much* attention the outfit has drawn. This is what the
//! attention is about. Every door the crew open teaches the city a little more
//! about the trade they opened it with, and buildings across the whole board
//! harden against that trade for as long as the file stays warm.
//!
//! It exists because the board was still a stock list in the one way GDD 5.4
//! had not answered. Ripening and the closing window are clocks the player can
//! compute exactly; rivals are a die they cannot. Scrutiny is neither — it is
//! the city answering the outfit's own habits, so the mark that is a bad idea
//! this month is a bad idea *because of what the crew did last month*.
//!
//! The counter-play is deliberately not a purchase. Heat can be bribed down; a
//! reputation for going through the wires cannot. It costs weeks of quiet or a
//! change of method, and those are the two things the week loop already
//! charges for.

use super::outcome::ModifierEntry;
use crate::model::Skill;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How fast a trade gets noticed, how hard the file bites, and how long it
/// takes to go cold. Every number here is balance and lives in
/// `game_config.json` under `scrutiny`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScrutinyTuning {
    /// Attention a door the crew got through adds to its trade. Getting in is
    /// what teaches the building how you got in.
    pub per_door_cleared: i32,
    /// Attention a door that beat them adds. Less, but never nothing: a crew
    /// who tried the wires and failed still tried the wires.
    pub per_door_failed: i32,
    /// Attention every trade sheds each week, worked or not.
    pub decay_per_week: i32,
    /// A file this thin costs nothing. One job never hardens a city.
    pub free_threshold: i32,
    /// Every this much attention above the threshold is one more point on
    /// every DC that trade opens.
    pub step: i32,
    /// The most the city will ever add to a single trade.
    pub max_penalty: i32,
}

impl Default for ScrutinyTuning {
    fn default() -> Self {
        Self {
            per_door_cleared: 3,
            per_door_failed: 1,
            decay_per_week: 2,
            free_threshold: 4,
            step: 6,
            max_penalty: 3,
        }
    }
}

impl ScrutinyTuning {
    /// The most attention one trade can carry. Capped at exactly the top of the
    /// curve rather than left to run away, so a maxed-out file cools on a
    /// schedule the player can count instead of sitting at an invisible
    /// ceiling for months.
    pub fn ceiling(&self) -> i32 {
        self.free_threshold + self.step * self.max_penalty
    }

    /// What a trade's file is worth on every check that uses it.
    pub fn penalty_for(&self, attention: i32) -> i32 {
        if self.step <= 0 || attention <= self.free_threshold {
            return 0;
        }
        ((attention - self.free_threshold) / self.step).min(self.max_penalty)
    }

    /// Weeks of quiet it takes a file at `attention` to cost one point *less*,
    /// for the board to quote. The number the player can act on is the next
    /// band, not the far end of the curve — "eleven weeks until it is gone" is
    /// not a plan, and "three weeks until it stops costing three" is.
    ///
    /// Returns `0` when the trade is not being charged for at all.
    pub fn weeks_to_relief(&self, attention: i32) -> u32 {
        let penalty = self.penalty_for(attention);
        if penalty <= 0 || self.decay_per_week <= 0 {
            return 0;
        }
        // The most attention that still reads one band lower.
        let target = self.free_threshold + self.step * penalty - 1;
        let shed = (attention - target).max(1);
        ((shed + self.decay_per_week - 1) / self.decay_per_week) as u32
    }
}

/// The city's file on each trade the outfit works in. Empty at week one and
/// empty again after enough quiet.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scrutiny {
    /// Ordered, not hashed: this is serialised into the save and read in a
    /// fixed order by the week, and hash order would make a campaign's replay
    /// depend on where the map felt like putting things (GDD 5.7).
    trades: BTreeMap<Skill, i32>,
}

impl Scrutiny {
    pub fn get(&self, skill: Skill) -> i32 {
        self.trades.get(&skill).copied().unwrap_or(0)
    }

    /// The city saw the crew work this trade.
    pub fn note(&mut self, skill: Skill, attention: i32, tuning: &ScrutinyTuning) {
        if attention <= 0 {
            return;
        }
        let next = (self.get(skill) + attention).min(tuning.ceiling());
        self.trades.insert(skill, next);
    }

    /// What the file costs on every door of that trade, anywhere on the board.
    pub fn penalty(&self, skill: Skill, tuning: &ScrutinyTuning) -> i32 {
        tuning.penalty_for(self.get(skill))
    }

    /// The named line the planning breakdown and the run both read, so what the
    /// player is shown before committing is what the die is added to
    /// afterwards (pillar 2).
    pub fn entry(&self, skill: Skill, tuning: &ScrutinyTuning) -> Option<ModifierEntry> {
        let penalty = self.penalty(skill, tuning);
        (penalty > 0).then(|| ModifierEntry::new(Self::label(skill), -penalty))
    }

    /// Kept short deliberately: this shares a column with a dozen other named
    /// modifiers on the planning breakdown, and a label that crowds its own
    /// value is a modifier the player has to squint at (pillar 2).
    pub fn label(skill: Skill) -> String {
        format!("Watched: {}", skill.label())
    }

    /// Every trade the city is currently charging for, worst first. Ties break
    /// on the skill's own order so the list never shuffles between frames.
    pub fn watched(&self, tuning: &ScrutinyTuning) -> Vec<(Skill, i32)> {
        let mut watched: Vec<(Skill, i32)> = self
            .trades
            .iter()
            .map(|(skill, attention)| (*skill, tuning.penalty_for(*attention)))
            .filter(|(_, penalty)| *penalty > 0)
            .collect();
        watched.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        watched
    }

    /// A week goes by and every file thins, whether the outfit worked or not.
    /// Returns the trades whose *penalty* actually came down, which is the only
    /// part of the thinning worth telling the player about.
    pub fn cool(&mut self, tuning: &ScrutinyTuning) -> Vec<Skill> {
        let decay = tuning.decay_per_week.max(0);
        let mut relieved = Vec::new();

        for (skill, attention) in &mut self.trades {
            let before = tuning.penalty_for(*attention);
            *attention = (*attention - decay).max(0);
            if tuning.penalty_for(*attention) < before {
                relieved.push(*skill);
            }
        }
        self.trades.retain(|_, attention| *attention > 0);
        relieved
    }

    pub fn is_empty(&self) -> bool {
        self.trades.values().all(|attention| *attention <= 0)
    }
}

#[cfg(test)]
mod tests;
