//! Authored bitmap art and the small lookup rules that keep it legible.
//!
//! The floorplan, dice, chips, meters, and ordinary controls remain procedural
//! so they stay crisp, accessible, and responsive. These textures provide the
//! authored identity and atmosphere called for by `artwork_todo.md`.

use crate::data::GameData;
use crate::model::CharacterClass;
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

    pub fn draw_backplate(&self, screen: crate::ui::Screen, weather: Option<&str>) {
        let texture = match screen {
            crate::ui::Screen::Board
            | crate::ui::Screen::Planning
            | crate::ui::Screen::Run
            | crate::ui::Screen::Results => self.city_plate(weather),
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

    fn city_plate(&self, weather: Option<&str>) -> &Texture2D {
        match weather {
            Some("clear") => &self.city_clear,
            Some("fog") => &self.city_fog,
            _ => &self.city_rain,
        }
    }

    pub fn draw_portrait(&self, id: &str, rect: Rect, small: bool) {
        self.draw_portrait_state(id, rect, small, PortraitState::Neutral);
    }

    pub fn draw_portrait_state(&self, id: &str, rect: Rect, small: bool, state: PortraitState) {
        let texture = self
            .state_portrait(id, small, state)
            .unwrap_or(&self.portrait_unknown);
        draw_texture_ex(texture, rect.x, rect.y, WHITE, dest_size(rect));
        self.draw_portrait_treatment(rect, state);
    }

    fn state_portrait(&self, id: &str, small: bool, state: PortraitState) -> Option<&Texture2D> {
        match state {
            PortraitState::Unknown => Some(&self.portrait_unknown),
            PortraitState::Locked | PortraitState::Unavailable => Some(&self.portrait_locked),
            _ => self
                .portraits
                .iter()
                .find(|portrait| portrait.id == id)
                .map(|portrait| {
                    if small {
                        &portrait.small
                    } else {
                        &portrait.full
                    }
                }),
        }
    }

    fn draw_portrait_treatment(&self, rect: Rect, state: PortraitState) {
        let brass = Color::new(0.76, 0.54, 0.22, 0.95);
        let cyan = Color::new(0.24, 0.72, 0.78, 0.95);
        let mint = Color::new(0.45, 0.76, 0.63, 0.95);
        let amber = Color::new(0.88, 0.61, 0.25, 0.95);
        let red = Color::new(0.78, 0.28, 0.27, 0.95);
        match state {
            PortraitState::Neutral => {}
            PortraitState::Speaking => {
                draw_rectangle(rect.x, rect.bottom() - 4.0, rect.w, 4.0, cyan);
                draw_circle(rect.right() - 10.0, rect.y + 10.0, 3.0, cyan);
            }
            PortraitState::Pleased => {
                draw_line(
                    rect.right() - 18.0,
                    rect.y + 12.0,
                    rect.right() - 10.0,
                    rect.y + 20.0,
                    3.0,
                    mint,
                );
                draw_line(
                    rect.right() - 10.0,
                    rect.y + 20.0,
                    rect.right() - 4.0,
                    rect.y + 8.0,
                    3.0,
                    mint,
                );
            }
            PortraitState::Worried => {
                draw_line(
                    rect.x + 8.0,
                    rect.y + 10.0,
                    rect.x + 24.0,
                    rect.y + 6.0,
                    2.0,
                    amber,
                );
                draw_line(
                    rect.x + 8.0,
                    rect.y + 16.0,
                    rect.x + 24.0,
                    rect.y + 12.0,
                    2.0,
                    amber,
                );
            }
            PortraitState::Injured => {
                draw_rectangle(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    Color::new(0.32, 0.10, 0.12, 0.20),
                );
                let bandage = Rect::new(rect.x + 8.0, rect.bottom() - 22.0, 28.0, 12.0);
                draw_rectangle(
                    bandage.x,
                    bandage.y,
                    bandage.w,
                    bandage.h,
                    Color::new(0.86, 0.76, 0.58, 0.9),
                );
                draw_line(
                    bandage.x + 8.0,
                    bandage.y,
                    bandage.x + 8.0,
                    bandage.bottom(),
                    1.0,
                    red,
                );
                draw_line(
                    bandage.x + 18.0,
                    bandage.y,
                    bandage.x + 18.0,
                    bandage.bottom(),
                    1.0,
                    red,
                );
            }
            PortraitState::Exhausted => {
                draw_rectangle(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    Color::new(0.03, 0.05, 0.08, 0.34),
                );
                let center = vec2(rect.right() - 16.0, rect.bottom() - 16.0);
                draw_circle_lines(center.x, center.y, 9.0, 2.0, brass);
                draw_line(center.x, center.y, center.x, center.y - 5.0, 2.0, brass);
                draw_line(
                    center.x,
                    center.y,
                    center.x + 4.0,
                    center.y + 3.0,
                    2.0,
                    brass,
                );
            }
            PortraitState::Arrested => {
                for index in 1..4 {
                    let x = rect.x + rect.w * index as f32 / 4.0;
                    draw_line(x, rect.y, x, rect.bottom(), 3.0, red);
                }
            }
            PortraitState::Unavailable => {
                draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 3.0, amber);
            }
            PortraitState::Unknown => {
                draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, brass);
            }
            PortraitState::Locked => {
                draw_rectangle_lines(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    3.0,
                    Color::new(0.45, 0.50, 0.58, 0.9),
                );
            }
        }
    }

    /// Draw a stable, text-free class badge. The outer ring is shared so the
    /// badge remains readable at 16–18 px; the inner glyph is distinct by
    /// class rather than relying on color alone.
    pub fn draw_class_badge(&self, class: CharacterClass, rect: Rect) {
        let center = vec2(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5);
        let radius = rect.w.min(rect.h) * 0.38;
        let stroke = 1.5_f32.max(rect.w * 0.08);
        let dark = Color::new(0.04, 0.06, 0.09, 0.9);
        let brass = Color::new(0.76, 0.54, 0.22, 0.95);
        let cyan = Color::new(0.24, 0.72, 0.78, 0.95);
        draw_circle(center.x, center.y, radius + stroke + 1.0, dark);
        draw_circle_lines(center.x, center.y, radius, stroke, brass);
        let glyph = match class {
            CharacterClass::Infiltrator => cyan,
            CharacterClass::Tech => Color::new(0.42, 0.70, 0.90, 1.0),
            CharacterClass::Face => Color::new(0.78, 0.54, 0.72, 1.0),
            CharacterClass::Muscle => Color::new(0.78, 0.28, 0.27, 0.95),
            CharacterClass::Acrobat => Color::new(0.45, 0.76, 0.63, 0.95),
            CharacterClass::Mastermind => Color::new(0.78, 0.66, 0.36, 1.0),
            CharacterClass::Wildcard => Color::new(0.88, 0.61, 0.25, 0.95),
        };
        match class {
            CharacterClass::Infiltrator => {
                draw_circle(center.x, center.y - 2.0, radius * 0.24, glyph);
                draw_rectangle(
                    center.x - radius * 0.12,
                    center.y,
                    radius * 0.24,
                    radius * 0.38,
                    glyph,
                );
            }
            CharacterClass::Tech => {
                draw_line(
                    center.x - radius * 0.45,
                    center.y,
                    center.x + radius * 0.45,
                    center.y,
                    stroke,
                    glyph,
                );
                draw_line(
                    center.x,
                    center.y - radius * 0.45,
                    center.x,
                    center.y + radius * 0.45,
                    stroke,
                    glyph,
                );
                draw_circle(center.x, center.y, radius * 0.12, glyph);
            }
            CharacterClass::Face => {
                draw_rectangle_lines(
                    center.x - radius * 0.42,
                    center.y - radius * 0.28,
                    radius * 0.72,
                    radius * 0.52,
                    stroke,
                    glyph,
                );
                draw_line(
                    center.x - radius * 0.05,
                    center.y + radius * 0.24,
                    center.x - radius * 0.24,
                    center.y + radius * 0.46,
                    stroke,
                    glyph,
                );
            }
            CharacterClass::Muscle => {
                draw_rectangle(
                    center.x - radius * 0.38,
                    center.y - radius * 0.30,
                    radius * 0.76,
                    radius * 0.60,
                    glyph,
                );
                draw_line(
                    center.x - radius * 0.18,
                    center.y - radius * 0.45,
                    center.x - radius * 0.18,
                    center.y + radius * 0.45,
                    stroke,
                    dark,
                );
            }
            CharacterClass::Acrobat => {
                draw_line(
                    center.x - radius * 0.45,
                    center.y + radius * 0.35,
                    center.x,
                    center.y - radius * 0.45,
                    stroke,
                    glyph,
                );
                draw_line(
                    center.x,
                    center.y - radius * 0.45,
                    center.x + radius * 0.45,
                    center.y + radius * 0.35,
                    stroke,
                    glyph,
                );
                draw_line(
                    center.x - radius * 0.30,
                    center.y + radius * 0.12,
                    center.x + radius * 0.30,
                    center.y + radius * 0.12,
                    stroke,
                    glyph,
                );
            }
            CharacterClass::Mastermind => {
                draw_circle_lines(center.x, center.y, radius * 0.34, stroke, glyph);
                draw_line(
                    center.x - radius * 0.45,
                    center.y,
                    center.x + radius * 0.45,
                    center.y,
                    stroke,
                    glyph,
                );
            }
            CharacterClass::Wildcard => {
                for index in 0..4 {
                    let angle = index as f32 * std::f32::consts::FRAC_PI_2;
                    draw_line(
                        center.x,
                        center.y,
                        center.x + angle.cos() * radius * 0.48,
                        center.y + angle.sin() * radius * 0.48,
                        stroke,
                        glyph,
                    );
                }
                draw_circle(center.x, center.y, radius * 0.13, glyph);
            }
        }
    }

    pub fn draw_target(&self, id: &str, rect: Rect) {
        if let Some(target) = self.targets.iter().find(|target| target.id == id) {
            draw_texture_ex(&target.texture, rect.x, rect.y, WHITE, dest_size(rect));
        } else {
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                Color::new(0.08, 0.12, 0.16, 1.0),
            );
            draw_rectangle_lines(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                2.0,
                Color::new(0.75, 0.55, 0.22, 0.8),
            );
        }
    }

    /// Equipment uses authored bitmap icon art; rarity remains procedural so
    /// one recognizable object can carry five clear foils and state overlays.
    pub fn draw_item_icon(&self, id: &str, rect: Rect, rarity: crate::model::EquipmentRarity) {
        if let Some(item) = self.items.iter().find(|item| item.id == id) {
            draw_texture_ex(&item.texture, rect.x, rect.y, WHITE, dest_size(rect));
        } else {
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                Color::new(0.08, 0.12, 0.16, 1.0),
            );
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

#[cfg(test)]
mod tests;
