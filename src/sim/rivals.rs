//! The other outfits working the same city.
//!
//! Ripening (GDD 5.4) made waiting on a mark a bet against harder doors and a
//! closing window. Both of those are clocks the player can read exactly, which
//! makes the bet arithmetic rather than a gamble. Rivals are the part that
//! cannot be read exactly: the crew are not the only people in the city who can
//! see a job getting fatter, and a mark left to ripen is a mark somebody else is
//! also watching.
//!
//! Nothing here is a new content axis — a rival is a name and a roll. What it
//! adds is a reason to take a job *now* that is not the payroll.
//!
//! The draw comes from the session's RNG at a fixed point in
//! [`crate::sim::advance_week`], so a seed still replays exactly (GDD 5.7).

use crate::data::{GameData, RivalConfig};
use crate::state::{BoardEntry, GameSession};

/// A mark taken out from under the crew, described for the week summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RivalJob {
    /// Who got there first.
    pub rival: String,
    pub target_id: String,
    pub target_name: String,
    /// How ripe the mark was when it went. The player's own hesitation, in a
    /// number.
    pub ripeness: u32,
    /// Doors the crew had already paid to put on a file, now worth nothing.
    pub casing_wasted: u32,
}

impl RivalJob {
    pub fn headline(&self) -> String {
        if self.casing_wasted > 0 {
            format!(
                "{} took {} — {} doors of file work wasted",
                self.rival, self.target_name, self.casing_wasted
            )
        } else {
            format!(
                "{} took {} out from under you",
                self.rival, self.target_name
            )
        }
    }
}

/// How likely somebody else is to take this mark before the week is out. Shown
/// on the board so waiting is a risk the player accepts rather than one sprung
/// on them (pillar 2).
pub fn interest_in(entry: &BoardEntry, config: &RivalConfig) -> f32 {
    if entry.ripeness < config.min_ripeness {
        return 0.0;
    }
    let over = entry.ripeness - config.min_ripeness;
    (config.base_chance + over as f32 * config.chance_per_ripeness).min(config.max_chance)
}

/// Roll the week's competition. At most one mark goes: losing the board to
/// rivals in a single week would be a different and much worse game.
pub fn roll_rivals(session: &mut GameSession, data: &GameData) -> Option<RivalJob> {
    let rivals = &data.config.rivals;
    if rivals.names.is_empty() {
        return None;
    }

    // Board order, so the draw sequence depends only on state the seed decides.
    for index in 0..session.board.len() {
        let chance = interest_in(&session.board[index], rivals);
        if chance <= 0.0 {
            continue;
        }
        if session.rng.next_f32() >= chance {
            continue;
        }

        let name_index = session.rng.below(rivals.names.len());
        let entry = session.board.remove(index);
        let target_name = data
            .targets
            .get(&entry.target_id)
            .map(|target| target.name.clone())
            .unwrap_or_else(|| entry.target_id.clone());

        session.tally.marks_lost_to_rivals += 1;
        return Some(RivalJob {
            rival: rivals.names[name_index].clone(),
            target_id: entry.target_id,
            target_name,
            ripeness: entry.ripeness,
            casing_wasted: entry.casing,
        });
    }
    None
}

#[cfg(test)]
mod tests;
