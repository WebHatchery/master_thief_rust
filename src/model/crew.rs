//! Crew members: the dossier the player hires, levels, breaks, and mourns.

use super::attributes::{Attributes, Skill, Skills};
use super::equipment::SlotAssignment;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterClass {
    Infiltrator,
    Tech,
    Face,
    Muscle,
    Acrobat,
    Mastermind,
    Wildcard,
}

impl CharacterClass {
    pub fn label(self) -> &'static str {
        match self {
            CharacterClass::Infiltrator => "Infiltrator",
            CharacterClass::Tech => "Tech",
            CharacterClass::Face => "Face",
            CharacterClass::Muscle => "Muscle",
            CharacterClass::Acrobat => "Acrobat",
            CharacterClass::Mastermind => "Mastermind",
            CharacterClass::Wildcard => "Wildcard",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    pub fn label(self) -> &'static str {
        match self {
            Rarity::Common => "Common",
            Rarity::Uncommon => "Uncommon",
            Rarity::Rare => "Rare",
            Rarity::Epic => "Epic",
            Rarity::Legendary => "Legendary",
        }
    }

    /// 0-4. What a hand of this standing thinks they are worth is a step per
    /// tier, so the retainer formula needs the tier as a number.
    pub fn tier(self) -> i32 {
        match self {
            Rarity::Common => 0,
            Rarity::Uncommon => 1,
            Rarity::Rare => 2,
            Rarity::Epic => 3,
            Rarity::Legendary => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjurySeverity {
    Minor,
    Major,
}

impl InjurySeverity {
    /// Flat penalty an active injury applies to every check.
    pub fn check_penalty(self) -> i32 {
        match self {
            InjurySeverity::Minor => -1,
            InjurySeverity::Major => -3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Injury {
    pub description: String,
    pub severity: InjurySeverity,
    /// Weeks of rest still needed before it clears.
    pub weeks_remaining: u32,
}

impl Injury {
    pub fn minor(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            severity: InjurySeverity::Minor,
            weeks_remaining: 1,
        }
    }

    pub fn major(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            severity: InjurySeverity::Major,
            weeks_remaining: 3,
        }
    }
}

/// Everything about a crew member that a week of play can change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Condition {
    /// 0-100. Above 50 it starts costing them dice.
    pub fatigue: i32,
    /// 0-100. High loyalty steadies a hand; low loyalty makes it shake.
    pub loyalty: i32,
    pub injuries: Vec<Injury>,
    /// Consecutive weeks the outfit could not cover their retainer.
    #[serde(default)]
    pub weeks_unpaid: i32,
    /// True once they have said they are done. One week of warning, then they
    /// take their kit and go.
    #[serde(default)]
    pub notice_given: bool,
    /// Did this hand actually stand in a building this week? Chemistry cools
    /// between pairs who did not (GDD 5.5). Cleared when the week turns over.
    #[serde(default)]
    pub worked_this_week: bool,
}

impl Default for Condition {
    fn default() -> Self {
        Self {
            fatigue: 0,
            loyalty: 60,
            injuries: Vec::new(),
            weeks_unpaid: 0,
            notice_given: false,
            worked_this_week: false,
        }
    }
}

impl Condition {
    // Whether a hand can go at all, and whether they are past the point where
    // they should be asked to, are both *rules* and live with the rest of the
    // condition curve in `rules::ConditionTuning`. `model` stays a pure type;
    // it does not know what the numbers mean.

    pub fn add_fatigue(&mut self, amount: i32) {
        self.fatigue = (self.fatigue + amount).clamp(0, 100);
    }

    pub fn adjust_loyalty(&mut self, delta: i32) {
        self.loyalty = (self.loyalty + delta).clamp(0, 100);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progression {
    pub level: i32,
    pub experience: i32,
    pub experience_to_next: i32,
    pub attribute_points: i32,
    pub skill_points: i32,
    /// 0-10 mastery of the member's specialisation. Earned at the doors of
    /// that trade, not handed out — see [`crate::rules::attributes::work_the_trade`].
    pub mastery_level: i32,
    /// Doors of their own trade cleared since the last rank. Resets on promotion.
    #[serde(default)]
    pub specialty_doors: u32,
    pub jobs_completed: u32,
    pub jobs_succeeded: u32,
}

impl Default for Progression {
    fn default() -> Self {
        Self {
            level: 1,
            experience: 0,
            experience_to_next: 100,
            attribute_points: 0,
            skill_points: 0,
            mastery_level: 0,
            specialty_doors: 0,
            jobs_completed: 0,
            jobs_succeeded: 0,
        }
    }
}

impl Progression {
    pub fn success_rate(&self) -> f32 {
        if self.jobs_completed == 0 {
            0.0
        } else {
            self.jobs_succeeded as f32 / self.jobs_completed as f32
        }
    }
}

/// A hired hand. `training` holds raw ranks; the effective skill totals come
/// from [`crate::rules::attributes::effective_skills`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrewMember {
    pub id: String,
    pub name: String,
    pub specialty: String,
    /// The skill mastery sharpens. The original hung the mastery bonus on
    /// `social` for everyone, which read as a typo; here it lands on the
    /// trade the member actually practises.
    pub specialty_skill: Skill,
    pub background: String,
    pub rarity: Rarity,
    pub class: CharacterClass,
    pub attributes: Attributes,
    pub training: Skills,
    #[serde(default)]
    pub progression: Progression,
    #[serde(default)]
    pub equipment: SlotAssignment,
    #[serde(default)]
    pub condition: Condition,
    pub special_ability: String,
    #[serde(default)]
    pub personality_traits: Vec<String>,
    pub hire_cost: i64,
}
