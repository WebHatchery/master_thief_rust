//! Authored bitmap art and the small lookup rules that keep it legible.
//!
//! The floorplan, dice, chips, meters, and ordinary controls remain procedural
//! so they stay crisp, accessible, and responsive. These textures provide the
//! authored identity and atmosphere called for by `artwork_todo.md`.

use macroquad::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct Artwork {
    pub wordmark: Texture2D,
    pub city_plate: Texture2D,
    pub safehouse_plate: Texture2D,
    pub portrait_sheet: Texture2D,
    pub target_sheet: Texture2D,
    pub item_sheet: Texture2D,
    pub slot_glyphs: [Texture2D; 5],
}

impl Artwork {
    pub async fn load() -> Self {
        let wordmark = load("assets/images/brand/master_thief_wordmark.png").await;
        let city_plate = load("assets/images/environments/night_city_rain.png").await;
        let safehouse_plate = load("assets/images/environments/safehouse_desk.png").await;
        let portrait_sheet = load("assets/images/portraits/crew_portrait_sheet.png").await;
        let target_sheet = load("assets/images/targets/target_contact_sheet.png").await;
        let item_sheet = load("assets/images/items/equipment_icon_sheet.png").await;
        let slot_glyphs = [
            load_icon("assets/images/icons/slot_weapon.png").await,
            load_icon("assets/images/icons/slot_armor.png").await,
            load_icon("assets/images/icons/slot_accessory.png").await,
            load_icon("assets/images/icons/slot_tool.png").await,
            load_icon("assets/images/icons/slot_gadget.png").await,
        ];
        Self {
            wordmark,
            city_plate,
            safehouse_plate,
            portrait_sheet,
            target_sheet,
            item_sheet,
            slot_glyphs,
        }
    }

    pub fn draw_backplate(&self, screen: crate::ui::Screen) {
        let texture = match screen {
            crate::ui::Screen::Board
            | crate::ui::Screen::Planning
            | crate::ui::Screen::Run
            | crate::ui::Screen::Results => &self.city_plate,
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
        let index = stable_index(id, 8);
        let col = index % 4;
        let row = index / 4;
        let source = Rect::new(col as f32 * 384.0, row as f32 * 512.0, 384.0, 512.0);
        draw_texture_ex(
            &self.portrait_sheet,
            rect.x,
            rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(rect.w, rect.h)),
                source: Some(source),
                ..Default::default()
            },
        );
        let _ = small;
    }

    pub fn draw_target(&self, id: &str, rect: Rect) {
        let index = stable_index(id, 9);
        let col = index % 3;
        let row = index / 3;
        let source = Rect::new(
            col as f32 * self.target_sheet.width() / 3.0,
            row as f32 * self.target_sheet.height() / 3.0,
            self.target_sheet.width() / 3.0,
            self.target_sheet.height() / 3.0,
        );
        draw_texture_ex(
            &self.target_sheet,
            rect.x,
            rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(rect.w, rect.h)),
                source: Some(source),
                ..Default::default()
            },
        );
    }

    /// Equipment uses authored bitmap icon art; rarity remains procedural so
    /// one recognizable object can carry five clear foils and state overlays.
    pub fn draw_item_icon(&self, id: &str, rect: Rect, rarity: crate::model::EquipmentRarity) {
        let index = stable_index(id, 25);
        let source = Rect::new(
            (index % 5) as f32 * self.item_sheet.width() / 5.0,
            (index / 5) as f32 * self.item_sheet.height() / 5.0,
            self.item_sheet.width() / 5.0,
            self.item_sheet.height() / 5.0,
        );
        draw_texture_ex(
            &self.item_sheet,
            rect.x,
            rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(rect.w, rect.h)),
                source: Some(source),
                ..Default::default()
            },
        );
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

fn stable_index(value: &str, count: usize) -> usize {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    (hasher.finish() as usize) % count
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
