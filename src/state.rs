//! The safehouse ledger: everything a campaign remembers between weeks.

use crate::data::{GameConfig, GameData};
use crate::model::{CrewMember, EquipmentDef, EquipmentSlot, HeistTarget, Loadout};
use crate::rules::chemistry::Chemistry;
use macroquad_toolkit::rng::SeededRng;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One mark on the board, and how much the crew has bothered to learn about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardEntry {
    pub target_id: String,
    /// Cased targets show their DCs and environment on the planning screen.
    pub cased: bool,
    /// Weeks this mark stays on the board before the window closes.
    pub weeks_remaining: u32,
}

impl BoardEntry {
    pub fn new(target_id: impl Into<String>) -> Self {
        Self {
            target_id: target_id.into(),
            cased: false,
            weeks_remaining: 4,
        }
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
    /// Equipment ids in the lockup, including items currently assigned.
    pub inventory: Vec<String>,
    pub board: Vec<BoardEntry>,
    /// Who works well with whom (GDD 5.5).
    #[serde(default)]
    pub chemistry: Chemistry,
    /// Crew-pool ids on offer this week.
    #[serde(default)]
    pub recruits: Vec<String>,
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
            inventory: config.starting_inventory.clone(),
            board: Vec::new(),
            chemistry: Chemistry::default(),
            recruits: Vec::new(),
        };
        session.issue_starting_kit(data);
        session.refresh_board(config, data);
        session.refresh_recruits(config, data);
        session
    }

    pub fn crew_ids(&self) -> Vec<String> {
        self.crew.iter().map(|member| member.id.clone()).collect()
    }

    /// Redraw the hiring pool from everyone not already on the payroll.
    pub fn refresh_recruits(&mut self, config: &GameConfig, data: &GameData) {
        let mut pool: Vec<String> = data
            .crew_pool
            .ids()
            .filter(|id| !self.crew.iter().any(|member| &member.id == *id))
            .cloned()
            .collect();
        pool.sort();

        self.recruits.clear();
        while self.recruits.len() < config.recruit_pool_size && !pool.is_empty() {
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
        if self.budget < recruit.hire_cost {
            return Err(format!("{} wants more than the outfit has", recruit.name));
        }

        self.budget -= recruit.hire_cost;
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

    /// Age the board by a week, dropping marks whose window has closed.
    pub fn age_board(&mut self) {
        for entry in &mut self.board {
            entry.weeks_remaining = entry.weeks_remaining.saturating_sub(1);
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
