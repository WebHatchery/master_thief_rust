//! Who is asking for work this week.

use super::chrome::{empty_notice, list_card, rarity_color};
use super::{UiAction, UiContext};
use crate::model::CrewMember;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

const ROW_HEIGHT: f32 = 86.0;

pub fn draw(ctx: &UiContext<'_>, content: Rect, actions: &mut Vec<UiAction>) {
    if ctx.session.recruits.is_empty() {
        empty_notice(content, "Nobody is asking for work this week.");
        return;
    }

    let mouse = ctx.mouse();
    let layout = GridLayout::new(content.x, content.y, content.w, 8.0, 1, ROW_HEIGHT);

    for (index, recruit_id) in ctx.session.recruits.iter().enumerate() {
        let Some(recruit) = ctx.data.crew_pool.get(recruit_id) else {
            continue;
        };
        let (x, y, w, h) = layout.get_item_rect(index, 0.0);
        let rect = Rect::new(x, y, w, h);
        if rect.bottom() > content.bottom() {
            break;
        }

        let fee = ctx.session.hire_fee(recruit, &ctx.data.config);
        let affordable = ctx.session.budget >= fee;
        list_card(rect, false, rarity_color(recruit.rarity), mouse);
        draw_recruit(ctx, rect, recruit, fee, affordable);

        if button_rect_tone_at(
            Rect::new(rect.right() - 96.0, rect.bottom() - 32.0, 84.0, 26.0),
            "Hire",
            affordable,
            ButtonTone::Positive,
            mouse,
        ) {
            actions.push(UiAction::HireRecruit(recruit.id.clone()));
        }
    }
}

fn draw_recruit(ctx: &UiContext<'_>, rect: Rect, recruit: &CrewMember, fee: i64, affordable: bool) {
    draw_ui_text_ex(
        &recruit.name,
        rect.x + 14.0,
        rect.y + 24.0,
        TextStyle::new(17.0, dark::TEXT_BRIGHT).params(),
    );
    draw_ui_text_ex(
        &format!(
            "{} · {} · {}",
            recruit.specialty,
            recruit.class.label(),
            recruit.rarity.label()
        ),
        rect.x + 14.0,
        rect.y + 44.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );
    draw_ui_text_ex(
        recruit.specialty_skill.label(),
        rect.x + 14.0,
        rect.y + 66.0,
        TextStyle::new(13.0, rarity_color(recruit.rarity)).params(),
    );
    draw_text_right(
        &if fee > recruit.hire_cost {
            // Say so, rather than quietly charging more than the dossier says.
            format!(
                "{} to sign (+{}% known)",
                format_compact_money(fee),
                ((fee - recruit.hire_cost) * 100 / recruit.hire_cost.max(1))
            )
        } else {
            format!("{} to sign", format_compact_money(fee))
        },
        rect.right() - 14.0,
        rect.y + 24.0,
        TextStyle::new(
            16.0,
            if affordable {
                Color::new(0.56, 0.82, 0.58, 1.0)
            } else {
                Color::new(0.86, 0.46, 0.42, 1.0)
            },
        ),
    );

    // The half of the question the screen never asked. Signing them is one
    // payment; keeping them is every week from here.
    let payroll = &ctx.data.config.payroll;
    draw_text_right(
        &format!(
            "{}/wk to keep",
            format_compact_money(crate::sim::retainer_for(recruit, payroll))
        ),
        rect.right() - 14.0,
        rect.y + 46.0,
        TextStyle::new(14.0, Color::new(0.88, 0.72, 0.44, 1.0)),
    );

    let after = crate::sim::runway_after_hiring(ctx.session, payroll, recruit, fee);
    // Left of the Hire button, which owns the bottom-right corner of the row.
    draw_text_right(
        &format!("leaves {}wk runway", after.min(99)),
        rect.right() - 106.0,
        rect.y + 70.0,
        TextStyle::new(
            13.0,
            match after {
                0..=1 => Color::new(0.90, 0.42, 0.38, 1.0),
                2..=4 => Color::new(0.90, 0.66, 0.30, 1.0),
                _ => dark::TEXT_DIM,
            },
        ),
    );
}
