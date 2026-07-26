//! The six attributes, the six skills they feed, and the stats derived from them.

use serde::{Deserialize, Serialize};

/// The six attributes every crew member carries, on the 3-20 scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attributes {
    pub strength: i32,
    pub dexterity: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub charisma: i32,
    pub constitution: i32,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            strength: 10,
            dexterity: 10,
            intelligence: 10,
            wisdom: 10,
            charisma: 10,
            constitution: 10,
        }
    }
}

impl Attributes {
    pub fn get(&self, kind: AttributeKind) -> i32 {
        match kind {
            AttributeKind::Strength => self.strength,
            AttributeKind::Dexterity => self.dexterity,
            AttributeKind::Intelligence => self.intelligence,
            AttributeKind::Wisdom => self.wisdom,
            AttributeKind::Charisma => self.charisma,
            AttributeKind::Constitution => self.constitution,
        }
    }

    pub fn add(&mut self, kind: AttributeKind, delta: i32) {
        let slot = match kind {
            AttributeKind::Strength => &mut self.strength,
            AttributeKind::Dexterity => &mut self.dexterity,
            AttributeKind::Intelligence => &mut self.intelligence,
            AttributeKind::Wisdom => &mut self.wisdom,
            AttributeKind::Charisma => &mut self.charisma,
            AttributeKind::Constitution => &mut self.constitution,
        };
        *slot += delta;
    }

    pub fn total(&self) -> i32 {
        AttributeKind::ALL.iter().map(|k| self.get(*k)).sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributeKind {
    Strength,
    Dexterity,
    Intelligence,
    Wisdom,
    Charisma,
    Constitution,
}

impl AttributeKind {
    pub const ALL: [AttributeKind; 6] = [
        AttributeKind::Strength,
        AttributeKind::Dexterity,
        AttributeKind::Intelligence,
        AttributeKind::Wisdom,
        AttributeKind::Charisma,
        AttributeKind::Constitution,
    ];

    pub fn short_label(self) -> &'static str {
        match self {
            AttributeKind::Strength => "STR",
            AttributeKind::Dexterity => "DEX",
            AttributeKind::Intelligence => "INT",
            AttributeKind::Wisdom => "WIS",
            AttributeKind::Charisma => "CHA",
            AttributeKind::Constitution => "CON",
        }
    }
}

/// The six skills, each fed by a pair of attributes plus training.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Skill {
    Stealth,
    Athletics,
    Combat,
    Lockpicking,
    Hacking,
    Social,
}

impl Skill {
    pub const ALL: [Skill; 6] = [
        Skill::Stealth,
        Skill::Athletics,
        Skill::Combat,
        Skill::Lockpicking,
        Skill::Hacking,
        Skill::Social,
    ];

    /// The attribute pair a skill draws on when an encounter names no
    /// explicit primary attribute.
    pub fn attribute_pair(self) -> (AttributeKind, AttributeKind) {
        match self {
            Skill::Stealth => (AttributeKind::Dexterity, AttributeKind::Wisdom),
            Skill::Athletics => (AttributeKind::Strength, AttributeKind::Constitution),
            Skill::Combat => (AttributeKind::Strength, AttributeKind::Dexterity),
            Skill::Lockpicking => (AttributeKind::Dexterity, AttributeKind::Intelligence),
            Skill::Hacking => (AttributeKind::Intelligence, AttributeKind::Wisdom),
            Skill::Social => (AttributeKind::Charisma, AttributeKind::Wisdom),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Skill::Stealth => "Stealth",
            Skill::Athletics => "Athletics",
            Skill::Combat => "Combat",
            Skill::Lockpicking => "Lockpicking",
            Skill::Hacking => "Hacking",
            Skill::Social => "Social",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Skill::Stealth => "stealth",
            Skill::Athletics => "athletics",
            Skill::Combat => "combat",
            Skill::Lockpicking => "lockpicking",
            Skill::Hacking => "hacking",
            Skill::Social => "social",
        }
    }
}

/// A value per skill. Used both for training ranks and for computed totals.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skills {
    #[serde(default)]
    pub stealth: i32,
    #[serde(default)]
    pub athletics: i32,
    #[serde(default)]
    pub combat: i32,
    #[serde(default)]
    pub lockpicking: i32,
    #[serde(default)]
    pub hacking: i32,
    #[serde(default)]
    pub social: i32,
}

impl Skills {
    pub fn get(&self, skill: Skill) -> i32 {
        match skill {
            Skill::Stealth => self.stealth,
            Skill::Athletics => self.athletics,
            Skill::Combat => self.combat,
            Skill::Lockpicking => self.lockpicking,
            Skill::Hacking => self.hacking,
            Skill::Social => self.social,
        }
    }

    pub fn set(&mut self, skill: Skill, value: i32) {
        let slot = match skill {
            Skill::Stealth => &mut self.stealth,
            Skill::Athletics => &mut self.athletics,
            Skill::Combat => &mut self.combat,
            Skill::Lockpicking => &mut self.lockpicking,
            Skill::Hacking => &mut self.hacking,
            Skill::Social => &mut self.social,
        };
        *slot = value;
    }

    pub fn add(&mut self, skill: Skill, delta: i32) {
        let current = self.get(skill);
        self.set(skill, current + delta);
    }

    pub fn total(&self) -> i32 {
        Skill::ALL.iter().map(|s| self.get(*s)).sum()
    }
}

/// Stats computed from attributes. Only the ones a rule actually consumes are
/// kept; the original carried several that nothing ever read.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DerivedStats {
    pub health: i32,
    pub stamina: i32,
    pub initiative: i32,
}
