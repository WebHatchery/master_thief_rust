//! Crew chemistry: who works well with whom, and who will not work at all.
//!
//! The original tracked a `characterRelationships` field that nothing ever
//! wrote — three `// TODO: Implement full relationship system` sites and a dead
//! column in the save. GDD 5.5 says build it properly or delete the field; this
//! is the building. Chemistry moves on shared outcomes, personality sets the
//! rate, and the result is a small named modifier when two hands are on the same
//! job — enough to make the roster a composition problem rather than a sum.

use super::outcome::{ModifierEntry, Outcome};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The scale chemistry lives on. Beyond the refusal floor a pair will not take
/// the same job.
pub const MAX: i32 = 100;
pub const MIN: i32 = -100;
/// At or below this, the pair refuses to work together.
pub const REFUSAL: i32 = -60;
/// At or above this, a pair has become a unit — they read each other on a job,
/// and they know it when the cut is discussed (GDD 5.5).
///
/// Set against what play actually produces, not against the nominal ±100
/// scale: a campaign's warmest pair lands in the teens or twenties, because
/// every week apart cools them. A threshold of forty was one no crew ever
/// reached, which made the whole tier decoration.
pub const PARTNERSHIP: i32 = 20;
/// How many points of chemistry buy one point on a check. Same reasoning: at
/// twenty-five, the modifier this system exists to produce almost never fired.
const POINTS_PER_MODIFIER: i32 = 12;
/// The most chemistry can swing a single check, either way.
const MODIFIER_CAP: i32 = 3;

/// A symmetric table of pair chemistry, stored one entry per unordered pair.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chemistry {
    pairs: BTreeMap<String, i32>,
}

fn key(a: &str, b: &str) -> String {
    if a <= b {
        format!("{}|{}", a, b)
    } else {
        format!("{}|{}", b, a)
    }
}

impl Chemistry {
    pub fn get(&self, a: &str, b: &str) -> i32 {
        if a == b {
            return 0;
        }
        self.pairs.get(&key(a, b)).copied().unwrap_or(0)
    }

    pub fn set(&mut self, a: &str, b: &str, value: i32) {
        if a == b {
            return;
        }
        self.pairs.insert(key(a, b), value.clamp(MIN, MAX));
    }

    pub fn adjust(&mut self, a: &str, b: &str, delta: i32) {
        let next = self.get(a, b) + delta;
        self.set(a, b, next);
    }

    /// Will these two take the same job?
    pub fn refuses(&self, a: &str, b: &str) -> bool {
        self.get(a, b) <= REFUSAL
    }

    /// Anyone on the crew this member will not stand beside.
    pub fn refusals<'a>(&self, member: &str, crew: impl Iterator<Item = &'a str>) -> Vec<String> {
        crew.filter(|other| *other != member && self.refuses(member, other))
            .map(|other| other.to_owned())
            .collect()
    }

    /// The named modifier one member carries for the company they are keeping
    /// on this job. Averaged, so a big crew does not stack into a landslide.
    pub fn modifier(&self, member: &str, others: &[String]) -> Option<ModifierEntry> {
        let scores: Vec<i32> = others
            .iter()
            .filter(|other| other.as_str() != member)
            .map(|other| self.get(member, other))
            .collect();

        if scores.is_empty() {
            return None;
        }

        let average = scores.iter().sum::<i32>() / scores.len() as i32;
        let value = (average / POINTS_PER_MODIFIER).clamp(-MODIFIER_CAP, MODIFIER_CAP);
        (value != 0).then(|| {
            ModifierEntry::new(
                if value > 0 {
                    "Crew rapport"
                } else {
                    "Crew friction"
                },
                value,
            )
        })
    }

    /// Have these two become a unit?
    pub fn is_partnership(&self, a: &str, b: &str) -> bool {
        self.get(a, b) >= PARTNERSHIP
    }

    /// Established pairs among the hands down for one job, each counted once.
    /// This is what the cut is negotiated against: a pair who work as one know
    /// what a pair is worth.
    pub fn partnerships_among(&self, crew: &[String]) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        for (index, a) in crew.iter().enumerate() {
            for b in crew.iter().skip(index + 1) {
                if self.is_partnership(a, b) {
                    pairs.push((a.clone(), b.clone()));
                }
            }
        }
        pairs
    }

    /// A week apart. Warmth and grudges both fade toward indifference when a
    /// pair is not put in the same building — chemistry the fixer wants has to
    /// be kept up, and a grudge they can wait out will cool on its own.
    pub fn cool_off(&mut self, worked_together: &[String], amount: i32) {
        if amount <= 0 {
            return;
        }
        let mut worked: Vec<String> = Vec::new();
        for (index, a) in worked_together.iter().enumerate() {
            for b in worked_together.iter().skip(index + 1) {
                worked.push(key(a, b));
            }
        }

        for (pair, value) in self.pairs.iter_mut() {
            if worked.contains(pair) {
                continue;
            }
            *value -= value.signum() * amount.min(value.abs());
        }
        self.pairs.retain(|_, value| *value != 0);
    }

    /// Every pair the table has an opinion about, for the crew screen.
    pub fn known_pairs(&self) -> impl Iterator<Item = (&str, &str, i32)> {
        self.pairs.iter().filter_map(|(pair, value)| {
            let (a, b) = pair.split_once('|')?;
            Some((a, b, *value))
        })
    }

    /// Drop anyone who has left the payroll, so a retired hand's grudges do not
    /// outlive them in the save.
    pub fn retain_crew(&mut self, crew: &[String]) {
        self.pairs.retain(|pair, _| {
            pair.split_once('|')
                .map(|(a, b)| crew.iter().any(|id| id == a) && crew.iter().any(|id| id == b))
                .unwrap_or(false)
        });
    }
}

/// How hard each personality takes what it just watched, keyed by trait.
///
/// Authored in `assets/data/traits.json` rather than matched in Rust. It began
/// as a `match` on trait names, which drifted from the roster the moment either
/// side changed: six of the thirty traits the characters actually carry —
/// blunt, curious, ruthless, stubborn, superstitious, unpredictable — matched
/// no arm and fell through to 1.0, so those hands reacted to nothing in
/// particular while appearing to have a personality. A table the content test
/// can check against cannot drift that way.
///
/// The bands are: hands who work to a method barely re-read a bad night (0.7);
/// hands who take everything personally take this personally too (1.4);
/// expecting the worst blunts it when it arrives (0.85); warm people warm
/// faster and cool faster (1.15).
pub type TraitRates = std::collections::BTreeMap<String, f32>;

/// The rate a watcher reacts at, given everything they are. Traits multiply, so
/// a steady cynic moves less than either alone.
pub fn reaction_rate(traits: &[String], rates: &TraitRates) -> f32 {
    let mut rate: f32 = 1.0;
    for name in traits {
        if let Some(found) = rates.get(&name.to_ascii_lowercase()) {
            rate *= found;
        }
    }
    rate.clamp(0.4, 2.0)
}

/// What watching one door does to a pair, before the watcher's personality is
/// applied. Succeeding together raises it; watching a partner blow it lowers it.
pub fn shared_outcome_delta(outcome: Outcome) -> i32 {
    match outcome {
        Outcome::CriticalSuccess => 4,
        Outcome::Success => 2,
        Outcome::Neutral => 0,
        Outcome::Failure => -2,
        Outcome::CriticalFailure => -5,
    }
}

/// Apply one door's outcome across the pairs that watched it happen.
pub fn record_outcome(
    chemistry: &mut Chemistry,
    actor: &str,
    watchers: &[(String, Vec<String>)],
    outcome: Outcome,
    rates: &TraitRates,
) {
    let base = shared_outcome_delta(outcome);
    if base == 0 {
        return;
    }

    for (watcher, traits) in watchers {
        if watcher == actor {
            continue;
        }
        let delta = (base as f32 * reaction_rate(traits, rates)).round() as i32;
        chemistry.adjust(actor, watcher, delta);
    }
}

#[cfg(test)]
mod partnership_tests;

#[cfg(test)]
mod tests;
