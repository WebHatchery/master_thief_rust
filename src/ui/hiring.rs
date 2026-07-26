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

        let affordable = ctx.session.budget >= recruit.hire_cost;
        list_card(rect, false, rarity_color(recruit.rarity), mouse);
        draw_recruit(rect, recruit, affordable);

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

fn draw_recruit(rect: Rect, recruit: &CrewMember, affordable: bool) {
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
        &format_compact_money(recruit.hire_cost),
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
}
