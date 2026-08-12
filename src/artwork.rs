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
}

impl Artwork {
    pub async fn load() -> Self {
        let wordmark = load("assets/images/brand/master_thief_wordmark.png").await;
        let city_plate = load("assets/images/environments/night_city_rain.png").await;
        let safehouse_plate = load("assets/images/environments/safehouse_desk.png").await;
        let portrait_sheet = load("assets/images/portraits/crew_portrait_sheet.png").await;
        let target_sheet = load("assets/images/targets/target_contact_sheet.png").await;
        Self {
            wordmark,
            city_plate,
            safehouse_plate,
            portrait_sheet,
            target_sheet,
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

    /// Equipment is intentionally code-drawn: five slot families and five
    /// rarity foils stay sharp at 24 px and can show wear/locked overlays
    /// without shipping 335 duplicate bitmaps.
    pub fn draw_item_glyph(&self, id: &str, rect: Rect, rarity: crate::model::EquipmentRarity) {
        let fill = match rarity {
            crate::model::EquipmentRarity::Basic => Color::new(0.36, 0.39, 0.45, 1.0),
            crate::model::EquipmentRarity::Improved => Color::new(0.36, 0.68, 0.48, 1.0),
            crate::model::EquipmentRarity::Advanced => Color::new(0.30, 0.58, 0.78, 1.0),
            crate::model::EquipmentRarity::Masterwork => Color::new(0.64, 0.42, 0.75, 1.0),
            crate::model::EquipmentRarity::Legendary => Color::new(0.82, 0.58, 0.18, 1.0),
        };
        let index = stable_index(id, 5);
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::new(0.04, 0.06, 0.09, 0.92),
        );
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, fill);
        let center = rect.center();
        match index {
            0 => draw_line(
                rect.x + 12.0,
                center.y,
                rect.right() - 12.0,
                center.y,
                5.0,
                fill,
            ),
            1 => draw_circle(center.x, center.y, rect.w * 0.25, fill),
            2 => draw_rectangle(center.x - 13.0, center.y - 18.0, 26.0, 36.0, fill),
            3 => draw_poly(center.x, center.y, 4, rect.w * 0.28, 45.0, fill),
            _ => draw_triangle(
                vec2(center.x, center.y - 18.0),
                vec2(center.x - 20.0, center.y + 16.0),
                vec2(center.x + 20.0, center.y + 16.0),
                fill,
            ),
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

fn stable_index(value: &str, count: usize) -> usize {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    (hasher.finish() as usize) % count
}
