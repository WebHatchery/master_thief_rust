//! Getting out.
//!
//! GDD 12 settles open question 5 by making notoriety monotonic, on the grounds
//! that "the campaign is therefore finite by design" and the finite version is
//! the stronger game. It is monotonic. Nothing was ever finite: heat climbed,
//! bribes got dearer, hands got taken, and the week loop went on offering
//! another week for as long as anybody kept clicking.
//!
//! A heist story does not end with the city catching up. It ends with the crew
//! deciding they have enough — and being wrong about that is the whole genre.
//! So retirement is a verb rather than an event: the fixer chooses when, and the
//! choice is the campaign's shape. Every week worked is more money and more
//! notoriety, and notoriety never goes back down.
//!
//! What the outfit walks away with is its cash plus whatever the lockup fetches,
//! and the lockup is sold through the same fence the rest of the game uses — so
//! an outfit that gets out while the city is watching liquidates at the same bad
//! rate it would have got on any other Tuesday. Cooling off before you walk is
//! worth real money, which is the last decision the campaign asks for.
//!
//! No RNG: what you leave with is arithmetic, and the player can read it before
//! agreeing to it.

use crate::data::{GameConfig, GameData};
use crate::state::GameSession;
use macroquad_toolkit::ui::format_money;
use serde::{Deserialize, Serialize};

/// What the outfit walked away with, kept in the save so the records screen can
/// read out the reckoning afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retirement {
    pub week: u32,
    /// Cash in hand when they stopped.
    pub cash: i64,
    /// What the lockup fetched, at the fence rate the outfit's heat earned it.
    pub lockup: i64,
    /// Hands still on the payroll, who split nothing — they were paid weekly.
    pub crew: usize,
    /// Anybody the city was still holding. They do not come home.
    pub left_behind: Vec<String>,
    pub notoriety: i32,
}

impl Retirement {
    pub fn take(&self) -> i64 {
        self.cash + self.lockup
    }

    /// The line the records screen leads with.
    pub fn headline(&self) -> String {
        format!(
            "Out in week {} with {}",
            self.week,
            format_money(self.take())
        )
    }
}

/// What getting out today would be worth. Shown before the player commits to
/// it, because a campaign should not end on a number nobody saw coming.
pub fn quote(session: &GameSession, data: &GameData, config: &GameConfig) -> Retirement {
    // The lockup goes through the fence like everything else, which means a hot
    // outfit gets a hot outfit's price for its own tools.
    let share = super::fence::share_at(session.heat, &config.fence);
    let lockup: i64 = session
        .inventory
        .iter()
        .filter_map(|id| data.equipment.get(id))
        .map(|item| (item.cost as f32 * share) as i64)
        .sum();

    Retirement {
        week: session.week,
        cash: session.budget.max(0),
        lockup,
        crew: session.crew.len(),
        left_behind: session
            .custody
            .iter()
            .map(|record| record.member.name.clone())
            .collect(),
        notoriety: session.notoriety,
    }
}

/// Stop. Everything the outfit had becomes one number and the campaign is over.
pub fn retire(
    session: &mut GameSession,
    data: &GameData,
    config: &GameConfig,
) -> Result<String, String> {
    if session.retired.is_some() {
        return Err("The outfit is already out".to_owned());
    }

    let reckoning = quote(session, data, config);
    let headline = reckoning.headline();
    session.tally.final_take = reckoning.take();
    session.retired = Some(reckoning);
    Ok(headline)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(seed: u64) -> (GameData, GameSession) {
        let data = GameData::load().unwrap();
        let session = GameSession::new(&data.config, &data, seed);
        (data, session)
    }

    #[test]
    fn the_take_is_the_cash_and_what_the_lockup_fetches() {
        let (data, mut session) = setup(1);
        session.budget = 500_000;

        let quoted = quote(&session, &data, &data.config);
        assert_eq!(quoted.cash, 500_000);
        assert!(quoted.lockup > 0, "the starting kit is worth something");
        assert_eq!(quoted.take(), quoted.cash + quoted.lockup);
    }

    #[test]
    fn getting_out_while_the_city_is_watching_costs_real_money() {
        // The last decision the campaign asks for: cool off, then walk.
        let (data, mut session) = setup(2);
        session.budget = 100_000;

        session.heat = 0;
        let quiet = quote(&session, &data, &data.config).take();
        session.heat = 200;
        let hunted = quote(&session, &data, &data.config).take();

        assert!(
            hunted < quiet,
            "a hunted outfit liquidated at a calm outfit's price"
        );
    }

    #[test]
    fn retiring_ends_it_and_cannot_be_done_twice() {
        let (data, mut session) = setup(3);
        session.budget = 250_000;

        let first = retire(&mut session, &data, &data.config);
        assert!(first.is_ok());
        assert!(session.retired.is_some());
        assert_eq!(
            session.tally.final_take,
            session.retired.as_ref().unwrap().take()
        );

        assert!(retire(&mut session, &data, &data.config).is_err());
    }

    #[test]
    fn anybody_still_in_a_cell_is_named_and_stays_there() {
        let (data, mut session) = setup(4);
        session.heat = 90;
        crate::sim::law::tests_support_arrest(&mut session, &data.config);

        let quoted = quote(&session, &data, &data.config);
        assert_eq!(quoted.left_behind.len(), 1);
        assert!(!quoted.left_behind[0].is_empty());
    }

    #[test]
    fn the_reckoning_reads_as_a_sentence() {
        let (data, mut session) = setup(5);
        session.budget = 1_250_000;
        session.week = 31;

        let headline = retire(&mut session, &data, &data.config).unwrap();
        assert!(headline.contains("week 31"));
        assert!(headline.contains('$'));
    }
}
