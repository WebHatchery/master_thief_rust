//! The outfit's own books: what the week costs, how exposed the crew is, and
//! who the city is holding.
//!
//! Pillar 2 says every modifier is named and visible. The weekly bill and the
//! chance of a knock at the door are modifiers on the whole campaign, so they
//! get the same treatment as a die roll: the number, where it comes from, and
//! what it would cost to change it.

use super::chrome::stat_row;
use super::{UiAction, UiContext};
use crate::sim::{
    attention_chance, bribe_cost, safehouse_upkeep, weekly_outgoings, weeks_of_runway,
};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

const DANGER: Color = Color::new(0.90, 0.42, 0.36, 1.0);
const CAUTION: Color = Color::new(0.90, 0.66, 0.30, 1.0);
const CALM: Color = Color::new(0.56, 0.82, 0.58, 1.0);

pub fn draw(ctx: &UiContext<'_>, content: Rect, actions: &mut Vec<UiAction>) {
    let payroll = &ctx.data.config.payroll;
    let session = ctx.session;

    let upkeep = safehouse_upkeep(session, payroll);
    let due = weekly_outgoings(session, payroll);
    let runway = weeks_of_runway(session, payroll);

    section(content, "The week's bill");
    stat_row(
        row(content, 0),
        "Safehouse",
        &format_money(upkeep),
        15.0,
        dark::TEXT,
    );
    stat_row(
        row(content, 1),
        &format!("Retainers ({})", session.crew.len()),
        &format_money(due - upkeep),
        15.0,
        dark::TEXT,
    );
    stat_row(
        row(content, 2),
        "Every week",
        &format_money(due),
        16.0,
        dark::TEXT_BRIGHT,
    );
    stat_row(
        row(content, 3),
        "Quiet weeks affordable",
        &runway_label(runway),
        15.0,
        runway_color(runway),
    );

    let heat_y = content.y + 118.0;
    draw_heat(Rect::new(content.x, heat_y, content.w, 96.0), ctx, actions);
    draw_custody(
        Rect::new(
            content.x,
            heat_y + 104.0,
            content.w,
            content.bottom() - heat_y - 164.0,
        ),
        ctx,
        actions,
    );
    draw_the_way_out(
        Rect::new(content.x, content.bottom() - 52.0, content.w, 52.0),
        ctx,
        actions,
    );
}

fn draw_heat(rect: Rect, ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let session = ctx.session;
    let law = &ctx.data.config.law;
    let risk = attention_chance(session, law);

    section(rect, "The city");
    stat_row(
        Rect::new(rect.x, rect.y + 22.0, rect.w, 20.0),
        "Heat",
        &format!(
            "{} (+{} DC)",
            session.heat,
            session.heat_dc_penalty(&ctx.data.config)
        ),
        15.0,
        if session.heat_dc_penalty(&ctx.data.config) > 0 {
            DANGER
        } else {
            dark::TEXT
        },
    );
    stat_row(
        Rect::new(rect.x, rect.y + 44.0, rect.w, 20.0),
        if session.surveillance_weeks > 0 {
            "Tailed — risk of a visit"
        } else {
            "Risk of a visit"
        },
        &format!("{:.0}%", risk * 100.0),
        15.0,
        risk_color(risk),
    );

    let cost = bribe_cost(session, law);
    let affordable = session.heat > 0 && session.budget >= cost;
    if button_rect_tone_at(
        Rect::new(rect.x, rect.y + 68.0, rect.w, 26.0),
        &format!(
            "Grease palms — {} for -{} heat",
            format_compact_money(cost),
            law.bribe_heat_relief
        ),
        affordable,
        ButtonTone::Primary,
        ctx.mouse(),
    ) {
        actions.push(UiAction::GreasePalms);
    }
}

/// The last decision the campaign asks for. Shown with what walking away is
/// worth today, because a campaign should not end on a number nobody saw.
fn draw_the_way_out(rect: Rect, ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    if let Some(done) = &ctx.session.retired {
        draw_ui_text_ex(
            &done.headline(),
            rect.x,
            rect.y + 14.0,
            TextStyle::new(15.0, CALM).params(),
        );
        return;
    }

    let quoted = crate::sim::retirement::quote(ctx.session, ctx.data, &ctx.data.config);
    stat_row(
        Rect::new(rect.x, rect.y, rect.w, 20.0),
        "Walk away with",
        &format_compact_money(quoted.take()),
        15.0,
        CALM,
    );
    if button_rect_tone_at(
        Rect::new(rect.x, rect.y + 24.0, rect.w, 24.0),
        "Get out while you can",
        true,
        ButtonTone::Danger,
        ctx.mouse(),
    ) {
        actions.push(UiAction::Retire);
    }
}

fn draw_custody(rect: Rect, ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let session = ctx.session;
    section(rect, "Held");

    if session.custody.is_empty() {
        draw_ui_text_ex(
            "Nobody is in a cell. Keep it that way.",
            rect.x,
            rect.y + 40.0,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );
        draw_wavering(rect, ctx, actions);
        return;
    }

    for (index, record) in session.custody.iter().enumerate() {
        let y = rect.y + 26.0 + index as f32 * 46.0;
        if y + 42.0 > rect.bottom() {
            break;
        }
        draw_ui_text_ex(
            &format!("{} — taken week {}", record.member.name, record.week_taken),
            rect.x,
            y + 14.0,
            TextStyle::new(14.0, DANGER).params(),
        );
        if button_rect_tone_at(
            Rect::new(rect.x, y + 20.0, rect.w, 22.0),
            &format!("Post bail — {}", format_compact_money(record.bail)),
            session.budget >= record.bail,
            ButtonTone::Positive,
            ctx.mouse(),
        ) {
            actions.push(UiAction::PostBail(record.member.id.clone()));
        }
    }
}

/// Anybody threatening to walk, and the price of talking them round. Only ever
/// drawn when there is somebody to draw.
fn draw_wavering(rect: Rect, ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let payroll = &ctx.data.config.payroll;
    let wavering: Vec<_> = ctx
        .session
        .crew
        .iter()
        .filter(|member| {
            member.condition.notice_given
                || member.condition.loyalty <= payroll.notice_loyalty_threshold * 2
        })
        .collect();

    if wavering.is_empty() {
        return;
    }

    draw_ui_text_ex(
        "Unhappy",
        rect.x,
        rect.y + 76.0,
        TextStyle::new(16.0, dark::TEXT_BRIGHT).params(),
    );

    for (index, member) in wavering.iter().enumerate() {
        let y = rect.y + 84.0 + index as f32 * 46.0;
        if y + 42.0 > rect.bottom() {
            break;
        }
        let cost = crate::sim::bonus_cost(member, payroll);
        draw_ui_text_ex(
            &format!(
                "{} — loyalty {}{}",
                member.name,
                member.condition.loyalty,
                if member.condition.notice_given {
                    ", has given notice"
                } else {
                    ""
                }
            ),
            rect.x,
            y + 14.0,
            TextStyle::new(
                14.0,
                if member.condition.notice_given {
                    DANGER
                } else {
                    CAUTION
                },
            )
            .params(),
        );
        if button_rect_tone_at(
            Rect::new(rect.x, y + 20.0, rect.w, 22.0),
            &format!(
                "Pay a bonus — {} for +{} loyalty",
                format_compact_money(cost),
                payroll.bonus_loyalty_restored
            ),
            ctx.session.budget >= cost,
            ButtonTone::Secondary,
            ctx.mouse(),
        ) {
            actions.push(UiAction::PayBonus(member.id.clone()));
        }
    }
}

fn runway_label(weeks: i64) -> String {
    if weeks >= 99 {
        "99+".to_owned()
    } else {
        format!("{}", weeks)
    }
}

fn runway_color(weeks: i64) -> Color {
    if weeks <= 1 {
        DANGER
    } else if weeks <= 4 {
        CAUTION
    } else {
        CALM
    }
}

fn risk_color(risk: f32) -> Color {
    if risk >= 0.25 {
        DANGER
    } else if risk > 0.0 {
        CAUTION
    } else {
        CALM
    }
}

fn row(content: Rect, index: usize) -> Rect {
    Rect::new(
        content.x,
        content.y + 22.0 + index as f32 * 23.0,
        content.w,
        20.0,
    )
}

fn section(rect: Rect, title: &str) {
    draw_ui_text_ex(
        title,
        rect.x,
        rect.y + 14.0,
        TextStyle::new(16.0, dark::TEXT_BRIGHT).params(),
    );
}
