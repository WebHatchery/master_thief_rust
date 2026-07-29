//! Marks and the doors between the crew and the payout.

use super::attributes::{AttributeKind, Skill, Skills};
use super::equipment::EquipmentSlot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Complexity {
    Simple,
    Moderate,
    Complex,
    Legendary,
}

impl Complexity {
    pub const ALL: [Complexity; 4] = [
        Complexity::Simple,
        Complexity::Moderate,
        Complexity::Complex,
        Complexity::Legendary,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Complexity::Simple => "Simple",
            Complexity::Moderate => "Moderate",
            Complexity::Complex => "Complex",
            Complexity::Legendary => "Legendary",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Complexity::Simple => "simple",
            Complexity::Moderate => "moderate",
            Complexity::Complex => "complex",
            Complexity::Legendary => "legendary",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DifficultyBand {
    Easy,
    Medium,
    Hard,
    Extreme,
}

impl DifficultyBand {
    pub const ALL: [DifficultyBand; 4] = [
        DifficultyBand::Easy,
        DifficultyBand::Medium,
        DifficultyBand::Hard,
        DifficultyBand::Extreme,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DifficultyBand::Easy => "Easy",
            DifficultyBand::Medium => "Medium",
            DifficultyBand::Hard => "Hard",
            DifficultyBand::Extreme => "Extreme",
        }
    }
}

/// Per-slot bonuses an encounter grants for bringing the right kit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotBonuses {
    #[serde(default)]
    pub weapon: i32,
    #[serde(default)]
    pub armor: i32,
    #[serde(default)]
    pub accessory: i32,
    #[serde(default)]
    pub tool: i32,
    #[serde(default)]
    pub gadget: i32,
}

impl SlotBonuses {
    pub fn get(&self, slot: EquipmentSlot) -> i32 {
        match slot {
            EquipmentSlot::Weapon => self.weapon,
            EquipmentSlot::Armor => self.armor,
            EquipmentSlot::Accessory => self.accessory,
            EquipmentSlot::Tool => self.tool,
            EquipmentSlot::Gadget => self.gadget,
        }
    }

    pub fn is_empty(&self) -> bool {
        EquipmentSlot::ALL.iter().all(|slot| self.get(*slot) == 0)
    }
}

/// What a critical result does to the rest of the run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunEffect {
    /// The run carries on unchanged.
    #[default]
    None,
    /// The next door is already open — skip it.
    SkipNext,
    /// Something went wrong loudly — a complication is inserted.
    AddComplication,
}

impl RunEffect {
    /// The warning a scouted door carries about what a critical here would do
    /// to the rest of the run.
    ///
    /// GDD 12's third open question was whether *adding* an encounter can ever
    /// feel fair. It cannot while it arrives unannounced, and it can once the
    /// file says which doors are capable of it — so the answer is not to soften
    /// the effect but to put it on the report the crew paid to have made.
    ///
    /// The wording is fixed rather than authored per encounter because it is
    /// describing a *rule*, not content: `SkipNext` only ever hangs off a
    /// critical success and `AddComplication` only ever off a critical failure,
    /// across every encounter in the game.
    pub fn telegraph(self) -> Option<&'static str> {
        match self {
            RunEffect::None => None,
            RunEffect::SkipNext => Some("Do this one perfectly and the next door opens with it."),
            RunEffect::AddComplication => Some("Botch this one and something else comes running."),
        }
    }
}

/// A single door, guard, camera, or conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Encounter {
    pub id: String,
    pub name: String,
    pub description: String,
    pub primary_skill: Skill,
    #[serde(default)]
    pub primary_attribute: Option<AttributeKind>,
    /// Difficulty class, 5-25.
    pub difficulty: i32,
    pub complexity: Complexity,
    pub failure_consequence: String,
    #[serde(default)]
    pub critical_failure_effect: Option<String>,
    #[serde(default)]
    pub critical_success_reward: Option<String>,
    #[serde(default)]
    pub equipment_bonuses: SlotBonuses,
    #[serde(default)]
    pub critical_success_run_effect: RunEffect,
    #[serde(default)]
    pub critical_failure_run_effect: RunEffect,
    /// Complications may only be inserted, never picked as a target's own door.
    #[serde(default)]
    pub complication_only: bool,
}

impl Encounter {
    /// The warnings this door carries about rewriting the run, best case first.
    /// Empty for the sixty-one doors that are only ever themselves.
    pub fn run_effect_telegraphs(&self) -> Vec<&'static str> {
        [
            self.critical_success_run_effect,
            self.critical_failure_run_effect,
        ]
        .into_iter()
        .filter_map(RunEffect::telegraph)
        .collect()
    }

    /// Can anything that happens here change the shape of the rest of the job?
    pub fn can_rewrite_the_run(&self) -> bool {
        self.critical_success_run_effect != RunEffect::None
            || self.critical_failure_run_effect != RunEffect::None
    }
}

/// Where the environment stands for one job. Chosen at planning time and
/// visible before commit — never rolled inside resolution.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    pub time_of_day: String,
    pub weather: String,
    #[serde(default)]
    pub factors: Vec<String>,
}

impl Environment {
    pub fn new(time_of_day: impl Into<String>, weather: impl Into<String>) -> Self {
        Self {
            time_of_day: time_of_day.into(),
            weather: weather.into(),
            factors: Vec::new(),
        }
    }

    pub fn with_factors(mut self, factors: Vec<String>) -> Self {
        self.factors = factors;
        self
    }

    /// Every environment id that contributes a modifier, in the fixed order
    /// resolution reads them.
    pub fn modifier_ids(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.time_of_day.as_str())
            .chain(std::iter::once(self.weather.as_str()))
            .chain(self.factors.iter().map(|f| f.as_str()))
    }
}

/// A named modifier that the environment can apply to a check. Fully authored
/// in `environment.json` so times of day, weather, and site quirks share one
/// mechanism.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentModifier {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub skill_modifiers: Skills,
    /// Applied to every skill on top of the per-skill values.
    #[serde(default)]
    pub all_skills: i32,
}

impl EnvironmentModifier {
    pub fn value_for(&self, skill: Skill) -> i32 {
        self.skill_modifiers.get(skill) + self.all_skills
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeistTarget {
    pub id: String,
    pub name: String,
    pub description: String,
    pub difficulty: DifficultyBand,
    pub potential_payout: i64,
    /// Encounter ids, resolved in order.
    pub encounters: Vec<String>,
    #[serde(default)]
    pub environment: Environment,
    /// Reputation needed before the mark appears on the board.
    #[serde(default)]
    pub required_reputation: i32,
    /// Notoriety added for taking the job at all.
    #[serde(default)]
    pub notoriety: i32,
    #[serde(default)]
    pub possible_loot: Vec<String>,
}
