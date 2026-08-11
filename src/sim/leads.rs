//! Work the crew bring in themselves.
//!
//! Loyalty does three things, and all three of them are threats: it moves the
//! die, it decides who gives notice, and it decides who holds out for a bigger
//! cut. So the only reason to keep a hand happy is to stop something bad, and
//! "stop something bad" is a weaker pull than it looks — a fixer with a thin
//! week will always find something more urgent than goodwill.
//!
//! A tip-off is the other direction. A hand who is genuinely content hears
//! things: a cousin who works nights somewhere, a room somebody mentioned. The
//! mark arrives off the board, with part of its file already written, because
//! the person who brought it knows the place. It is the one thing in the week
//! that a good roster *generates* rather than merely survives.
//!
//! It also makes the board partly a function of who the outfit employs, where
//! before it was entirely a function of the seed.
//!
//! One draw per week from the session RNG, at a fixed point in
//! [`crate::sim::advance_week`] (GDD 5.7).

use crate::data::{GameData, LeadConfig};
use crate::state::{BoardEntry, GameSession};

/// A mark somebody on the payroll brought in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lead {
    /// Who heard about it.
    pub finder: String,
    pub target_id: String,
    pub target_name: String,
    /// Doors already on the file, because they know the place.
    pub doors_on_file: u32,
}

impl Lead {
    pub fn headline(&self) -> String {
        if self.doors_on_file > 0 {
            format!(
                "{} brought in {} — {} doors already on the file",
                self.finder, self.target_name, self.doors_on_file
            )
        } else {
            format!("{} brought in {}", self.finder, self.target_name)
        }
    }
}

/// The chance somebody brings something in this week. Every contented hand is
/// another set of ears, with a ceiling so a large happy crew does not simply
/// print opportunities.
pub fn chance_of_a_lead(session: &GameSession, config: &LeadConfig) -> f32 {
    let ears = session
        .crew
        .iter()
        .filter(|member| member.condition.loyalty >= config.loyalty_threshold)
        .count();
    if ears == 0 {
        return 0.0;
    }
    (ears as f32 * config.chance_per_hand).min(config.max_chance)
}

/// Roll for a tip-off, and put it on the board if one comes in.
pub fn roll_lead(session: &mut GameSession, data: &GameData) -> Option<Lead> {
    let config = &data.config.leads;
    let chance = chance_of_a_lead(session, config);
    if chance <= 0.0 || session.rng.next_f32() >= chance {
        return None;
    }

    // Only marks the outfit's name already opens, and only ones nobody is
    // already looking at. Sorted: registry order is not stable (GDD 5.7).
    let mut candidates: Vec<String> = session
        .eligible_targets(data)
        .into_iter()
        .map(|target| target.id.clone())
        .filter(|id| !session.board.iter().any(|entry| &entry.target_id == id))
        .collect();
    candidates.sort();
    if candidates.is_empty() {
        return None;
    }

    // Whoever is happiest hears it first; the id breaks ties so two equally
    // contented hands always resolve the same way.
    let finder = session
        .crew
        .iter()
        .filter(|member| member.condition.loyalty >= config.loyalty_threshold)
        .max_by(|a, b| {
            a.condition
                .loyalty
                .cmp(&b.condition.loyalty)
                .then(b.id.cmp(&a.id))
        })?
        .name
        .clone();

    let target_id = candidates[session.rng.below(candidates.len())].clone();
    let target_name = data
        .targets
        .get(&target_id)
        .map(|target| target.name.clone())
        .unwrap_or_else(|| target_id.clone());
    let doors = data
        .targets
        .get(&target_id)
        .map(|target| target.encounters.len() as u32)
        .unwrap_or(0);
    let on_file = config.doors_on_file.min(doors);

    let mut entry = BoardEntry::new(target_id.clone());
    entry.casing = on_file;
    entry.weeks_remaining += config.extra_weeks;
    session.board.push(entry);
    session.tally.leads_brought_in += 1;

    Some(Lead {
        finder,
        target_id,
        target_name,
        doors_on_file: on_file,
    })
}

#[cfg(test)]
mod tests;
