//! The board: which marks are on offer, what the crew know about them, and what
//! a week of not taking one does to them.
//!
//! Split out of `state.rs` on its own responsibility — a mark's file, its
//! window, and its ripening are one subject, and the ledger was carrying them
//! alongside everything else a campaign remembers.

use super::GameSession;
use crate::data::{BoardConfig, GameConfig, GameData};
use crate::model::HeistTarget;
use serde::{Deserialize, Serialize};

/// One mark on the board, and how much the crew has bothered to learn about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardEntry {
    pub target_id: String,
    /// How many of this mark's doors the crew has actually put on the file,
    /// front to back. Casing is bought a door at a time, so a mark can be half
    /// known — the front hall scouted and the vault still a rumour.
    pub casing: u32,
    /// Weeks this mark stays on the board before the window closes.
    pub weeks_remaining: u32,
    /// Weeks the crew has left this one sitting. A mark nobody takes ripens:
    /// the score grows and so does what is standing between them and it.
    #[serde(default)]
    pub ripeness: u32,
}

impl BoardEntry {
    pub fn new(target_id: impl Into<String>) -> Self {
        Self {
            target_id: target_id.into(),
            casing: 0,
            weeks_remaining: 4,
            ripeness: 0,
        }
    }

    /// Is this door's difficulty on the file? Doors are scouted front to back:
    /// the crew learn the way in before they learn the way to the vault.
    pub fn knows_door(&self, index: usize) -> bool {
        index < self.casing as usize
    }

    pub fn is_fully_cased(&self, doors: usize) -> bool {
        self.casing as usize >= doors
    }

    /// Nobody has looked at this one at all.
    pub fn is_blind(&self) -> bool {
        self.casing == 0
    }

    /// What the next door on the file costs. The front hall is cheap; every
    /// door after it is deeper into a building somebody is watching.
    pub fn next_casing_cost(&self, config: &GameConfig) -> i64 {
        config.casing_cost + config.casing_cost_step * self.casing as i64
    }

    /// What sitting on this mark has added to its payout, as a percentage.
    pub fn payout_bonus_pct(&self, config: &BoardConfig) -> i64 {
        self.ripeness.min(config.ripeness_max) as i64 * config.ripeness_payout_pct
    }

    /// What sitting on it has added to every door, as a penalty on the check.
    pub fn door_penalty(&self, config: &BoardConfig) -> i32 {
        self.ripeness.min(config.ripeness_max) as i32 * config.ripeness_door_penalty
    }

    /// The payout this mark is currently worth, ripening included.
    pub fn ripened_payout(&self, base: i64, config: &BoardConfig) -> i64 {
        base + base * self.payout_bonus_pct(config) / 100
    }
}

impl GameSession {
    /// Marks the crew's reputation has opened up.
    pub fn eligible_targets<'a>(&self, data: &'a GameData) -> Vec<&'a HeistTarget> {
        let mut targets: Vec<&HeistTarget> = data
            .targets
            .iter()
            .map(|(_, target)| target)
            .filter(|target| target.required_reputation <= self.reputation)
            .collect();
        // Sorted for the same reason the recruit pool is: the board draws from
        // this list with the run's RNG, and registry order is not stable.
        targets.sort_by_key(|target| (target.required_reputation, target.id.clone()));
        targets
    }

    /// Fill the board up to the configured size with marks the crew can take,
    /// drawing in a fixed order from the run's RNG.
    pub fn refresh_board(&mut self, config: &GameConfig, data: &GameData) {
        let eligible: Vec<String> = self
            .eligible_targets(data)
            .into_iter()
            .map(|target| target.id.clone())
            .filter(|id| !self.board.iter().any(|entry| &entry.target_id == id))
            .collect();

        let mut pool = eligible;
        while self.board.len() < config.targets_on_board && !pool.is_empty() {
            let index = self.rng.below(pool.len());
            self.board.push(BoardEntry::new(pool.remove(index)));
        }
    }

    /// Age the board by a week: every mark left sitting ripens by one step and
    /// loses a week of its window. Marks whose window has closed come off.
    pub fn age_board(&mut self) {
        for entry in &mut self.board {
            entry.weeks_remaining = entry.weeks_remaining.saturating_sub(1);
            entry.ripeness += 1;
        }
        self.board.retain(|entry| entry.weeks_remaining > 0);
    }

    pub fn board_entry(&self, target_id: &str) -> Option<&BoardEntry> {
        self.board.iter().find(|entry| entry.target_id == target_id)
    }

    /// What the city's file on its own trades costs this mark, door by door, for
    /// the board to quote before the player commits to anything (pillar 2).
    /// Doors the crew has not scouted are counted all the same — the outfit
    /// knows its own reputation even where it does not know the building.
    pub fn watched_doors(&self, target: &HeistTarget, data: &GameData) -> Vec<(usize, i32)> {
        data.encounters_for(target)
            .into_iter()
            .enumerate()
            .filter_map(|(index, encounter)| {
                let penalty = self
                    .scrutiny
                    .penalty(encounter.primary_skill, &data.config.scrutiny);
                (penalty > 0).then_some((index, penalty))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Skill;

    fn data() -> GameData {
        GameData::load().unwrap()
    }

    #[test]
    fn the_board_only_offers_marks_the_crews_name_can_open() {
        let data = data();
        let session = GameSession::new(&data.config, &data, 7);

        for entry in &session.board {
            let target = data.targets.get(&entry.target_id).unwrap();
            assert!(target.required_reputation <= session.reputation);
        }
    }

    #[test]
    fn reputation_opens_new_marks() {
        let data = data();
        let mut session = GameSession::new(&data.config, &data, 7);
        let early = session.eligible_targets(&data).len();

        session.reputation = 100;
        assert!(session.eligible_targets(&data).len() > early);
    }

    #[test]
    fn the_same_seed_lays_out_the_same_board() {
        let data = data();
        let a = GameSession::new(&data.config, &data, 20260726);
        let b = GameSession::new(&data.config, &data, 20260726);

        let ids_a: Vec<&str> = a.board.iter().map(|e| e.target_id.as_str()).collect();
        let ids_b: Vec<&str> = b.board.iter().map(|e| e.target_id.as_str()).collect();
        assert_eq!(ids_a, ids_b);
    }

    #[test]
    fn a_mark_left_sitting_is_worth_more_and_costs_more() {
        // The board used to be a stock list: waiting changed nothing, so there
        // was no reason not to take the best mark the moment it appeared.
        let data = data();
        let mut session = GameSession::new(&data.config, &data, 31);
        let board = &data.config.board;
        let entry = session.board[0].clone();
        let base = data.targets.get(&entry.target_id).unwrap().potential_payout;

        assert_eq!(entry.payout_bonus_pct(board), 0);
        assert_eq!(entry.door_penalty(board), 0);
        assert_eq!(entry.ripened_payout(base, board), base);

        session.age_board();
        session.age_board();
        let ripened = &session.board[0];

        assert_eq!(ripened.ripeness, 2);
        assert_eq!(
            ripened.payout_bonus_pct(board),
            2 * board.ripeness_payout_pct
        );
        assert_eq!(ripened.door_penalty(board), 2 * board.ripeness_door_penalty);
        assert!(ripened.ripened_payout(base, board) > base);
    }

    #[test]
    fn ripening_stops_before_a_mark_becomes_the_whole_campaign() {
        let data = data();
        let mut session = GameSession::new(&data.config, &data, 32);
        let board = &data.config.board;

        // Ripeness keeps counting, but neither number does past the cap.
        for _ in 0..3 {
            session.age_board();
        }
        let capped = session.board[0].clone();
        let mut past = capped.clone();
        past.ripeness = 50;

        assert_eq!(
            past.payout_bonus_pct(board),
            board.ripeness_max as i64 * board.ripeness_payout_pct
        );
        assert_eq!(
            past.door_penalty(board),
            board.ripeness_max as i32 * board.ripeness_door_penalty
        );
    }

    #[test]
    fn the_board_ages_out_marks_whose_window_closed() {
        let data = data();
        let mut session = GameSession::new(&data.config, &data, 11);
        let starting = session.board.len();
        assert!(starting > 0);

        for _ in 0..4 {
            session.age_board();
        }
        assert!(session.board.is_empty());
    }

    #[test]
    fn a_mark_quotes_the_city_watch_on_its_own_doors_before_anybody_commits() {
        // Pillar 2 again, one screen earlier than the planning breakdown: the
        // whole point of the city hardening against a method is that the player
        // chooses a *different mark*, and they can only do that on the board.
        let data = data();
        let mut session = GameSession::new(&data.config, &data, 44);
        let target = data
            .targets
            .get(&session.board[0].target_id)
            .unwrap()
            .clone();
        assert!(session.watched_doors(&target, &data).is_empty());

        let doors = data.encounters_for(&target);
        let trade = doors[0].primary_skill;
        session
            .scrutiny
            .note(trade, data.config.scrutiny.ceiling(), &data.config.scrutiny);

        let watched = session.watched_doors(&target, &data);
        assert!(!watched.is_empty(), "a watched trade quoted nothing");
        for (index, penalty) in watched {
            assert_eq!(doors[index].primary_skill, trade);
            assert_eq!(penalty, data.config.scrutiny.max_penalty);
        }
    }

    #[test]
    fn a_trade_nobody_has_worked_is_quoted_at_nothing() {
        let data = data();
        let session = GameSession::new(&data.config, &data, 45);
        for skill in Skill::ALL {
            assert_eq!(session.scrutiny.penalty(skill, &data.config.scrutiny), 0);
        }
    }
}
