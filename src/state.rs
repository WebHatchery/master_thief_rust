//! The safehouse ledger: everything a campaign remembers between weeks.

mod board;

pub use board::BoardEntry;

use crate::data::{GameConfig, GameData};
use crate::model::{CrewMember, EquipmentDef, EquipmentSlot, Loadout};
use crate::rules::chemistry::Chemistry;
use crate::rules::Scrutiny;
use crate::sim::CampaignTally;
use macroquad_toolkit::achievements::Achievements;
use macroquad_toolkit::rng::SeededRng;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    /// What the crew's attention has already gone on this week. They only have
    /// so much of it, and it buys two different things — doors put on a file,
    /// and hours spent drilling a hand who has levelled — so spending it on one
    /// mark is spending it away from every other mark *and* from everybody's
    /// training (GDD 12, open question 2).
    #[serde(default, alias = "casing_this_week")]
    pub attention_spent_this_week: u32,
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
    /// What the city has worked out about how this outfit gets in. Heat is how
    /// much attention the crew have drawn; this is what the attention is about,
    /// and it hardens every door of that trade on the whole board (GDD 5.4).
    #[serde(default)]
    pub scrutiny: Scrutiny,
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
            attention_spent_this_week: 0,
            inventory: config.starting_inventory.clone(),
            kit_wear: std::collections::BTreeMap::new(),
            board: Vec::new(),
            chemistry: Chemistry::default(),
            scrutiny: Scrutiny::default(),
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

    /// What the crew can still turn their attention to this week — another door
    /// on a file, or an afternoon drilling somebody. One pool, two uses.
    pub fn attention_left_this_week(&self, config: &GameConfig) -> u32 {
        config
            .attention_per_week
            .saturating_sub(self.attention_spent_this_week)
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
    /// Drill a hand on one attribute. Costs a point they earned *and* an hour of
    /// the week the crew could have spent looking at a building — banked points
    /// used to be a button that was always right to press the moment it lit up,
    /// which is not a decision.
    pub fn spend_attribute_point(
        &mut self,
        config: &GameConfig,
        member_id: &str,
        kind: crate::model::AttributeKind,
    ) -> bool {
        if self.attention_left_this_week(config) == 0 {
            return false;
        }
        let Some(member) = self.member_mut(member_id) else {
            return false;
        };
        if member.progression.attribute_points <= 0 || member.attributes.get(kind) >= 20 {
            return false;
        }
        member.progression.attribute_points -= 1;
        member.attributes.add(kind, 1);
        self.attention_spent_this_week += 1;
        true
    }

    /// The same for a trade. Points keep until the fixer has a week to spare for
    /// them, so a hand who levelled in a busy fortnight stays as they were.
    pub fn spend_skill_point(
        &mut self,
        config: &GameConfig,
        member_id: &str,
        skill: crate::model::Skill,
    ) -> bool {
        if self.attention_left_this_week(config) == 0 {
            return false;
        }
        let Some(member) = self.member_mut(member_id) else {
            return false;
        };
        if member.progression.skill_points <= 0 {
            return false;
        }
        member.progression.skill_points -= 1;
        member.training.add(skill, 1);
        self.attention_spent_this_week += 1;
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

    /// Crew who can be put on a door at all. A spent hand is *available* — the
    /// fixer is allowed to send somebody who should be resting, and pay for it
    /// (GDD 5.6). Only injuries take a name off this list.
    pub fn available_crew<'a>(
        &'a self,
        tuning: &'a crate::rules::ConditionTuning,
    ) -> impl Iterator<Item = &'a CrewMember> {
        self.crew
            .iter()
            .filter(move |member| tuning.can_work(&member.condition))
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
mod tests;
