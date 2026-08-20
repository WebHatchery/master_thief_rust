//! Authored bitmap art and the small lookup rules that keep it legible.
//!
//! The floorplan, dice, chips, meters, and ordinary controls remain procedural
//! so they stay crisp, accessible, and responsive. These textures provide the
//! authored identity and atmosphere called for by `artwork_todo.md`.

use crate::data::GameData;
use macroquad::prelude::*;

mod render;

struct NamedTexture {
    id: String,
    texture: Texture2D,
}

struct PortraitTexture {
    id: String,
    full: Texture2D,
    small: Texture2D,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortraitState {
    Neutral,
    Speaking,
    Pleased,
    Worried,
    Injured,
    Exhausted,
    Arrested,
    Unavailable,
    Unknown,
    Locked,
}

impl PortraitState {
    pub fn key(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Speaking => "speaking",
            Self::Pleased => "pleased",
            Self::Worried => "worried",
            Self::Injured => "injured",
            Self::Exhausted => "exhausted",
            Self::Arrested => "arrested",
            Self::Unavailable => "unavailable",
            Self::Unknown => "unknown",
            Self::Locked => "locked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ItemState {
    Neutral,
    Equipped,
    Selected,
    Locked,
    NewlyFound,
    Worn,
    Broken,
    Sold,
}

pub struct Artwork {
    pub wordmark: Texture2D,
    pub city_clear: Texture2D,
    pub city_fog: Texture2D,
    pub city_rain: Texture2D,
    pub safehouse_plate: Texture2D,
    portraits: Vec<PortraitTexture>,
    targets: Vec<NamedTexture>,
    items: Vec<NamedTexture>,
    portrait_unknown: Texture2D,
    portrait_locked: Texture2D,
    pub slot_glyphs: [Texture2D; 5],
}

impl Artwork {
    pub async fn load(data: &GameData) -> Self {
        let wordmark = load("assets/images/brand/master_thief_wordmark.png").await;
        let city_clear = load("assets/images/environments/night_city_clear.png").await;
        let city_fog = load("assets/images/environments/night_city_fog.png").await;
        let city_rain = load("assets/images/environments/night_city_rain.png").await;
        let safehouse_plate = load("assets/images/environments/safehouse_desk.png").await;
        let portrait_unknown = load("assets/images/portraits/portrait_unknown.png").await;
        let portrait_locked = load("assets/images/portraits/portrait_locked.png").await;
        let mut portrait_ids: Vec<String> = data.crew_pool.ids().cloned().collect();
        portrait_ids.sort();
        let mut portraits = Vec::with_capacity(portrait_ids.len());
        for id in portrait_ids {
            portraits.push(PortraitTexture {
                full: load(&format!("assets/images/portraits/{id}_neutral.png")).await,
                small: load(&format!("assets/images/portraits/{id}_neutral_small.png")).await,
                id,
            });
        }
        let mut target_ids: Vec<String> = data.targets.ids().cloned().collect();
        target_ids.sort();
        let mut targets = Vec::with_capacity(target_ids.len());
        for id in target_ids {
            targets.push(NamedTexture {
                texture: load(&format!("assets/images/targets/{id}.png")).await,
                id,
            });
        }
        let mut item_ids: Vec<String> = data.equipment.ids().cloned().collect();
        item_ids.sort();
        let mut items = Vec::with_capacity(item_ids.len());
        for id in item_ids {
            items.push(NamedTexture {
                texture: load_icon(&format!("assets/images/items/{id}.png")).await,
                id,
            });
        }
        let slot_glyphs = [
            load_icon("assets/images/icons/slot_weapon.png").await,
            load_icon("assets/images/icons/slot_armor.png").await,
            load_icon("assets/images/icons/slot_accessory.png").await,
            load_icon("assets/images/icons/slot_tool.png").await,
            load_icon("assets/images/icons/slot_gadget.png").await,
        ];
        Self {
            wordmark,
            city_clear,
            city_fog,
            city_rain,
            safehouse_plate,
            portraits,
            targets,
            items,
            portrait_unknown,
            portrait_locked,
            slot_glyphs,
        }
    }
}

async fn load(path: &str) -> Texture2D {
    let texture = load_texture(path)
        .await
        .unwrap_or_else(|err| panic!("failed to load authored artwork {path}: {err}"));
    texture.set_filter(FilterMode::Linear);
    texture
}

async fn load_icon(path: &str) -> Texture2D {
    let texture = load_texture(path)
        .await
        .unwrap_or_else(|err| panic!("failed to load authored icon {path}: {err}"));
    texture.set_filter(FilterMode::Nearest);
    texture
}

#[cfg(test)]
mod tests;
