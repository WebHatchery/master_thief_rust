//! The safehouse ledger: everything a campaign remembers between weeks.

use crate::data::{BoardConfig, GameConfig, GameData};
use crate::model::{CrewMember, EquipmentDef, EquipmentSlot, HeistTarget, Loadout};
use crate::rules::chemistry::Chemistry;
use crate::sim::CampaignTally;
use macroquad_toolkit::achievements::Achievements;
use macroquad_toolkit::rng::SeededRng;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

/// The whole campaign, in one serialisable place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSession {
    /// The run seed, shown on the records screen so a campaign is shareable.
    pub seed: u64,
    /// Every draw in the sim comes from here, in a fixed order (GDD 5.7).
    pub rng: SeededRng,
    pub week: u32,
    pub budget: i64,
    pub reputation: i32,
    pub notoriety: i32,
    pub heat: i32,
    pub crew: Vec<CrewMember>,
    /// Hands the city is holding. Off the roster, still on the books, and one
    /// bail payment away from working again (GDD 5.6, and open question 1).
    #[serde(default)]
    pub custody: Vec<CustodyRecord>,
    /// Weeks a tail stays on the crew, costing every door.
    #[serde(default)]
    pub surveillance_weeks: u32,
    /// Doors put on a file this week. The crew only has so much attention, and
    /// spending it on one mark is spending it away from every other
    /// (GDD 12, open question 2).
    #[serde(default)]
    pub casing_this_week: u32,
    /// Equipment ids in the lockup, including items currently assigned.
    pub inventory: Vec<String>,
    /// Jobs each piece of kit has been carried through since its last refit.
    /// Keyed by item id: the lockup has no per-instance identity, and two of
    /// the same tool wearing at the same rate is a smaller lie than wear that
    /// vanishes when a tool changes hands (GDD 3, "repair equipment").
    #[serde(default)]
    pub kit_wear: std::collections::BTreeMap<String, u32>,
    pub board: Vec<BoardEntry>,
    /// Who works well with whom (GDD 5.5).
    #[serde(default)]
    pub chemistry: Chemistry,
    /// Crew-pool ids on offer this week.
    #[serde(default)]
    pub recruits: Vec<String>,
    /// Everything the campaign counts about itself.
    #[serde(default)]
    pub tally: CampaignTally,
    #[serde(default)]
    pub achievements: Achievements,
    /// One line per job, oldest first — the records screen reads this.
    #[serde(default)]
    pub history: Vec<JobRecord>,
    /// Set once the outfit has walked away. A retired campaign takes no more
    /// weeks and runs no more jobs; it is a finished thing to be read
    /// (GDD 12, open question 5).
    #[serde(default)]
    pub retired: Option<crate::sim::retirement::Retirement>,
}

/// Somebody the city is holding, kept whole so bail returns the same person
/// with the same levels, kit, and history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustodyRecord {
    pub member: CrewMember,
    pub week_taken: u32,
    pub bail: i64,
}

/// A finished job, kept for the records screen and its charts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobRecord {
    pub week: u32,
    pub target_name: String,
    pub difficulty: crate::model::DifficultyBand,
    pub success: bool,
    pub doors_passed: usize,
    pub doors_total: usize,
    pub payout: i64,
    pub delegated: bool,
    /// Reputation and notoriety as they stood after the job.
    pub reputation: i32,
    pub notoriety: i32,
}

impl GameSession {
    pub fn new(config: &GameConfig, data: &GameData, seed: u64) -> Self {
        let crew = config
            .starting_crew
            .iter()
            .filter_map(|id| data.crew_pool.get(id).cloned())
            .collect();

        let mut session = Self {
            seed,
            rng: SeededRng::new(seed),
            week: 1,
            budget: config.starting_budget,
            reputation: 0,
            notoriety: 0,
            heat: 0,
            crew,
            custody: Vec::new(),
            surveillance_weeks: 0,
            casing_this_week: 0,
            inventory: config.starting_inventory.clone(),
            kit_wear: std::collections::BTreeMap::new(),
            board: Vec::new(),
            chemistry: Chemistry::default(),
            recruits: Vec::new(),
            tally: CampaignTally::default(),
            achievements: Achievements::default(),
            history: Vec::new(),
            retired: None,
        };
        session
            .achievements
            .sync_definitions(data.awards.iter().map(|a| a.definition()).collect());
        session.issue_starting_kit(data);
        session.refresh_board(config, data);
        session.refresh_recruits(config, data);
        session
    }

    pub fn crew_ids(&self) -> Vec<String> {
        self.crew.iter().map(|member| member.id.clone()).collect()
    }

    /// What this recruit actually asks for. A widely known outfit pays danger
    /// money: signing on with people everyone is watching is worth extra, and
    /// notoriety is what makes it so (GDD 2, pillar 4).
    pub fn hire_fee(&self, recruit: &CrewMember, config: &GameConfig) -> i64 {
        let premium = (self.notoriety.max(0) as f32 * config.recruiting.fee_per_notoriety)
            .min(config.recruiting.max_fee_premium);
        recruit.hire_cost + (recruit.hire_cost as f32 * premium) as i64
    }

    /// How many people bother turning up this week. Fewer, the better known the
    /// outfit gets — though somebody is always desperate enough.
    pub fn applicants_this_week(&self, config: &GameConfig) -> usize {
        let shrink = if config.recruiting.pool_shrink_per_notoriety > 0 {
            (self.notoriety.max(0) / config.recruiting.pool_shrink_per_notoriety) as usize
        } else {
            0
        };
        config
            .recruit_pool_size
            .saturating_sub(shrink)
            .max(config.recruiting.min_pool)
    }

    /// Doors the crew still has the attention to scout this week.
    pub fn casing_left_this_week(&self, config: &GameConfig) -> u32 {
        config
            .casing_steps_per_week
            .saturating_sub(self.casing_this_week)
    }

    /// Has the outfit stopped for good?
    pub fn is_retired(&self) -> bool {
        self.retired.is_some()
    }

    /// Is the city holding this hand?
    pub fn is_held(&self, member_id: &str) -> bool {
        self.custody
            .iter()
            .any(|record| record.member.id == member_id)
    }

    /// Redraw the hiring pool from everyone not already on the payroll.
    pub fn refresh_recruits(&mut self, config: &GameConfig, data: &GameData) {
        // Nobody in a cell is out looking for work, and nobody already on the
        // payroll answers their own advertisement.
        let mut pool: Vec<String> = data
            .crew_pool
            .ids()
            .filter(|id| !self.crew.iter().any(|member| &member.id == *id))
            .filter(|id| !self.is_held(id))
            .cloned()
            .collect();
        // Load-bearing: `DataRegistry` is a `HashMap` and `ids()` has no
        // defined order, so the draw below would otherwise depend on hash
        // order rather than on the seed (GDD 5.7).
        pool.sort();

        self.recruits.clear();
        let wanted = self.applicants_this_week(config);
        while self.recruits.len() < wanted && !pool.is_empty() {
            let index = self.rng.below(pool.len());
            self.recruits.push(pool.remove(index));
        }
    }

    /// Put a candidate on the payroll. Returns what went wrong, if anything.
    pub fn hire(&mut self, data: &GameData, recruit_id: &str) -> Result<String, String> {
        let Some(recruit) = data.crew_pool.get(recruit_id) else {
            return Err("Nobody by that name is asking for work".to_owned());
        };
        if self.crew.iter().any(|member| member.id == recruit.id) {
            return Err(format!("{} already works for you", recruit.name));
        }
        let fee = self.hire_fee(recruit, &data.config);
        if self.budget < fee {
            return Err(format!("{} wants more than the outfit has", recruit.name));
        }

        self.budget -= fee;
        self.crew.push(recruit.clone());
        self.recruits.retain(|id| id != recruit_id);
        Ok(recruit.name.clone())
    }

    /// Buy a piece of kit into the lockup.
    pub fn buy(&mut self, data: &GameData, item_id: &str) -> Result<String, String> {
        let Some(item) = data.equipment.get(item_id) else {
            return Err("Nobody sells that".to_owned());
        };
        if self.budget < item.cost {
            return Err(format!("{} costs more than the outfit has", item.name));
        }

        self.budget -= item.cost;
        self.inventory.push(item.id.clone());
        Ok(item.name.clone())
    }

    /// Hand a piece of kit to somebody, swapping out whatever was in the slot.
    pub fn equip(&mut self, data: &GameData, member_id: &str, item_id: &str) -> Result<(), String> {
        let Some(item) = data.equipment.get(item_id) else {
            return Err("Nobody has one of those".to_owned());
        };
        if !self
            .unassigned_inventory(data)
            .iter()
            .any(|def| def.id == item.id)
        {
            return Err(format!("{} is already on somebody", item.name));
        }

        let Some(member) = self.member_mut(member_id) else {
            return Err("They are not on the payroll".to_owned());
        };
        if member.progression.level < item.required_level {
            return Err(format!(
                "{} needs level {} before they can use that",
                member.name, item.required_level
            ));
        }
        if !item.required_class.is_empty() && !item.required_class.contains(&member.class) {
            return Err(format!("{} is not trained for that", member.name));
        }

        member.equipment.set(item.slot, Some(item.id.clone()));
        Ok(())
    }

    pub fn unequip(&mut self, member_id: &str, slot: EquipmentSlot) {
        if let Some(member) = self.member_mut(member_id) {
            member.equipment.set(slot, None);
        }
    }

    /// Spend a level-up point. Returns false when there is none to spend.
    pub fn spend_attribute_point(
        &mut self,
        member_id: &str,
        kind: crate::model::AttributeKind,
    ) -> bool {
        let Some(member) = self.member_mut(member_id) else {
            return false;
        };
        if member.progression.attribute_points <= 0 || member.attributes.get(kind) >= 20 {
            return false;
        }
        member.progression.attribute_points -= 1;
        member.attributes.add(kind, 1);
        true
    }

    pub fn spend_skill_point(&mut self, member_id: &str, skill: crate::model::Skill) -> bool {
        let Some(member) = self.member_mut(member_id) else {
            return false;
        };
        if member.progression.skill_points <= 0 {
            return false;
        }
        member.progression.skill_points -= 1;
        member.training.add(skill, 1);
        true
    }

    /// Put the opening lockup on the people who can use it: each item goes to
    /// the free slot of the hand whose skills it flatters most.
    fn issue_starting_kit(&mut self, data: &GameData) {
        let items: Vec<String> = self.inventory.clone();

        for item_id in items {
            let Some(item) = data.equipment.get(&item_id) else {
                continue;
            };

            let best = self
                .crew
                .iter()
                .enumerate()
                .filter(|(_, member)| member.equipment.get(item.slot).is_none())
                .max_by_key(|(_, member)| item.skill_bonus(member.specialty_skill))
                .map(|(index, _)| index);

            if let Some(index) = best {
                self.crew[index]
                    .equipment
                    .set(item.slot, Some(item_id.clone()));
            }
        }
    }

    pub fn to_save(&self, version: &str) -> SaveData {
        SaveData {
            version: version.to_owned(),
            session: self.clone(),
        }
    }

    pub fn from_save(save: SaveData) -> Self {
        save.session
    }

    pub fn member(&self, id: &str) -> Option<&CrewMember> {
        self.crew.iter().find(|member| member.id == id)
    }

    pub fn member_mut(&mut self, id: &str) -> Option<&mut CrewMember> {
        self.crew.iter_mut().find(|member| member.id == id)
    }

    /// Crew fit to be assigned to a door this week.
    pub fn available_crew(&self) -> impl Iterator<Item = &CrewMember> {
        self.crew
            .iter()
            .filter(|member| member.condition.is_fit_for_work())
    }

    /// Resolve a member's kit against the catalogue.
    pub fn loadout<'a>(&self, member: &CrewMember, data: &'a GameData) -> Loadout<'a> {
        Loadout::resolve(&member.equipment, |id| data.equipment.get(id))
    }

    /// Items in the lockup that nobody is carrying.
    pub fn unassigned_inventory<'a>(&'a self, data: &'a GameData) -> Vec<&'a EquipmentDef> {
        let assigned: Vec<&str> = self
            .crew
            .iter()
            .flat_map(|member| member.equipment.item_ids())
            .collect();

        let mut remaining = assigned.clone();
        self.inventory
            .iter()
            .filter(|id| {
                // One copy is consumed per assignment, so duplicates in the
                // lockup stay visible.
                match remaining.iter().position(|held| held == id) {
                    Some(index) => {
                        remaining.remove(index);
                        false
                    }
                    None => true,
                }
            })
            .filter_map(|id| data.equipment.get(id))
            .collect()
    }

    /// How much the city's attention adds to every difficulty class.
    pub fn heat_dc_penalty(&self, config: &GameConfig) -> i32 {
        if config.heat_dc_step <= 0 {
            return 0;
        }
        ((self.heat - config.heat_safe_threshold).max(0)) / config.heat_dc_step
    }

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: String,
    pub session: GameSession,
}

/// Unwrap the toolkit's save envelope and read the campaign out of it. There is
/// no legacy format yet; when one appears this is where it gets translated.
/// Bring a loaded campaign up to the current content: achievement text may have
/// changed and new ones may exist, while unlock state is preserved.
pub fn adopt_current_definitions(session: &mut GameSession, data: &GameData) {
    session
        .achievements
        .sync_definitions(data.awards.iter().map(|a| a.definition()).collect());
}

pub fn migrate_save_value(
    detected_version: Option<String>,
    value: Value,
    config: &GameConfig,
) -> Result<SaveData, String> {
    let payload = value.get("data").cloned().unwrap_or(value);

    let mut save: SaveData = serde_json::from_value(payload).map_err(|err| {
        format!(
            "Unsupported save format {:?}: {}",
            detected_version.as_deref().unwrap_or("unknown"),
            err
        )
    })?;
    save.version = config.version.clone();
    Ok(save)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> GameData {
        GameData::load().unwrap()
    }

    #[test]
    fn a_new_campaign_hires_the_starting_crew_and_stocks_the_board() {
        let data = data();
        let session = GameSession::new(&data.config, &data, 42);

        assert_eq!(session.crew.len(), data.config.starting_crew.len());
        assert_eq!(session.budget, data.config.starting_budget);
        assert!(!session.board.is_empty());
        assert!(session.board.len() <= data.config.targets_on_board);
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
    fn a_save_round_trips_through_json() {
        let data = data();
        let mut session = GameSession::new(&data.config, &data, 99);
        session.budget -= 4200;
        session.heat = 31;
        session.crew[0].condition.fatigue = 45;

        let save = session.to_save(&data.config.version);
        let encoded = serde_json::to_value(&save).unwrap();
        let restored = migrate_save_value(
            Some(data.config.version.clone()),
            serde_json::json!({ "data": encoded }),
            &data.config,
        )
        .unwrap();

        assert_eq!(
            serde_json::to_value(GameSession::from_save(restored)).unwrap(),
            serde_json::to_value(&session).unwrap()
        );
    }

    #[test]
    fn a_save_the_game_cannot_read_is_reported_not_swallowed() {
        let data = data();
        let result = migrate_save_value(
            Some("0.0.1".to_owned()),
            serde_json::json!({ "data": { "nonsense": true } }),
            &data.config,
        );
        assert!(result.is_err());
    }

    #[test]
    fn heat_only_bites_above_the_safe_threshold() {
        let data = data();
        let mut session = GameSession::new(&data.config, &data, 3);

        session.heat = data.config.heat_safe_threshold;
        assert_eq!(session.heat_dc_penalty(&data.config), 0);

        session.heat = data.config.heat_safe_threshold + data.config.heat_dc_step * 2;
        assert_eq!(session.heat_dc_penalty(&data.config), 2);
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
    fn a_known_outfit_pays_danger_money_to_sign_anybody() {
        // Pillar 4: the two axes pull opposite ways. Reputation opened marks
        // and notoriety priced two rare purchases, which is not opposition —
        // being known now costs the outfit the thing reputation buys most of.
        let data = data();
        let mut session = GameSession::new(&data.config, &data, 60);
        let recruit = data
            .crew_pool
            .get(session.recruits.first().expect("somebody is asking"))
            .unwrap();

        let quiet = session.hire_fee(recruit, &data.config);
        assert_eq!(quiet, recruit.hire_cost, "an unknown outfit pays list");

        session.notoriety = 120;
        let known = session.hire_fee(recruit, &data.config);
        assert!(known > quiet, "infamy was free at the hiring table");

        session.notoriety = 100_000;
        let infamous = session.hire_fee(recruit, &data.config);
        assert_eq!(
            infamous,
            recruit.hire_cost
                + (recruit.hire_cost as f32 * data.config.recruiting.max_fee_premium) as i64,
            "the premium has to stop somewhere"
        );
    }

    #[test]
    fn fewer_people_turn_up_for_an_outfit_everyone_is_watching() {
        let data = data();
        let mut session = GameSession::new(&data.config, &data, 61);
        assert_eq!(
            session.applicants_this_week(&data.config),
            data.config.recruit_pool_size
        );

        session.notoriety = data.config.recruiting.pool_shrink_per_notoriety;
        assert_eq!(
            session.applicants_this_week(&data.config),
            data.config.recruit_pool_size - 1
        );

        // Somebody is always desperate enough.
        session.notoriety = 100_000;
        assert_eq!(
            session.applicants_this_week(&data.config),
            data.config.recruiting.min_pool
        );
    }

    #[test]
    fn the_premium_is_actually_charged_and_not_merely_displayed() {
        let data = data();
        let mut session = GameSession::new(&data.config, &data, 62);
        session.notoriety = 150;
        session.budget = 10_000_000;
        let id = session.recruits[0].clone();
        let fee = session.hire_fee(data.crew_pool.get(&id).unwrap(), &data.config);

        session.hire(&data, &id).unwrap();
        assert_eq!(session.budget, 10_000_000 - fee);
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
    fn the_opening_lockup_is_issued_to_the_people_who_can_use_it() {
        let data = data();
        let session = GameSession::new(&data.config, &data, 5);

        let carried: usize = session
            .crew
            .iter()
            .map(|member| member.equipment.item_ids().count())
            .sum();
        assert_eq!(carried, data.config.starting_inventory.len());
        assert!(session.unassigned_inventory(&data).is_empty());
    }

    #[test]
    fn kit_taken_off_a_hand_reappears_in_the_lockup() {
        let data = data();
        let mut session = GameSession::new(&data.config, &data, 5);
        let (slot, item) = session
            .crew
            .iter()
            .find_map(|member| {
                crate::model::EquipmentSlot::ALL
                    .into_iter()
                    .find_map(|slot| member.equipment.get(slot).map(|id| (slot, id.to_owned())))
            })
            .expect("somebody is carrying the starting kit");

        let holder = session
            .crew
            .iter()
            .position(|member| member.equipment.get(slot) == Some(item.as_str()))
            .unwrap();
        session.crew[holder].equipment.set(slot, None);

        let lockup: Vec<&str> = session
            .unassigned_inventory(&data)
            .iter()
            .map(|def| def.id.as_str())
            .collect();
        assert_eq!(lockup, vec![item.as_str()]);
    }

    #[test]
    fn an_injured_or_exhausted_hand_is_not_offered_for_work() {
        let data = data();
        let mut session = GameSession::new(&data.config, &data, 5);
        assert_eq!(session.available_crew().count(), session.crew.len());

        session.crew[0].condition.fatigue = 95;
        assert_eq!(session.available_crew().count(), session.crew.len() - 1);
    }
}
