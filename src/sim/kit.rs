//! Kit that wears out, and what it costs to put right.
//!
//! GDD 3 lists the outfit step as "buy, assign, and *repair* equipment" and
//! GDD 9 gives the shop "purchase, repair, assignment". Repair never existed,
//! because nothing ever wore out: a tool bought in week two was exactly as good
//! in week forty. That made the Outfitter the one screen in the game with no
//! recurring decision on it — a shop you visit once.
//!
//! Kit now takes a job's worth of wear every time it goes through a door, and
//! worn kit reads as a named penalty on every check the hand makes. Refitting
//! is a bill. The decision it creates is the ordinary one every working outfit
//! has: keep the good tool serviced, or run it down and replace it.
//!
//! No RNG: wear is counted, not rolled.

use crate::data::{GameConfig, KitConfig};
use crate::model::CrewMember;
use crate::state::GameSession;
use macroquad_toolkit::ui::format_money;

/// What refitting one hand's kit would cost, and what it would give back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Refit {
    pub cost: i64,
    /// Pieces of kit that have any wear on them at all.
    pub pieces: usize,
    /// The check penalty the refit would clear.
    pub penalty_cleared: i32,
}

impl Refit {
    pub fn is_needed(&self) -> bool {
        self.pieces > 0
    }
}

/// What one piece of kit at this much wear costs on a check.
fn penalty_for(wear: u32, config: &KitConfig) -> i32 {
    if config.jobs_per_penalty == 0 {
        return 0;
    }
    ((wear / config.jobs_per_penalty) as i32).min(config.max_penalty_per_item)
}

/// The penalty this hand carries for the state of their tools. Named on the
/// check like everything else, so a player can see the shop bill in the dice
/// before they pay it (pillar 2).
pub fn wear_penalty(session: &GameSession, member: &CrewMember, config: &KitConfig) -> i32 {
    member
        .equipment
        .item_ids()
        .map(|id| penalty_for(session.kit_wear.get(id).copied().unwrap_or(0), config))
        .sum()
}

/// One job's use, applied to everything a hand was carrying. Called once per
/// job per member, not once per door — a night out is a night out.
pub fn wear_kit(session: &mut GameSession, member_id: &str) {
    let carried: Vec<String> = match session.member(member_id) {
        Some(member) => member
            .equipment
            .item_ids()
            .map(|id| id.to_owned())
            .collect(),
        None => return,
    };
    for id in carried {
        *session.kit_wear.entry(id).or_insert(0) += 1;
    }
}

/// What it would take to put this hand's tools back in order.
pub fn refit_quote(session: &GameSession, member: &CrewMember, config: &KitConfig) -> Refit {
    let mut refit = Refit::default();
    for id in member.equipment.item_ids() {
        let wear = session.kit_wear.get(id).copied().unwrap_or(0);
        if wear == 0 {
            continue;
        }
        refit.pieces += 1;
        refit.cost += config.refit_base + config.refit_per_job * wear as i64;
        refit.penalty_cleared += penalty_for(wear, config);
    }
    refit
}

/// Pay for it. Everything the hand is carrying comes back to as-new.
pub fn refit(
    session: &mut GameSession,
    config: &GameConfig,
    member_id: &str,
) -> Result<String, String> {
    let Some(member) = session.member(member_id) else {
        return Err("They are not on the payroll".to_owned());
    };
    let quoted = refit_quote(session, member, &config.kit);
    let name = member.name.clone();
    let carried: Vec<String> = member
        .equipment
        .item_ids()
        .map(|id| id.to_owned())
        .collect();

    if !quoted.is_needed() {
        return Err(format!("{}'s kit is in good order", name));
    }
    if session.budget < quoted.cost {
        return Err(format!(
            "Refitting {} costs {}",
            name,
            format_money(quoted.cost)
        ));
    }

    session.budget -= quoted.cost;
    session.tally.refits_paid += quoted.cost;
    for id in carried {
        session.kit_wear.remove(&id);
    }

    Ok(format!(
        "{}'s kit refitted for {}",
        name,
        format_money(quoted.cost)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;

    fn setup(seed: u64) -> (GameData, GameSession) {
        let data = GameData::load().unwrap();
        let session = GameSession::new(&data.config, &data, seed);
        (data, session)
    }

    fn carrier(session: &GameSession) -> String {
        session
            .crew
            .iter()
            .find(|member| member.equipment.item_ids().count() > 0)
            .expect("somebody carries the starting kit")
            .id
            .clone()
    }

    #[test]
    fn new_kit_costs_nothing_and_carries_no_penalty() {
        let (data, session) = setup(1);
        let member = session.member(&carrier(&session)).unwrap();

        assert_eq!(wear_penalty(&session, member, &data.config.kit), 0);
        assert!(!refit_quote(&session, member, &data.config.kit).is_needed());
    }

    #[test]
    fn kit_wears_a_job_at_a_time_and_eventually_bites() {
        let (data, mut session) = setup(2);
        let id = carrier(&session);
        let config = &data.config.kit;

        for _ in 0..config.jobs_per_penalty {
            wear_kit(&mut session, &id);
        }

        let member = session.member(&id).unwrap();
        assert_eq!(wear_penalty(&session, member, config), 1);
        assert!(refit_quote(&session, member, config).is_needed());
    }

    #[test]
    fn no_single_tool_can_ruin_a_hand_on_its_own() {
        let (data, mut session) = setup(3);
        let id = carrier(&session);
        let config = &data.config.kit;

        for _ in 0..config.jobs_per_penalty * (config.max_penalty_per_item as u32 + 10) {
            wear_kit(&mut session, &id);
        }

        let member = session.member(&id).unwrap();
        let carried = member.equipment.item_ids().count() as i32;
        assert_eq!(
            wear_penalty(&session, member, config),
            carried * config.max_penalty_per_item
        );
    }

    #[test]
    fn a_refit_clears_the_penalty_and_the_money_is_gone() {
        let (data, mut session) = setup(4);
        let id = carrier(&session);
        for _ in 0..data.config.kit.jobs_per_penalty * 2 {
            wear_kit(&mut session, &id);
        }
        session.budget = 5_000_000;

        let member = session.member(&id).unwrap();
        let quoted = refit_quote(&session, member, &data.config.kit);
        assert!(quoted.penalty_cleared > 0);

        assert!(refit(&mut session, &data.config, &id).is_ok());
        let member = session.member(&id).unwrap();
        assert_eq!(wear_penalty(&session, member, &data.config.kit), 0);
        assert_eq!(session.budget, 5_000_000 - quoted.cost);
    }

    #[test]
    fn the_longer_it_is_left_the_dearer_the_refit() {
        let (data, mut session) = setup(5);
        let id = carrier(&session);

        for _ in 0..3 {
            wear_kit(&mut session, &id);
        }
        let early = refit_quote(&session, session.member(&id).unwrap(), &data.config.kit).cost;
        for _ in 0..10 {
            wear_kit(&mut session, &id);
        }
        let late = refit_quote(&session, session.member(&id).unwrap(), &data.config.kit).cost;

        assert!(late > early, "neglect was free");
    }

    #[test]
    fn a_refit_nobody_can_afford_leaves_the_kit_worn() {
        let (data, mut session) = setup(6);
        let id = carrier(&session);
        for _ in 0..20 {
            wear_kit(&mut session, &id);
        }
        session.budget = 0;

        assert!(refit(&mut session, &data.config, &id).is_err());
        assert!(wear_penalty(&session, session.member(&id).unwrap(), &data.config.kit) > 0);
    }

    #[test]
    fn kit_nobody_is_carrying_does_not_wear() {
        let (data, mut session) = setup(7);
        let bare = session
            .crew
            .iter()
            .find(|member| member.equipment.item_ids().count() == 0)
            .map(|member| member.id.clone());

        if let Some(id) = bare {
            wear_kit(&mut session, &id);
            assert_eq!(
                wear_penalty(&session, session.member(&id).unwrap(), &data.config.kit),
                0
            );
        }
        assert!(session.kit_wear.values().all(|wear| *wear > 0));
    }
}
