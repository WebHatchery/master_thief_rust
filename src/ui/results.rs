//! The results screen: the dice, laid out with every modifier that made them.

use super::chrome::{draw_panel, empty_notice, panel_style, stat_row, title_style};
use super::run::outcome_color;
use super::{content_rect, UiContext};
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
    let layout = GridLayout::new(content.x, content.y, content.w, 8.0, 1, 124.0);

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
    // Every modifier, in columns, rather than one joined line that ran off the
    // side of the panel once doors started carrying a dozen of them.
    super::chrome::draw_modifier_grid(
        Rect::new(rect.x + 12.0, rect.y + 84.0, rect.w - 24.0, 36.0),
        door.result.check.significant(),
        usize::MAX,
        12.0,
    );

    if let Some(injury) = &door.injury {
        draw_text_right(
            &injury.description,
            rect.right() - 12.0,
            rect.y + 63.0,
            TextStyle::new(13.0, Color::new(0.90, 0.44, 0.38, 1.0)),
        );
    }

    // What the critical did, in the encounter's own words, plus what it did to
    // the plan. Only criticals have either (GDD 5.2).
    let consequence: Vec<&str> = door
        .critical_effect
        .as_deref()
        .into_iter()
        .chain(door.structural_note())
        .collect();
    if !consequence.is_empty() {
        draw_ui_text_ex(
            &consequence.join(" "),
            rect.x + 12.0,
            rect.y + 81.0,
            TextStyle::new(13.0, tone).params(),
        );
    }
}

fn draw_ledger(ctx: &UiContext<'_>, report: &JobReport) {
    let content = draw_panel(
        ledger_rect(),
        match (report.success, report.was_called_off()) {
            (_, true) => "Walked",
            (true, _) => "Paid",
            _ => "Burned",
        },
    );

    let rows: [(String, String); 8] = [
        (
            "Doors cleared".to_owned(),
            format!("{}/{}", report.doors_passed(), report.doors_total()),
        ),
        (
            "Success rate".to_owned(),
            format!("{:.0}%", report.success_rate() * 100.0),
        ),
        ("Take (gross)".to_owned(), format_money(report.gross)),
        (
            format!("Crew's cut ({:.0}%)", report.cut.percent()),
            format!("-{}", format_money(report.cut.take_of(report.gross))),
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
            Rect::new(content.x, content.y + index as f32 * 23.0, content.w, 22.0),
            label,
            value,
            17.0,
            dark::TEXT,
        );
    }

    // What the standing order actually did, in the same words the planning
    // screen used to offer it. A job that ended early has to say why it ended
    // early, or the ledger reads as a botched one (pillar 2).
    let note = match report.called_off_with {
        Some(1) => "Called off on your order — one door left standing.".to_owned(),
        Some(left) => format!("Called off on your order — {} doors left standing.", left),
        None if report.delegated => crate::sim::delegation::summarise(&report.delegation_misses),
        None => "Planned by you, door by door.".to_owned(),
    };
    draw_text_block(
        &note,
        content.x,
        content.y + 190.0,
        content.w,
        44.0,
        15.0,
        3.0,
        dark::TEXT_DIM,
    );

    // Naming the hand who was standing free is the whole point: it is what
    // teaches the planning screen (GDD 5.3).
    for (index, miss) in report.delegation_misses.iter().take(3).enumerate() {
        draw_ui_text_ex(
            &format!(
                "{}: {} ({:+}) — {} was free ({:+})",
                miss.encounter_name, miss.chosen, miss.chosen_bonus, miss.better, miss.better_bonus
            ),
            content.x,
            content.y + 238.0 + index as f32 * 18.0,
            TextStyle::new(13.0, Color::new(0.88, 0.72, 0.44, 1.0)).params(),
        );
    }

    if !report.loot.is_empty() {
        draw_ui_text_ex(
            "Carried out",
            content.x,
            content.y + 302.0,
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
            content.y + 304.0,
            content.w,
            56.0,
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
