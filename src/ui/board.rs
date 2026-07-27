//! The board: the marks currently worth looking at, and what casing bought.

use super::chrome::{
    difficulty_color, draw_panel, empty_notice, list_card, panel_style, stat_row, title_style,
};
use super::{detail_rect, list_rect, UiAction, UiContext};
use crate::model::HeistTarget;
use crate::state::BoardEntry;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

const ROW_HEIGHT: f32 = 78.0;

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    draw_board(ctx, actions);
    draw_detail(ctx, actions);
}

fn selected_entry<'a>(ctx: &'a UiContext<'a>) -> Option<&'a BoardEntry> {
    ctx.selected_target
        .and_then(|id| ctx.session.board_entry(id))
        .or_else(|| ctx.session.board.first())
}

fn draw_board(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let content = draw_panel(list_rect(), "Marks");
    if ctx.session.board.is_empty() {
        empty_notice(content, "Nothing worth taking this week.");
        return;
    }

    let mouse = ctx.mouse();
    let selected = selected_entry(ctx).map(|entry| entry.target_id.clone());
    let layout = GridLayout::new(content.x, content.y, content.w, 8.0, 1, ROW_HEIGHT);

    for (index, entry) in ctx.session.board.iter().enumerate() {
        let Some(target) = ctx.data.targets.get(&entry.target_id) else {
            continue;
        };
        let (x, y, w, h) = layout.get_item_rect(index, 0.0);
        let rect = Rect::new(x, y, w, h);
        if rect.bottom() > content.bottom() {
            break;
        }

        let is_selected = selected.as_deref() == Some(entry.target_id.as_str());
        if list_card(
            rect,
            is_selected,
            difficulty_color(target.difficulty),
            mouse,
        ) {
            actions.push(UiAction::SelectTarget(entry.target_id.clone()));
        }

        draw_ui_text_ex(
            &target.name,
            rect.x + 14.0,
            rect.y + 25.0,
            TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
        );
        draw_ui_text_ex(
            &format!(
                "{} · {} doors · {}",
                target.difficulty.label(),
                target.encounters.len(),
                if entry.cased { "cased" } else { "blind" }
            ),
            rect.x + 14.0,
            rect.y + 46.0,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );
        let board = &ctx.data.config.board;
        draw_text_right(
            &format_compact_money(entry.ripened_payout(target.potential_payout, board)),
            rect.right() - 14.0,
            rect.y + 25.0,
            TextStyle::new(16.0, Color::new(0.56, 0.82, 0.58, 1.0)),
        );
        let bonus = entry.payout_bonus_pct(board);
        draw_text_right(
            &if bonus > 0 {
                format!("{} wk left · ripe +{}%", entry.weeks_remaining, bonus)
            } else {
                format!("{} wk left", entry.weeks_remaining)
            },
            rect.right() - 14.0,
            rect.y + 46.0,
            TextStyle::new(
                14.0,
                if bonus > 0 {
                    Color::new(0.90, 0.72, 0.36, 1.0)
                } else {
                    dark::TEXT_DIM
                },
            ),
        );
    }
}

fn draw_detail(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let rect = detail_rect();
    let Some(entry) = selected_entry(ctx) else {
        draw_surface_with_title(rect, Some("The Job"), &panel_style(), title_style());
        empty_notice(rect, "Advance the week to see new work.");
        return;
    };
    let Some(target) = ctx.data.targets.get(&entry.target_id) else {
        return;
    };

    let content = draw_panel(rect, &target.name);
    draw_text_block(
        &target.description,
        content.x,
        content.y,
        content.w,
        40.0,
        15.0,
        3.0,
        dark::TEXT_DIM,
    );

    let summary_w = 240.0;
    draw_summary(
        Rect::new(
            content.right() - summary_w,
            content.y + 44.0,
            summary_w,
            150.0,
        ),
        ctx,
        target,
        entry,
    );
    draw_doors(
        Rect::new(
            content.x,
            content.y + 44.0,
            content.w - summary_w - 18.0,
            content.bottom() - content.y - 44.0,
        ),
        ctx,
        target,
        entry,
    );

    let cost = ctx.data.config.casing_cost;
    let can_case = !entry.cased && ctx.session.budget >= cost;
    let label = if entry.cased {
        "Cased".to_owned()
    } else {
        format!("Case the mark ({})", format_compact_money(cost))
    };
    if button_rect_tone_at(
        Rect::new(
            content.right() - 396.0,
            content.bottom() - 40.0,
            220.0,
            38.0,
        ),
        &label,
        can_case,
        ButtonTone::Primary,
        ctx.mouse(),
    ) {
        actions.push(UiAction::CaseTarget(entry.target_id.clone()));
    }

    let crew_ready = ctx.session.available_crew().count() > 0;
    if button_rect_tone_at(
        Rect::new(
            content.right() - 166.0,
            content.bottom() - 40.0,
            166.0,
            38.0,
        ),
        "Delegate",
        crew_ready,
        ButtonTone::Secondary,
        ctx.mouse(),
    ) {
        actions.push(UiAction::DelegateJob(entry.target_id.clone()));
    }
    if button_rect_tone_at(
        Rect::new(
            content.right() - 570.0,
            content.bottom() - 40.0,
            164.0,
            38.0,
        ),
        "Plan the job",
        crew_ready,
        ButtonTone::Positive,
        ctx.mouse(),
    ) {
        actions.push(UiAction::PlanJob(entry.target_id.clone()));
    }
}

fn draw_summary(rect: Rect, ctx: &UiContext<'_>, target: &HeistTarget, entry: &BoardEntry) {
    let board = &ctx.data.config.board;
    let ripe = entry.payout_bonus_pct(board);
    let rows: [(String, String); 5] = [
        (
            "Payout".to_owned(),
            if ripe > 0 {
                format!(
                    "{} (+{}%)",
                    format_money(entry.ripened_payout(target.potential_payout, board)),
                    ripe
                )
            } else {
                format_money(target.potential_payout)
            },
        ),
        (
            "Difficulty".to_owned(),
            if entry.door_penalty(board) > 0 {
                format!(
                    "{} · doors +{}",
                    target.difficulty.label(),
                    entry.door_penalty(board)
                )
            } else {
                target.difficulty.label().to_owned()
            },
        ),
        ("Notoriety".to_owned(), format!("+{}", target.notoriety)),
        (
            "Reputation".to_owned(),
            format!("needs {}", target.required_reputation),
        ),
        (
            "Window".to_owned(),
            format!("{} weeks", entry.weeks_remaining),
        ),
    ];

    for (index, (label, value)) in rows.iter().enumerate() {
        stat_row(
            Rect::new(rect.x, rect.y + index as f32 * 24.0, rect.w, 20.0),
            label,
            value,
            16.0,
            dark::TEXT,
        );
    }

    draw_ui_text_ex(
        "Conditions",
        rect.x,
        rect.y + 142.0,
        TextStyle::new(16.0, dark::TEXT_BRIGHT).params(),
    );

    let conditions: Vec<String> = target
        .environment
        .modifier_ids()
        .map(|id| {
            ctx.data
                .environment
                .get(id)
                .map(|modifier| modifier.name.clone())
                .unwrap_or_else(|| id.to_owned())
        })
        .collect();
    draw_text_block(
        &conditions.join(", "),
        rect.x,
        rect.y + 150.0,
        rect.w,
        60.0,
        14.0,
        3.0,
        dark::TEXT_DIM,
    );
}

fn draw_doors(rect: Rect, ctx: &UiContext<'_>, target: &HeistTarget, entry: &BoardEntry) {
    draw_ui_text_ex(
        "Doors",
        rect.x,
        rect.y + 14.0,
        TextStyle::new(17.0, dark::TEXT_BRIGHT).params(),
    );

    let heat = ctx.session.heat_dc_penalty(&ctx.data.config);
    let encounters = ctx.data.encounters_for(target);
    let layout = GridLayout::new(rect.x, rect.y + 26.0, rect.w, 8.0, 1, 62.0);

    for (index, encounter) in encounters.iter().enumerate() {
        let (x, y, w, h) = layout.get_item_rect(index, 0.0);
        let door = Rect::new(x, y, w, h);
        if door.bottom() > rect.bottom() {
            break;
        }

        draw_surface(
            door,
            &SurfaceStyle::new(Color::new(0.09, 0.10, 0.13, 1.0))
                .with_left_accent(3.0, dark::ACCENT)
                .with_border(1.0, Color::new(0.42, 0.48, 0.58, 0.35)),
        );
        draw_ui_text_ex(
            &format!("{}. {}", index + 1, encounter.name),
            door.x + 12.0,
            door.y + 23.0,
            TextStyle::new(16.0, dark::TEXT).params(),
        );
        draw_ui_text_ex(
            &format!(
                "{} · {}",
                encounter.primary_skill.label(),
                encounter.complexity.label()
            ),
            door.x + 12.0,
            door.y + 45.0,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );

        let dc_label = if entry.cased {
            format!("DC {}", encounter.difficulty)
        } else {
            "DC ??".to_owned()
        };
        draw_text_right(
            &dc_label,
            door.right() - 12.0,
            door.y + 23.0,
            TextStyle::new(
                16.0,
                if entry.cased {
                    dark::TEXT_BRIGHT
                } else {
                    dark::TEXT_DIM
                },
            ),
        );
        if entry.cased && heat > 0 {
            draw_text_right(
                &format!("heat +{}", heat),
                door.right() - 12.0,
                door.y + 45.0,
                TextStyle::new(13.0, Color::new(0.90, 0.52, 0.42, 1.0)),
            );
        }
    }
}
