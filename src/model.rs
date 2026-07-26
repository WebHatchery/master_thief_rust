//! Pure data types for crew, equipment, targets, and encounters.
//!
//! Nothing in `model` knows about macroquad, rendering, or the running game —
//! it is the vocabulary the rules engine and the UI both speak.

pub mod attributes;
pub mod crew;
pub mod equipment;
pub mod target;

pub use attributes::{AttributeKind, Attributes, DerivedStats, Skill, Skills};
pub use crew::{CharacterClass, CrewMember, Progression, Rarity};
pub use equipment::{EquipmentDef, EquipmentRarity, EquipmentSlot, Loadout};
pub use target::{
    DifficultyBand, Encounter, Environment, EnvironmentModifier, HeistTarget, RunEffect,
};
