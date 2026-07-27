//! Equipment: five slots, five grades, and the bonuses they carry into a job.

use super::attributes::{AttributeKind, Attributes, Skill, Skills};
use super::crew::CharacterClass;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentSlot {
    Weapon,
    Armor,
    Accessory,
    Tool,
    Gadget,
}

impl EquipmentSlot {
    pub const ALL: [EquipmentSlot; 5] = [
        EquipmentSlot::Weapon,
        EquipmentSlot::Armor,
        EquipmentSlot::Accessory,
        EquipmentSlot::Tool,
        EquipmentSlot::Gadget,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EquipmentSlot::Weapon => "Weapon",
            EquipmentSlot::Armor => "Armor",
            EquipmentSlot::Accessory => "Accessory",
            EquipmentSlot::Tool => "Tool",
            EquipmentSlot::Gadget => "Gadget",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentRarity {
    Basic,
    Improved,
    Advanced,
    Masterwork,
    Legendary,
}

impl EquipmentRarity {
    pub const ALL: [EquipmentRarity; 5] = [
        EquipmentRarity::Basic,
        EquipmentRarity::Improved,
        EquipmentRarity::Advanced,
        EquipmentRarity::Masterwork,
        EquipmentRarity::Legendary,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EquipmentRarity::Basic => "Basic",
            EquipmentRarity::Improved => "Improved",
            EquipmentRarity::Advanced => "Advanced",
            EquipmentRarity::Masterwork => "Masterwork",
            EquipmentRarity::Legendary => "Legendary",
        }
    }
}

/// Optional per-attribute bonuses, authored sparsely in JSON.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeBonuses {
    #[serde(default)]
    pub strength: i32,
    #[serde(default)]
    pub dexterity: i32,
    #[serde(default)]
    pub intelligence: i32,
    #[serde(default)]
    pub wisdom: i32,
    #[serde(default)]
    pub charisma: i32,
    #[serde(default)]
    pub constitution: i32,
}

impl AttributeBonuses {
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

    pub fn apply_to(&self, attributes: &mut Attributes) {
        for kind in AttributeKind::ALL {
            attributes.add(kind, self.get(kind));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentDef {
    pub id: String,
    pub name: String,
    pub slot: EquipmentSlot,
    pub rarity: EquipmentRarity,
    pub description: String,
    #[serde(default)]
    pub attribute_bonuses: AttributeBonuses,
    #[serde(default)]
    pub skill_bonuses: Skills,
    #[serde(default)]
    /// Prose describing a rule that does not exist. Thirteen items carry one,
    /// each a different bespoke mechanic — "downgrades one major injury per
    /// job", "a failed lockpicking check does not raise the alarm" — and
    /// nothing implements any of them.
    ///
    /// Deliberately **not displayed**. Showing a player a sentence that
    /// describes a mechanic the game does not have is worse than showing them
    /// nothing: it is the screen lying about the dice, which is the one thing
    /// pillar 2 exists to prevent. The field is kept rather than deleted
    /// because the writing is good and thirteen implementations is a feature,
    /// not a tidy-up. Implement them or drop them; do not print them.
    pub special_effects: Vec<String>,
    pub cost: i64,
    #[serde(default)]
    pub required_level: i32,
    #[serde(default)]
    pub required_class: Vec<CharacterClass>,
}

impl EquipmentDef {
    pub fn skill_bonus(&self, skill: Skill) -> i32 {
        self.skill_bonuses.get(skill)
    }
}

/// Which item id sits in each of a member's five slots.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotAssignment {
    #[serde(default)]
    pub weapon: Option<String>,
    #[serde(default)]
    pub armor: Option<String>,
    #[serde(default)]
    pub accessory: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub gadget: Option<String>,
}

impl SlotAssignment {
    pub fn get(&self, slot: EquipmentSlot) -> Option<&str> {
        let value = match slot {
            EquipmentSlot::Weapon => &self.weapon,
            EquipmentSlot::Armor => &self.armor,
            EquipmentSlot::Accessory => &self.accessory,
            EquipmentSlot::Tool => &self.tool,
            EquipmentSlot::Gadget => &self.gadget,
        };
        value.as_deref()
    }

    pub fn set(&mut self, slot: EquipmentSlot, item_id: Option<String>) {
        let target = match slot {
            EquipmentSlot::Weapon => &mut self.weapon,
            EquipmentSlot::Armor => &mut self.armor,
            EquipmentSlot::Accessory => &mut self.accessory,
            EquipmentSlot::Tool => &mut self.tool,
            EquipmentSlot::Gadget => &mut self.gadget,
        };
        *target = item_id;
    }

    pub fn item_ids(&self) -> impl Iterator<Item = &str> {
        EquipmentSlot::ALL
            .into_iter()
            .filter_map(|slot| self.get(slot))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

/// The resolved kit a member carries into an encounter. Borrowed, so the rules
/// engine never has to own or clone the catalogue.
#[derive(Debug, Clone, Default)]
pub struct Loadout<'a> {
    items: Vec<(EquipmentSlot, &'a EquipmentDef)>,
}

impl<'a> Loadout<'a> {
    pub fn empty() -> Self {
        Self { items: Vec::new() }
    }

    /// Resolve a member's slot assignment against a catalogue. Unknown ids are
    /// skipped rather than failing — a save from an older content set should
    /// still load.
    pub fn resolve(
        assignment: &SlotAssignment,
        lookup: impl Fn(&str) -> Option<&'a EquipmentDef>,
    ) -> Self {
        let mut items = Vec::new();
        for slot in EquipmentSlot::ALL {
            if let Some(id) = assignment.get(slot) {
                if let Some(def) = lookup(id) {
                    items.push((slot, def));
                }
            }
        }
        Self { items }
    }

    pub fn iter(&self) -> impl Iterator<Item = (EquipmentSlot, &'a EquipmentDef)> + '_ {
        self.items.iter().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn skill_bonus(&self, skill: Skill) -> i32 {
        self.items
            .iter()
            .map(|(_, def)| def.skill_bonus(skill))
            .sum()
    }

    pub fn attribute_bonuses(&self) -> AttributeBonuses {
        let mut total = AttributeBonuses::default();
        for (_, def) in &self.items {
            total.strength += def.attribute_bonuses.strength;
            total.dexterity += def.attribute_bonuses.dexterity;
            total.intelligence += def.attribute_bonuses.intelligence;
            total.wisdom += def.attribute_bonuses.wisdom;
            total.charisma += def.attribute_bonuses.charisma;
            total.constitution += def.attribute_bonuses.constitution;
        }
        total
    }
}
