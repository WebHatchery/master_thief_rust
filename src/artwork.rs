//! Authored bitmap art and the small lookup rules that keep it legible.
//!
//! The floorplan, dice, chips, meters, and ordinary controls remain procedural
//! so they stay crisp, accessible, and responsive. These textures provide the
//! authored identity and atmosphere called for by `artwork_todo.md`.

use crate::data::GameData;
use macroquad::prelude::*;

struct NamedTexture {
    id: String,
    texture: Texture2D,
}

struct PortraitTexture {
    id: String,
    full: Texture2D,
    small: Texture2D,
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

    pub fn draw_backplate(&self, screen: crate::ui::Screen) {
        let texture = match screen {
            crate::ui::Screen::Board
            | crate::ui::Screen::Planning
            | crate::ui::Screen::Run
            | crate::ui::Screen::Results => &self.city_rain,
            _ => &self.safehouse_plate,
        };
        draw_texture_ex(
            texture,
            0.0,
            0.0,
            Color::new(0.25, 0.30, 0.38, 0.52),
            DrawTextureParams {
                dest_size: Some(vec2(crate::ui::LOGICAL_WIDTH, crate::ui::LOGICAL_HEIGHT)),
                ..Default::default()
            },
        );
        draw_rectangle(
            0.0,
            0.0,
            crate::ui::LOGICAL_WIDTH,
            crate::ui::LOGICAL_HEIGHT,
            Color::new(0.02, 0.04, 0.07, 0.54),
        );
    }

    pub fn draw_portrait(&self, id: &str, rect: Rect, small: bool) {
        let texture = self
            .portraits
            .iter()
            .find(|portrait| portrait.id == id)
            .map(|portrait| if small { &portrait.small } else { &portrait.full })
            .unwrap_or(&self.portrait_unknown);
        draw_texture_ex(texture, rect.x, rect.y, WHITE, dest_size(rect));
    }

    pub fn draw_target(&self, id: &str, rect: Rect) {
        if let Some(target) = self.targets.iter().find(|target| target.id == id) {
            draw_texture_ex(&target.texture, rect.x, rect.y, WHITE, dest_size(rect));
        } else {
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(0.08, 0.12, 0.16, 1.0));
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, Color::new(0.75, 0.55, 0.22, 0.8));
        }
    }

    /// Equipment uses authored bitmap icon art; rarity remains procedural so
    /// one recognizable object can carry five clear foils and state overlays.
    pub fn draw_item_icon(&self, id: &str, rect: Rect, rarity: crate::model::EquipmentRarity) {
        if let Some(item) = self.items.iter().find(|item| item.id == id) {
            draw_texture_ex(&item.texture, rect.x, rect.y, WHITE, dest_size(rect));
        } else {
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(0.08, 0.12, 0.16, 1.0));
        }
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, rarity_color(rarity));
    }

    pub fn draw_slot_glyph(&self, slot: crate::model::EquipmentSlot, rect: Rect) {
        let index = match slot {
            crate::model::EquipmentSlot::Weapon => 0,
            crate::model::EquipmentSlot::Armor => 1,
            crate::model::EquipmentSlot::Accessory => 2,
            crate::model::EquipmentSlot::Tool => 3,
            crate::model::EquipmentSlot::Gadget => 4,
        };
        draw_texture_ex(
            &self.slot_glyphs[index],
            rect.x,
            rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(rect.w, rect.h)),
                ..Default::default()
            },
        );
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

fn dest_size(rect: Rect) -> DrawTextureParams {
    DrawTextureParams {
        dest_size: Some(vec2(rect.w, rect.h)),
        ..Default::default()
    }
}

fn rarity_color(rarity: crate::model::EquipmentRarity) -> Color {
    match rarity {
        crate::model::EquipmentRarity::Basic => Color::new(0.55, 0.58, 0.64, 1.0),
        crate::model::EquipmentRarity::Improved => Color::new(0.36, 0.76, 0.54, 1.0),
        crate::model::EquipmentRarity::Advanced => Color::new(0.34, 0.66, 0.94, 1.0),
        crate::model::EquipmentRarity::Masterwork => Color::new(0.72, 0.50, 0.92, 1.0),
        crate::model::EquipmentRarity::Legendary => Color::new(0.95, 0.72, 0.30, 1.0),
    }
}
