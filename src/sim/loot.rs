//! What the crew carries out besides the money.
//!
//! `possible_loot` was declared on every target in the original and never
//! rolled — one of the two `// TODO` holes GDD 0 marks as **Build**. Draws come
//! from the run's seeded RNG in a fixed order, so a seed and a plan still
//! reproduce a campaign exactly.

use crate::data::GameData;
use crate::model::HeistTarget;
use crate::rules::Outcome;
use macroquad_toolkit::rng::SeededRng;

/// Roll what a finished job leaves behind. `outcomes` is every door in the
/// order it resolved; the job is only worth searching if it worked.
pub fn roll_loot(
    rng: &mut SeededRng,
    data: &GameData,
    target: &HeistTarget,
    outcomes: &[Outcome],
    success: bool,
) -> Vec<String> {
    let pool: Vec<&String> = target
        .possible_loot
        .iter()
        .filter(|id| data.equipment.contains(id))
        .collect();

    if pool.is_empty() || !success {
        return Vec::new();
    }

    let config = &data.config.payout;
    let mut found = Vec::new();
    if rng.chance(config.loot_chance) {
        found.push(pool[rng.below(pool.len())].clone());
    }

    // A door taken brilliantly is where the unexpected thing turns up.
    for outcome in outcomes {
        if *outcome == Outcome::CriticalSuccess && rng.chance(config.loot_chance_per_critical) {
            found.push(pool[rng.below(pool.len())].clone());
        }
    }

    found
}

#[cfg(test)]
mod tests;
