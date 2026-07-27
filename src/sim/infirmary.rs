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
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::model::crew::Injury;

    fn setup(seed: u64) -> (GameData, GameSession) {
        let data = GameData::load().unwrap();
        let session = GameSession::new(&data.config, &data, seed);
        (data, session)
    }

    #[test]
    fn a_healthy_hand_is_quoted_nothing_and_cannot_be_treated() {
        let (data, mut session) = setup(1);
        let id = session.crew[0].id.clone();

        let quoted = quote(&session.crew[0].condition, &data.config.treatment);
        assert!(!quoted.is_needed());
        assert_eq!(quoted.cost, 0);

        let budget = session.budget;
        assert!(treat(&mut session, &data.config, &id).is_err());
        assert_eq!(session.budget, budget);
    }

    #[test]
    fn a_serious_injury_costs_more_than_a_scrape() {
        let (data, mut session) = setup(2);
        let config = &data.config.treatment;

        session.crew[0].condition.injuries = vec![Injury::minor("Bruised ribs")];
        let minor = quote(&session.crew[0].condition, config).cost;
        session.crew[0].condition.injuries = vec![Injury::major("Torn shoulder")];
        let major = quote(&session.crew[0].condition, config).cost;

        assert!(major > minor, "a torn shoulder billed like a bruise");
    }

    #[test]
    fn the_longer_the_wait_would_have_been_the_dearer_the_cure() {
        let (data, mut session) = setup(3);
        let config = &data.config.treatment;

        session.crew[0].condition.injuries = vec![Injury::major("Torn shoulder")];
        let fresh = quote(&session.crew[0].condition, config).cost;
        session.crew[0].condition.injuries[0].weeks_remaining = 1;
        let nearly_healed = quote(&session.crew[0].condition, config).cost;

        assert!(
            nearly_healed < fresh,
            "paying to skip one week cost the same as skipping three"
        );
    }

    #[test]
    fn treatment_clears_everything_and_leaves_them_tired() {
        let (data, mut session) = setup(4);
        let id = session.crew[0].id.clone();
        session.crew[0].condition.injuries =
            vec![Injury::minor("Bruised ribs"), Injury::major("Bad hand")];
        session.crew[0].condition.fatigue = 20;
        session.budget = 5_000_000;

        let quoted = quote(&session.crew[0].condition, &data.config.treatment);
        assert!(treat(&mut session, &data.config, &id).is_ok());

        let member = session.member(&id).unwrap();
        assert!(member.condition.injuries.is_empty());
        assert_eq!(
            member.condition.fatigue,
            20 + data.config.treatment.fatigue_cost,
            "a body put back together in an afternoon knows it"
        );
        assert_eq!(session.budget, 5_000_000 - quoted.cost);
        assert_eq!(session.tally.injuries_treated, 2);
    }

    #[test]
    fn a_treatment_nobody_can_afford_leaves_them_hurt() {
        let (data, mut session) = setup(5);
        let id = session.crew[0].id.clone();
        session.crew[0].condition.injuries = vec![Injury::major("Torn shoulder")];
        session.budget = 0;

        assert!(treat(&mut session, &data.config, &id).is_err());
        assert_eq!(session.member(&id).unwrap().condition.injuries.len(), 1);
        assert_eq!(session.budget, 0);
    }

    #[test]
    fn treatment_puts_an_unfit_hand_back_to_work_the_same_week() {
        // The point of the verb: an injured hand was a fact to be waited out.
        // Now it is a bill, and paying it buys the week back.
        let (data, mut session) = setup(6);
        let id = session.crew[0].id.clone();
        session.crew[0].condition.injuries = vec![
            Injury::major("Torn shoulder"),
            Injury::major("Cracked rib"),
            Injury::minor("Sprain"),
        ];
        assert!(!session.member(&id).unwrap().condition.is_fit_for_work());

        session.budget = 5_000_000;
        assert!(treat(&mut session, &data.config, &id).is_ok());
        assert!(session.member(&id).unwrap().condition.is_fit_for_work());
    }
}
