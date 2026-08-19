//! Procedural floorplan glyphs and route marks.

use super::{Floorplan, NodeState};
use crate::model::Skill;
use macroquad::prelude::*;

/// The trade glyphs share a circular keyline and differ by silhouette, so a
/// color-blind player can still distinguish the six doors at node size.
pub fn draw_skill_glyph(skill: Skill, center: Vec2, radius: f32) {
    let color = Color::new(0.24, 0.72, 0.78, 0.95);
    draw_circle_lines(center.x, center.y, radius, 1.5, color);
    match skill {
        Skill::Stealth => {
            draw_line(
                center.x - radius * 0.55,
                center.y,
                center.x + radius * 0.55,
                center.y,
                2.0,
                color,
            );
            draw_circle(center.x, center.y, radius * 0.16, color);
        }
        Skill::Athletics => {
            draw_line(
                center.x - radius * 0.55,
                center.y + radius * 0.35,
                center.x,
                center.y - radius * 0.45,
                2.0,
                color,
            );
            draw_line(
                center.x,
                center.y - radius * 0.45,
                center.x + radius * 0.55,
                center.y + radius * 0.35,
                2.0,
                color,
            );
        }
        Skill::Combat => {
            draw_line(
                center.x - radius * 0.5,
                center.y - radius * 0.5,
                center.x + radius * 0.5,
                center.y + radius * 0.5,
                2.0,
                color,
            );
            draw_line(
                center.x + radius * 0.5,
                center.y - radius * 0.5,
                center.x - radius * 0.5,
                center.y + radius * 0.5,
                2.0,
                color,
            );
        }
        Skill::Lockpicking => {
            draw_circle_lines(
                center.x - radius * 0.18,
                center.y,
                radius * 0.24,
                2.0,
                color,
            );
            draw_line(
                center.x,
                center.y,
                center.x + radius * 0.55,
                center.y,
                2.0,
                color,
            );
            draw_line(
                center.x + radius * 0.30,
                center.y,
                center.x + radius * 0.30,
                center.y + radius * 0.25,
                2.0,
                color,
            );
        }
        Skill::Hacking => {
            draw_rectangle_lines(
                center.x - radius * 0.50,
                center.y - radius * 0.35,
                radius,
                radius * 0.70,
                2.0,
                color,
            );
            draw_line(
                center.x - radius * 0.25,
                center.y,
                center.x + radius * 0.25,
                center.y,
                1.5,
                color,
            );
        }
        Skill::Social => {
            draw_circle_lines(
                center.x,
                center.y - radius * 0.10,
                radius * 0.30,
                2.0,
                color,
            );
            draw_line(
                center.x - radius * 0.48,
                center.y + radius * 0.45,
                center.x,
                center.y + radius * 0.16,
                2.0,
                color,
            );
            draw_line(
                center.x,
                center.y + radius * 0.16,
                center.x + radius * 0.48,
                center.y + radius * 0.45,
                2.0,
                color,
            );
        }
    }
}

/// Time, weather, site, and security factors use a shared four-pixel keyline.
pub fn draw_factor_glyph(factor: &str, center: Vec2, radius: f32) {
    let color = Color::new(0.88, 0.61, 0.25, 0.95);
    match factor {
        "day" | "dusk" | "night" | "dawn" => {
            draw_circle_lines(center.x, center.y, radius * 0.45, 1.5, color);
            for index in 0..4 {
                let angle = index as f32 * std::f32::consts::FRAC_PI_2;
                draw_line(
                    center.x + angle.cos() * radius * 0.62,
                    center.y + angle.sin() * radius * 0.62,
                    center.x + angle.cos() * radius * 0.90,
                    center.y + angle.sin() * radius * 0.90,
                    1.5,
                    color,
                );
            }
        }
        "rain" | "fog" | "storm" | "clear" => {
            draw_line(
                center.x - radius * 0.5,
                center.y,
                center.x + radius * 0.5,
                center.y,
                2.0,
                color,
            );
            draw_line(
                center.x - radius * 0.30,
                center.y + radius * 0.30,
                center.x + radius * 0.30,
                center.y + radius * 0.30,
                2.0,
                color,
            );
            if factor == "storm" {
                draw_line(
                    center.x,
                    center.y - radius * 0.55,
                    center.x - radius * 0.18,
                    center.y + radius * 0.05,
                    2.0,
                    color,
                );
            }
        }
        "crowded" | "well_lit" | "noisy" | "high_security" => {
            draw_rectangle_lines(
                center.x - radius * 0.52,
                center.y - radius * 0.52,
                radius * 1.04,
                radius * 1.04,
                1.5,
                color,
            );
            draw_circle(center.x, center.y, radius * 0.15, color);
        }
        "wired" | "old_money" | "understaffed" | "private_security" => {
            draw_line(
                center.x - radius * 0.55,
                center.y,
                center.x + radius * 0.55,
                center.y,
                1.5,
                color,
            );
            draw_line(
                center.x,
                center.y - radius * 0.55,
                center.x,
                center.y + radius * 0.55,
                1.5,
                color,
            );
            draw_circle_lines(center.x, center.y, radius * 0.22, 1.5, color);
        }
        _ => draw_circle_lines(center.x, center.y, radius * 0.55, 1.5, color),
    }
}

pub fn draw_node_state(rect: Rect, state: NodeState) {
    let cyan = Color::new(0.24, 0.72, 0.78, 0.95);
    let brass = Color::new(0.76, 0.54, 0.22, 0.95);
    let mint = Color::new(0.45, 0.76, 0.63, 0.95);
    let red = Color::new(0.78, 0.28, 0.27, 0.95);
    let amber = Color::new(0.88, 0.61, 0.25, 0.95);
    match state {
        NodeState::Unknown => {
            draw_line(
                rect.x,
                rect.y,
                rect.right(),
                rect.bottom(),
                2.0,
                Color::new(0.40, 0.44, 0.52, 0.8),
            );
            draw_line(
                rect.right(),
                rect.y,
                rect.x,
                rect.bottom(),
                2.0,
                Color::new(0.40, 0.44, 0.52, 0.8),
            );
        }
        NodeState::Cased => draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.5, brass),
        NodeState::Locked => {
            draw_rectangle_lines(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                2.0,
                Color::new(0.40, 0.44, 0.52, 0.8),
            );
            draw_circle_lines(rect.x + 12.0, rect.y + 12.0, 5.0, 1.5, brass);
            draw_rectangle(rect.x + 9.0, rect.y + 12.0, 6.0, 5.0, brass);
        }
        NodeState::Selected => draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 3.0, cyan),
        NodeState::Assigned => draw_line(
            rect.x + 6.0,
            rect.bottom() - 5.0,
            rect.right() - 6.0,
            rect.bottom() - 5.0,
            3.0,
            mint,
        ),
        NodeState::Ready => draw_circle(rect.right() - 10.0, rect.y + 10.0, 4.0, cyan),
        NodeState::InProgress => {
            draw_circle_lines(rect.right() - 10.0, rect.y + 10.0, 6.0, 2.0, amber)
        }
        NodeState::Success | NodeState::CriticalSuccess => {
            draw_line(
                rect.x + 8.0,
                rect.bottom() - 12.0,
                rect.x + 15.0,
                rect.bottom() - 5.0,
                2.5,
                mint,
            );
            draw_line(
                rect.x + 15.0,
                rect.bottom() - 5.0,
                rect.x + 28.0,
                rect.bottom() - 20.0,
                2.5,
                mint,
            );
            if state == NodeState::CriticalSuccess {
                draw_circle(rect.right() - 10.0, rect.y + 10.0, 4.0, mint);
            }
        }
        NodeState::Failure | NodeState::CriticalFailure => {
            draw_line(
                rect.x + 8.0,
                rect.y + 8.0,
                rect.right() - 8.0,
                rect.bottom() - 8.0,
                2.5,
                red,
            );
            draw_line(
                rect.right() - 8.0,
                rect.y + 8.0,
                rect.x + 8.0,
                rect.bottom() - 8.0,
                2.5,
                red,
            );
            if state == NodeState::CriticalFailure {
                draw_line(
                    rect.x + 5.0,
                    rect.bottom() - 5.0,
                    rect.right() - 5.0,
                    rect.y + 5.0,
                    1.5,
                    red,
                );
            }
        }
        NodeState::Skipped => {
            draw_line(
                rect.x + 6.0,
                rect.y + 6.0,
                rect.right() - 6.0,
                rect.bottom() - 6.0,
                2.0,
                amber,
            );
            draw_line(
                rect.right() - 6.0,
                rect.y + 6.0,
                rect.x + 6.0,
                rect.bottom() - 6.0,
                2.0,
                amber,
            );
        }
    }
}

pub fn draw_route_ink(plan: &Floorplan, color: Color) {
    for corridor in plan.entry.iter().chain(plan.corridors.iter()) {
        let center = vec2(corridor.x + corridor.w * 0.5, corridor.y + corridor.h * 0.5);
        if corridor.w >= corridor.h {
            draw_line(corridor.x, center.y, corridor.right(), center.y, 2.0, color);
        } else {
            draw_line(
                center.x,
                corridor.y,
                center.x,
                corridor.bottom(),
                2.0,
                color,
            );
        }
    }
}

pub fn draw_door_silhouette(rect: Rect, encounter_name: &str) {
    let color = Color::new(0.76, 0.54, 0.22, 0.65);
    let inset = 12.0;
    let door = Rect::new(
        rect.right() - 28.0,
        rect.y + inset,
        16.0,
        rect.h - inset * 2.0,
    );
    draw_rectangle_lines(door.x, door.y, door.w, door.h, 1.5, color);
    let name = encounter_name.to_ascii_lowercase();
    if name.contains("vault") {
        draw_circle_lines(
            door.x + door.w * 0.5,
            door.y + door.h * 0.5,
            4.0,
            1.5,
            color,
        );
    } else if name.contains("roof") {
        draw_line(
            door.x,
            door.y + door.h * 0.5,
            door.right(),
            door.y + door.h * 0.5,
            1.5,
            color,
        );
        draw_line(
            door.x,
            door.y + door.h * 0.5,
            door.x + door.w * 0.5,
            door.y + door.h * 0.25,
            1.5,
            color,
        );
    } else if name.contains("lobby") {
        draw_line(
            door.x + door.w * 0.5,
            door.y,
            door.x + door.w * 0.5,
            door.bottom(),
            1.5,
            color,
        );
        draw_line(
            door.x + 3.0,
            door.y + door.h * 0.5,
            door.x + door.w * 0.5 - 2.0,
            door.y + door.h * 0.5,
            1.5,
            color,
        );
    } else if name.contains("service") || name.contains("loading") {
        for index in 1..4 {
            let y = door.y + door.h * index as f32 / 4.0;
            draw_line(door.x, y, door.right(), y, 1.0, color);
        }
    } else if name.contains("office") {
        draw_rectangle_lines(
            door.x + 3.0,
            door.y + door.h * 0.30,
            door.w - 6.0,
            door.h * 0.35,
            1.5,
            color,
        );
        draw_line(
            door.x + 3.0,
            door.y + door.h * 0.75,
            door.right() - 3.0,
            door.y + door.h * 0.75,
            1.5,
            color,
        );
    } else if name.contains("alley") {
        draw_line(
            door.x + 2.0,
            door.y + door.h - 3.0,
            door.right() - 2.0,
            door.y + 3.0,
            1.5,
            color,
        );
    } else if name.contains("elevator") {
        draw_line(
            door.x + door.w * 0.5,
            door.y + 3.0,
            door.x + door.w * 0.5,
            door.bottom() - 3.0,
            1.5,
            color,
        );
        draw_line(
            door.x + 3.0,
            door.y + door.h * 0.5,
            door.right() - 3.0,
            door.y + door.h * 0.5,
            1.5,
            color,
        );
    } else if name.contains("exit") {
        draw_line(
            door.x + 3.0,
            door.y + door.h * 0.5,
            door.right() - 3.0,
            door.y + door.h * 0.5,
            1.5,
            color,
        );
        draw_line(
            door.right() - 7.0,
            door.y + door.h * 0.5 - 4.0,
            door.right() - 3.0,
            door.y + door.h * 0.5,
            1.5,
            color,
        );
        draw_line(
            door.right() - 7.0,
            door.y + door.h * 0.5 + 4.0,
            door.right() - 3.0,
            door.y + door.h * 0.5,
            1.5,
            color,
        );
    } else {
        draw_circle(door.x + 4.0, door.y + door.h * 0.5, 1.5, color);
    }
}
