//! The results screen: the dice, laid out with every modifier that made them.

use super::chrome::{draw_panel, empty_notice, panel_style, stat_row, title_style};
use super::run::outcome_color;
use super::{content_rect, UiContext};
use crate::rules::encounter::EncounterResult;
use crate::sim::{DoorOutcome, JobReport};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

pub fn draw(ctx: &UiContext<'_>) {
    let Some(report) = ctx.last_report else {
        let rect = content_rect();
        draw_surface_with_title(rect, Some("Last Job"), &panel_style(), title_style());
        empty_notice(
            rect,
            "No job has been run yet. Take something off the board.",
        );
        return;
    };

    draw_doors(ctx, report);
    draw_ledger(ctx, report);
}

/// The results screen splits differently from the browsing screens: the doors
/// need the room, and the ledger is a narrow column.
const LEDGER_WIDTH: f32 = 430.0;

fn doors_rect() -> Rect {
    let content = content_rect();
    Rect::new(
        content.x,
        content.y,
        content.w - LEDGER_WIDTH - 14.0,
        content.h,
    )
}

fn ledger_rect() -> Rect {
    let content = content_rect();
    Rect::new(
        content.right() - LEDGER_WIDTH,
        content.y,
        LEDGER_WIDTH,
        content.h,
    )
}

fn draw_doors(ctx: &UiContext<'_>, report: &JobReport) {
    let content = draw_panel(
        doors_rect(),
        &format!("{} — door by door", report.target_name),
    );
    let layout = GridLayout::new(content.x, content.y, content.w, 8.0, 1, 88.0);

    for (index, door) in report.doors.iter().enumerate() {
        let (x, y, w, h) = layout.get_item_rect(index, 0.0);
        let door_rect = Rect::new(x, y, w, h);
        if door_rect.bottom() > content.bottom() {
            draw_ui_text_ex(
                &format!("+{} more doors", report.doors.len() - index),
                content.x,
                content.bottom() - 4.0,
                TextStyle::new(14.0, dark::TEXT_DIM).params(),
            );
            break;
        }
        draw_door(door_rect, door, index + 1);
    }
    let _ = ctx;
}

fn draw_door(rect: Rect, door: &DoorOutcome, ordinal: usize) {
    let tone = outcome_color(door.result.outcome);
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.085, 0.095, 0.12, 1.0))
            .with_left_accent(4.0, tone)
            .with_border(1.0, Color::new(0.42, 0.48, 0.58, 0.35)),
    );

    let title = if door.was_complication {
        format!("{}. {} (complication)", ordinal, door.encounter_name)
    } else {
        format!("{}. {}", ordinal, door.encounter_name)
    };
    draw_ui_text_ex(
        &title,
        rect.x + 12.0,
        rect.y + 22.0,
        TextStyle::new(17.0, dark::TEXT_BRIGHT).params(),
    );
    draw_ui_text_ex(
        &format!(
            "{} rolled {} {} = {} vs DC {}",
            door.result.check.member_name,
            door.result.roll,
            signed(door.result.check.bonus()),
            door.result.total,
            door.result.check.dc
        ),
        rect.x + 12.0,
        rect.y + 43.0,
        TextStyle::new(14.0, dark::TEXT).params(),
    );
    draw_ui_text_ex(
        &door.narrative,
        rect.x + 12.0,
        rect.y + 63.0,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );
    draw_text_right(
        door.result.outcome.label(),
        rect.right() - 12.0,
        rect.y + 22.0,
        TextStyle::new(15.0, tone),
    );
    draw_text_right(
        &modifier_summary(&door.result),
        rect.right() - 12.0,
        rect.y + 43.0,
        TextStyle::new(13.0, dark::TEXT_DIM),
    );

    if let Some(injury) = &door.injury {
        draw_text_right(
            &injury.description,
            rect.right() - 12.0,
            rect.y + 63.0,
            TextStyle::new(13.0, Color::new(0.90, 0.44, 0.38, 1.0)),
        );
    }
}

fn modifier_summary(result: &EncounterResult) -> String {
    result
        .check
        .significant()
        .map(|entry| format!("{} {}", entry.label, entry.signed()))
        .collect::<Vec<_>>()
        .join("  ")
}

fn draw_ledger(ctx: &UiContext<'_>, report: &JobReport) {
    let content = draw_panel(
        ledger_rect(),
        if report.success { "Paid" } else { "Burned" },
    );

    let rows: [(String, String); 6] = [
        (
            "Doors cleared".to_owned(),
            format!("{}/{}", report.doors_passed(), report.doors.len()),
        ),
        (
            "Success rate".to_owned(),
            format!("{:.0}%", report.success_rate() * 100.0),
        ),
        ("Take (net)".to_owned(), format_money(report.payout)),
        (
            "Reputation".to_owned(),
            format!("+{}", report.reputation_gained),
        ),
        (
            "Notoriety".to_owned(),
            format!("+{}", report.notoriety_gained),
        ),
        ("Heat".to_owned(), format!("+{}", report.heat_gained)),
    ];

    for (index, (label, value)) in rows.iter().enumerate() {
        stat_row(
            Rect::new(content.x, content.y + index as f32 * 26.0, content.w, 22.0),
            label,
            value,
            17.0,
            dark::TEXT,
        );
    }

    let note = if report.delegated {
        "Delegated: the crew picked their own doors. A hand-made plan would have put better people on the hard ones."
    } else {
        "Planned by you, door by door."
    };
    draw_text_block(
        note,
        content.x,
        content.y + 172.0,
        content.w,
        60.0,
        15.0,
        3.0,
        dark::TEXT_DIM,
    );

    if !report.loot.is_empty() {
        draw_ui_text_ex(
            "Carried out",
            content.x,
            content.y + 240.0,
            TextStyle::new(16.0, dark::TEXT_BRIGHT).params(),
        );
        let names: Vec<&str> = report
            .loot
            .iter()
            .map(|id| {
                ctx.data
                    .equipment
                    .get(id)
                    .map(|item| item.name.as_str())
                    .unwrap_or(id.as_str())
            })
            .collect();
        draw_text_block(
            &names.join(
                "
",
            ),
            content.x,
            content.y + 248.0,
            content.w,
            60.0,
            14.0,
            3.0,
            Color::new(0.56, 0.82, 0.60, 1.0),
        );
    }

    let injuries: Vec<&str> = report
        .doors
        .iter()
        .filter_map(|door| door.injury.as_ref())
        .map(|injury| injury.description.as_str())
        .collect();
    if !injuries.is_empty() {
        draw_ui_text_ex(
            "Cost",
            content.x,
            content.y + 244.0,
            TextStyle::new(16.0, dark::TEXT_BRIGHT).params(),
        );
        draw_text_block(
            &injuries.join("\n"),
            content.x,
            content.y + 252.0,
            content.w,
            90.0,
            14.0,
            3.0,
            Color::new(0.88, 0.46, 0.40, 1.0),
        );
    }
}

fn signed(value: i32) -> String {
    if value >= 0 {
        format!("+{}", value)
    } else {
        value.to_string()
    }
}
