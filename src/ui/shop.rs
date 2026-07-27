//! The outfitter: what money can buy, and who is carrying what.

use super::chrome::{draw_panel, empty_notice, list_card, stat_row};
use super::{content_rect, UiAction, UiContext};
use crate::model::{EquipmentDef, EquipmentRarity, EquipmentSlot, Skill};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

const ROW_HEIGHT: f32 = 78.0;
/// The outfitter needs more room than the browsing screens' narrow list.
const CATALOGUE_WIDTH: f32 = 632.0;

fn catalogue_rect() -> Rect {
    let content = content_rect();
    Rect::new(content.x, content.y, CATALOGUE_WIDTH, content.h)
}

fn lockup_rect() -> Rect {
    let content = content_rect();
    Rect::new(
        content.x + CATALOGUE_WIDTH + 14.0,
        content.y,
        content.w - CATALOGUE_WIDTH - 14.0,
        content.h,
    )
}

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    draw_catalogue(ctx, actions);
    draw_lockup(ctx, actions);
}

fn rarity_color(rarity: EquipmentRarity) -> Color {
    match rarity {
        EquipmentRarity::Basic => Color::new(0.62, 0.65, 0.70, 1.0),
        EquipmentRarity::Improved => Color::new(0.42, 0.74, 0.48, 1.0),
        EquipmentRarity::Advanced => Color::new(0.40, 0.62, 0.92, 1.0),
        EquipmentRarity::Masterwork => Color::new(0.70, 0.48, 0.92, 1.0),
        EquipmentRarity::Legendary => Color::new(0.95, 0.72, 0.30, 1.0),
    }
}

fn bonus_line(item: &EquipmentDef) -> String {
    let mut parts: Vec<String> = Skill::ALL
        .into_iter()
        .filter(|skill| item.skill_bonus(*skill) != 0)
        .map(|skill| format!("{} {:+}", skill.label(), item.skill_bonus(skill)))
        .collect();

    for kind in crate::model::AttributeKind::ALL {
        let value = item.attribute_bonuses.get(kind);
        if value != 0 {
            parts.push(format!("{} {:+}", kind.short_label(), value));
        }
    }

    if parts.is_empty() {
        "No bonuses".to_owned()
    } else {
        parts.join("  ")
    }
}

fn draw_catalogue(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let content = draw_panel(catalogue_rect(), "The Outfitter");
    let mouse = ctx.mouse();

    // A shop stocks what its customer can plausibly buy. The best thing the
    // outfit can afford leads; anything far beyond the budget is not displayed
    // at all, and the count of what is missing says so plainly.
    let ceiling = ctx.session.budget.max(1_000);
    let mut catalogue: Vec<&EquipmentDef> = ctx
        .data
        .equipment
        .iter()
        .map(|(_, item)| item)
        .filter(|item| item.cost <= ceiling)
        .collect();
    catalogue.sort_by_key(|item| (-item.cost, item.id.clone()));
    let out_of_reach = ctx.data.equipment.len() - catalogue.len();

    let mut shown = 0usize;
    let layout = GridLayout::new(content.x, content.y, content.w, 8.0, 2, ROW_HEIGHT);
    for (index, item) in catalogue.iter().enumerate() {
        let (x, y, w, h) = layout.get_item_rect(index, 0.0);
        let rect = Rect::new(x, y, w, h);
        if rect.bottom() > content.bottom() - 18.0 {
            break;
        }
        shown = index + 1;

        let affordable = ctx.session.budget >= item.cost;
        list_card(rect, false, rarity_color(item.rarity), mouse);
        draw_ui_text_ex(
            &item.name,
            rect.x + 14.0,
            rect.y + 23.0,
            TextStyle::new(16.0, dark::TEXT_BRIGHT).params(),
        );
        draw_ui_text_ex(
            &format!("{} · {}", item.slot.label(), item.rarity.label()),
            rect.x + 14.0,
            rect.y + 42.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
        draw_ui_text_ex(
            &bonus_line(item),
            rect.x + 14.0,
            rect.y + 62.0,
            TextStyle::new(13.0, Color::new(0.62, 0.68, 0.80, 1.0)).params(),
        );
        draw_text_right(
            &format_compact_money(item.cost),
            rect.right() - 14.0,
            rect.y + 23.0,
            TextStyle::new(
                15.0,
                if affordable {
                    Color::new(0.56, 0.82, 0.58, 1.0)
                } else {
                    Color::new(0.86, 0.46, 0.42, 1.0)
                },
            ),
        );

        if button_rect_tone_at(
            Rect::new(rect.right() - 84.0, rect.bottom() - 30.0, 72.0, 24.0),
            "Buy",
            affordable,
            ButtonTone::Positive,
            mouse,
        ) {
            actions.push(UiAction::BuyItem(item.id.clone()));
        }
    }

    let below_fold = catalogue.len().saturating_sub(shown);
    let mut notes = Vec::new();
    if below_fold > 0 {
        notes.push(format!("{} more in stock", below_fold));
    }
    if out_of_reach > 0 {
        notes.push(format!("{} beyond the outfit's means", out_of_reach));
    }
    if !notes.is_empty() {
        draw_ui_text_ex(
            &notes.join(" · "),
            content.x,
            content.bottom() + 4.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
    }
}

fn draw_lockup(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let holder = ctx
        .selected_member
        .and_then(|id| ctx.session.member(id))
        .or_else(|| ctx.session.crew.first());
    let title = match holder {
        Some(member) => format!("The Lockup — issuing to {}", member.name),
        None => "The Lockup".to_owned(),
    };
    let content = draw_panel(lockup_rect(), &title);

    let Some(member) = holder else {
        empty_notice(content, "Nobody on the payroll to carry it.");
        return;
    };

    let mouse = ctx.mouse();
    draw_ui_text_ex(
        "Carried",
        content.x,
        content.y + 14.0,
        TextStyle::new(17.0, dark::TEXT_BRIGHT).params(),
    );

    for (index, slot) in EquipmentSlot::ALL.iter().enumerate() {
        let rect = Rect::new(
            content.x,
            content.y + 24.0 + index as f32 * 46.0,
            content.w,
            40.0,
        );
        let item = member
            .equipment
            .get(*slot)
            .and_then(|id| ctx.data.equipment.get(id));

        draw_surface(
            rect,
            &SurfaceStyle::new(Color::new(0.09, 0.10, 0.13, 1.0)).with_border(
                1.0,
                match item {
                    Some(def) => rarity_color(def.rarity),
                    None => Color::new(0.35, 0.38, 0.45, 0.35),
                },
            ),
        );
        draw_ui_text_ex(
            slot.label(),
            rect.x + 10.0,
            rect.y + 16.0,
            TextStyle::new(12.0, dark::TEXT_DIM).params(),
        );
        draw_ui_text_ex(
            item.map(|def| def.name.as_str()).unwrap_or("— empty —"),
            rect.x + 10.0,
            rect.y + 33.0,
            TextStyle::new(
                14.0,
                if item.is_some() {
                    dark::TEXT
                } else {
                    dark::TEXT_DIM
                },
            )
            .params(),
        );

        if item.is_some()
            && button_rect_tone_at(
                Rect::new(rect.right() - 96.0, rect.y + 8.0, 86.0, 24.0),
                "Take back",
                true,
                ButtonTone::Danger,
                mouse,
            )
        {
            actions.push(UiAction::UnequipSlot {
                member_id: member.id.clone(),
                slot: *slot,
            });
        }
    }

    draw_ui_text_ex(
        "On the shelf",
        content.x,
        content.y + 264.0,
        TextStyle::new(17.0, dark::TEXT_BRIGHT).params(),
    );
    let share = crate::sim::fence::share_at(ctx.session.heat, &ctx.data.config.fence);
    draw_text_right(
        &format!(
            "The fence pays {:.0}% of list{}",
            share * 100.0,
            if share <= ctx.data.config.fence.min_share {
                " — too hot for better"
            } else {
                ""
            }
        ),
        content.right(),
        content.y + 264.0,
        TextStyle::new(13.0, Color::new(0.88, 0.72, 0.44, 1.0)),
    );

    let shelf = ctx.session.unassigned_inventory(ctx.data);
    let list = Rect::new(
        content.x,
        content.y + 274.0,
        content.w,
        content.bottom() - content.y - 274.0,
    );
    if shelf.is_empty() {
        empty_notice(list, "Everything the outfit owns is being carried.");
        return;
    }

    let layout = GridLayout::new(list.x, list.y, list.w, 8.0, 1, 62.0);
    for (index, item) in shelf.iter().enumerate() {
        let (x, y, w, h) = layout.get_item_rect(index, 0.0);
        let rect = Rect::new(x, y, w, h);
        if rect.bottom() > list.bottom() {
            break;
        }

        let usable = member.progression.level >= item.required_level
            && (item.required_class.is_empty() || item.required_class.contains(&member.class));

        list_card(rect, false, rarity_color(item.rarity), mouse);
        draw_ui_text_ex(
            &item.name,
            rect.x + 12.0,
            rect.y + 22.0,
            TextStyle::new(15.0, dark::TEXT_BRIGHT).params(),
        );
        stat_row(
            Rect::new(rect.x + 12.0, rect.y + 26.0, rect.w - 110.0, 18.0),
            item.slot.label(),
            "",
            13.0,
            dark::TEXT_DIM,
        );
        if !usable {
            draw_ui_text_ex(
                &format!("needs level {}", item.required_level),
                rect.x + 12.0,
                rect.y + 50.0,
                TextStyle::new(12.0, Color::new(0.86, 0.52, 0.44, 1.0)).params(),
            );
        }

        if button_rect_tone_at(
            Rect::new(rect.right() - 96.0, rect.y + 18.0, 84.0, 26.0),
            "Issue",
            usable,
            ButtonTone::Primary,
            mouse,
        ) {
            actions.push(UiAction::EquipItem {
                member_id: member.id.clone(),
                item_id: item.id.clone(),
            });
        }

        // The one way money comes in that is not a job — priced so the player
        // can see how much the city's attention is costing them (pillar 2).
        let offer = crate::sim::fence_quote(ctx.session, item, &ctx.data.config.fence);
        if button_rect_tone_at(
            Rect::new(rect.right() - 190.0, rect.y + 18.0, 88.0, 26.0),
            &format!("Sell {}", format_compact_money(offer.price)),
            true,
            ButtonTone::Secondary,
            mouse,
        ) {
            actions.push(UiAction::SellItem(item.id.clone()));
        }
    }
}
