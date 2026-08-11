//! Paying to put somebody back together.
//!
//! GDD 3 and 4 both name *treat* as one of the fixer's verbs, and the game
//! never had it: an injury cleared on its own schedule and nothing the player
//! did changed that. So a hurt hand was a fact to be waited out, not a decision.
//!
//! Treatment converts money into time, which is the trade the rest of the week
//! is already built on — waiting costs wages (5.6), lets marks ripen out of
//! reach (5.4), and lets rivals take them. A doctor is the way to buy the weeks
//! back, and the bill scales with how long the wait would have been, so the
//! injuries worth paying to fix are exactly the ones that would have cost most.
//!
//! No RNG: what treatment costs is arithmetic the player can read before they
//! agree to it.

use crate::data::{GameConfig, TreatmentConfig};
use crate::model::crew::{Condition, InjurySeverity};
use crate::state::GameSession;
use macroquad_toolkit::ui::format_money;

/// What patching one hand up would cost, and what it would buy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Treatment {
    pub cost: i64,
    pub injuries: usize,
    /// The longest any of them still had to run. This is the number the player
    /// is actually buying.
    pub weeks_saved: u32,
}

impl Treatment {
    pub fn is_needed(&self) -> bool {
        self.injuries > 0
    }
}

/// The bill for one injury: a call-out fee plus the weeks it would otherwise
/// have taken, and twice that for anything serious.
fn cost_of(severity: InjurySeverity, weeks_remaining: u32, config: &TreatmentConfig) -> i64 {
    let weeks = config.base_cost + config.cost_per_week * weeks_remaining as i64;
    match severity {
        InjurySeverity::Minor => weeks,
        InjurySeverity::Major => weeks * config.major_multiplier.max(1),
    }
}

/// What it would take to get this hand back on their feet today.
pub fn quote(condition: &Condition, config: &TreatmentConfig) -> Treatment {
    Treatment {
        cost: condition
            .injuries
            .iter()
            .map(|injury| cost_of(injury.severity, injury.weeks_remaining, config))
            .sum(),
        injuries: condition.injuries.len(),
        weeks_saved: condition
            .injuries
            .iter()
            .map(|injury| injury.weeks_remaining)
            .max()
            .unwrap_or(0),
    }
}

/// Pay for it. Clears every active injury at once — the fixer is buying a hand
/// back, not haggling over which bone — and leaves them tired rather than
/// fresh, because a body put back together in an afternoon knows it.
pub fn treat(
    session: &mut GameSession,
    config: &GameConfig,
    member_id: &str,
) -> Result<String, String> {
    let treatment = &config.treatment;
    let Some(member) = session.member(member_id) else {
        return Err("They are not on the payroll".to_owned());
    };
    let quoted = quote(&member.condition, treatment);
    let name = member.name.clone();

    if !quoted.is_needed() {
        return Err(format!("{} has nothing that needs a doctor", name));
    }
    if session.budget < quoted.cost {
        return Err(format!(
            "Patching {} up costs {}",
            name,
            format_money(quoted.cost)
        ));
    }

    session.budget -= quoted.cost;
    session.tally.treatment_paid += quoted.cost;
    session.tally.injuries_treated += quoted.injuries as i64;

    if let Some(member) = session.member_mut(member_id) {
        member.condition.injuries.clear();
        member.condition.add_fatigue(treatment.fatigue_cost);
    }

    Ok(format!(
        "{} patched up for {} — {} weeks bought back",
        name,
        format_money(quoted.cost),
        quoted.weeks_saved
    ))
}

#[cfg(test)]
mod tests;
