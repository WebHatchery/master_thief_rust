//! The crew screen: the roster as personnel dossiers, not portraits.

use super::chrome::{
    draw_panel, empty_notice, list_card, panel_style, rarity_color, stat_row, title_style,
};
use super::{detail_rect, list_rect, CrewTab, UiAction, UiContext};
use crate::model::{AttributeKind, CrewMember, EquipmentSlot, Skill};
use crate::rules::attributes::{derived_stats, equipped_attributes, equipped_skills, power_level};
use crate::sim::retainer_for;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

const ROW_HEIGHT: f32 = 76.0;

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let content = draw_panel(list_rect(), "The Crew");
    let strip = Rect::new(content.x, content.y, content.w, 30.0);
    let body = Rect::new(content.x, content.y + 38.0, content.w, content.h - 38.0);

    let labels: Vec<&str> = CrewTab::ALL.iter().map(|tab| tab.label()).collect();
    let active = CrewTab::ALL
        .iter()
        .position(|tab| *tab == ctx.crew_tab)
        .unwrap_or(0);
    if let Some(index) = tab_bar_styled_at(
        strip,
        &labels,
        active,
        TabOrientation::Horizontal,
        &TabStyle::default(),
        ctx.mouse(),
    ) {
        actions.push(UiAction::ShowCrewTab(CrewTab::ALL[index]));
    }

    match ctx.crew_tab {
        CrewTab::Roster => draw_roster(ctx, body, actions),
        CrewTab::ForHire => super::hiring::draw(ctx, body, actions),
        CrewTab::Outfit => super::outfit::draw(ctx, body, actions),
    }
    draw_dossier(ctx, actions);
}

fn selected_member<'a>(ctx: &'a UiContext<'a>) -> Option<&'a CrewMember> {
    ctx.selected_member
        .and_then(|id| ctx.session.member(id))
        .or_else(|| ctx.session.crew.first())
}

fn draw_roster(ctx: &UiContext<'_>, content: Rect, actions: &mut Vec<UiAction>) {
    if ctx.session.crew.is_empty() {
        empty_notice(content, "Nobody on the books.");
        return;
    }

    let mouse = ctx.mouse();
    let selected = selected_member(ctx).map(|member| member.id.clone());
    let layout = GridLayout::new(content.x, content.y, content.w, 8.0, 1, ROW_HEIGHT);

    for (index, member) in ctx.session.crew.iter().enumerate() {
        let (x, y, w, h) = layout.get_item_rect(index, 0.0);
        let rect = Rect::new(x, y, w, h);
        if rect.bottom() > content.bottom() {
            break;
        }

        let is_selected = selected.as_deref() == Some(member.id.as_str());
        if list_card(rect, is_selected, rarity_color(member.rarity), mouse) {
            actions.push(UiAction::SelectMember(member.id.clone()));
        }

        draw_ui_text_ex(
            &member.name,
            rect.x + 14.0,
            rect.y + 25.0,
            TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
        );
        draw_ui_text_ex(
            &format!(
                "{} · {}/wk",
                member.specialty,
                format_compact_money(retainer_for(member, &ctx.data.config.payroll))
            ),
            rect.x + 14.0,
            rect.y + 46.0,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );
        draw_text_right(
            &format!("Lv {}", member.progression.level),
            rect.right() - 14.0,
            rect.y + 25.0,
            TextStyle::new(15.0, rarity_color(member.rarity)),
        );

        let condition = condition_summary(member);
        draw_text_right(
            &condition.0,
            rect.right() - 14.0,
            rect.y + 46.0,
            TextStyle::new(14.0, condition.1),
        );

        meter(
            Rect::new(rect.x + 14.0, rect.y + 56.0, rect.w - 100.0, 8.0),
            member.condition.fatigue as f32,
            100.0,
            fatigue_color(member.condition.fatigue),
            None,
        );
    }
}

fn condition_summary(member: &CrewMember) -> (String, Color) {
    if member.condition.notice_given {
        ("Notice".to_owned(), Color::new(0.94, 0.36, 0.34, 1.0))
    } else if !member.condition.injuries.is_empty() {
        (
            format!("{} injured", member.condition.injuries.len()),
            Color::new(0.90, 0.42, 0.36, 1.0),
        )
    } else if !member.condition.is_fit_for_work() {
        ("Spent".to_owned(), Color::new(0.90, 0.62, 0.30, 1.0))
    } else {
        ("Ready".to_owned(), Color::new(0.46, 0.76, 0.52, 1.0))
    }
}

fn fatigue_color(fatigue: i32) -> Color {
    if fatigue > 80 {
        Color::new(0.88, 0.32, 0.32, 1.0)
    } else if fatigue > 50 {
        Color::new(0.90, 0.66, 0.30, 1.0)
    } else {
        Color::new(0.36, 0.62, 0.86, 1.0)
    }
}

fn draw_dossier(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let rect = detail_rect();
    let Some(member) = selected_member(ctx) else {
        draw_surface_with_title(rect, Some("Dossier"), &panel_style(), title_style());
        empty_notice(rect, "Hire somebody first.");
        return;
    };

    let content = draw_panel(rect, &format!("{} — {}", member.name, member.specialty));
    let loadout = ctx.session.loadout(member, ctx.data);
    let attributes = equipped_attributes(member, &loadout);
    let skills = equipped_skills(member, &loadout);
    let stats = derived_stats(&attributes, member.progression.level);

    draw_text_block(
        &member.background,
        content.x,
        content.y,
        content.w,
        40.0,
        15.0,
        3.0,
        dark::TEXT_DIM,
    );

    let columns_y = content.y + 48.0;
    let column_w = (content.w - 24.0) / 3.0;

    draw_attributes(
        Rect::new(content.x, columns_y, column_w, 190.0),
        ctx,
        member,
        &attributes,
        actions,
    );
    draw_skills(
        Rect::new(content.x + column_w + 12.0, columns_y, column_w, 190.0),
        ctx,
        member,
        &skills,
        actions,
    );
    draw_condition(
        Rect::new(
            content.x + (column_w + 12.0) * 2.0,
            columns_y,
            column_w,
            190.0,
        ),
        ctx,
        member,
        &stats,
        actions,
    );

    let kit_y = columns_y + 202.0;
    draw_kit(
        Rect::new(content.x, kit_y, content.w, content.bottom() - kit_y),
        ctx,
        member,
        power_level(member, &loadout),
    );
}

fn draw_attributes(
    rect: Rect,
    ctx: &UiContext<'_>,
    member: &CrewMember,
    attributes: &crate::model::Attributes,
    actions: &mut Vec<UiAction>,
) {
    let points = member.progression.attribute_points;
    section_title(rect, "Attributes");
    if points > 0 {
        draw_text_right(
            &format!("{} to spend", points),
            rect.right(),
            rect.y + 16.0,
            TextStyle::new(13.0, dark::ACCENT),
        );
    }

    for (index, kind) in AttributeKind::ALL.iter().enumerate() {
        let score = attributes.get(*kind);
        let modifier = crate::rules::attribute_modifier(score);
        let row = Rect::new(rect.x, rect.y + 26.0 + index as f32 * 24.0, rect.w, 20.0);
        let spendable = points > 0 && member.attributes.get(*kind) < 20;

        stat_row(
            Rect::new(
                row.x,
                row.y,
                row.w - if spendable { 26.0 } else { 0.0 },
                row.h,
            ),
            kind.short_label(),
            &format!("{} ({:+})", score, modifier),
            16.0,
            dark::TEXT,
        );
        if spendable
            && button_rect_tone_at(
                Rect::new(row.right() - 22.0, row.y + 1.0, 22.0, 18.0),
                "+",
                true,
                ButtonTone::Positive,
                ctx.mouse(),
            )
        {
            actions.push(UiAction::SpendAttribute {
                member_id: member.id.clone(),
                kind: *kind,
            });
        }
    }
}

fn draw_skills(
    rect: Rect,
    ctx: &UiContext<'_>,
    member: &CrewMember,
    skills: &crate::model::Skills,
    actions: &mut Vec<UiAction>,
) {
    let points = member.progression.skill_points;
    section_title(rect, "Skills");
    if points > 0 {
        draw_text_right(
            &format!("{} to spend", points),
            rect.right(),
            rect.y + 16.0,
            TextStyle::new(13.0, dark::ACCENT),
        );
    }

    for (index, skill) in Skill::ALL.iter().enumerate() {
        let is_specialty = *skill == member.specialty_skill;
        let label = if is_specialty {
            format!("{} *", skill.label())
        } else {
            skill.label().to_owned()
        };
        let row = Rect::new(rect.x, rect.y + 26.0 + index as f32 * 24.0, rect.w, 20.0);

        stat_row(
            Rect::new(
                row.x,
                row.y,
                row.w - if points > 0 { 26.0 } else { 0.0 },
                row.h,
            ),
            &label,
            &skills.get(*skill).to_string(),
            16.0,
            if is_specialty {
                dark::ACCENT
            } else {
                dark::TEXT
            },
        );
        if points > 0
            && button_rect_tone_at(
                Rect::new(row.right() - 22.0, row.y + 1.0, 22.0, 18.0),
                "+",
                true,
                ButtonTone::Positive,
                ctx.mouse(),
            )
        {
            actions.push(UiAction::SpendSkill {
                member_id: member.id.clone(),
                skill: *skill,
            });
        }
    }
}

fn draw_condition(
    rect: Rect,
    ctx: &UiContext<'_>,
    member: &CrewMember,
    stats: &crate::model::DerivedStats,
    actions: &mut Vec<UiAction>,
) {
    section_title(rect, "Condition");
    let condition = &member.condition;

    // What this hand costs to keep, beside the state they are in — the two
    // numbers a fixer weighs against each other every week.
    draw_text_right(
        &format!(
            "{}/wk",
            format_compact_money(retainer_for(member, &ctx.data.config.payroll))
        ),
        rect.right(),
        rect.y + 16.0,
        TextStyle::new(14.0, dark::TEXT_DIM),
    );

    meter(
        Rect::new(rect.x, rect.y + 30.0, rect.w, 20.0),
        condition.fatigue as f32,
        100.0,
        fatigue_color(condition.fatigue),
        Some(&format!("Fatigue {}", condition.fatigue)),
    );
    meter(
        Rect::new(rect.x, rect.y + 58.0, rect.w, 20.0),
        condition.loyalty as f32,
        100.0,
        Color::new(0.40, 0.66, 0.88, 1.0),
        Some(&format!(
            "Loyalty {}{}",
            condition.loyalty,
            if condition.notice_given {
                " — notice given"
            } else if condition.weeks_unpaid > 0 {
                " — unpaid"
            } else {
                ""
            }
        )),
    );

    stat_row(
        Rect::new(rect.x, rect.y + 84.0, rect.w, 20.0),
        "Health",
        &stats.health.to_string(),
        15.0,
        dark::TEXT,
    );
    stat_row(
        Rect::new(rect.x, rect.y + 106.0, rect.w, 20.0),
        "Initiative",
        &format!("{:+}", stats.initiative),
        15.0,
        dark::TEXT,
    );
    stat_row(
        Rect::new(rect.x, rect.y + 128.0, rect.w, 20.0),
        "Mastery",
        &format!("{}/10", member.progression.mastery_level),
        15.0,
        dark::TEXT,
    );

    let injuries = if condition.injuries.is_empty() {
        "No active injuries".to_owned()
    } else {
        condition
            .injuries
            .iter()
            .map(|injury| injury.description.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    draw_text_block(
        &injuries,
        rect.x,
        rect.y + 150.0,
        rect.w,
        32.0,
        14.0,
        2.0,
        Color::new(0.86, 0.50, 0.44, 1.0),
    );

    // Waiting an injury out is free and slow; a doctor is neither (GDD 4).
    let quoted = crate::sim::treatment_quote(condition, &ctx.data.config.treatment);
    if quoted.is_needed()
        && button_rect_tone_at(
            Rect::new(rect.x, rect.y + 176.0, rect.w, 22.0),
            &format!(
                "Treat — {} saves {} wk",
                format_compact_money(quoted.cost),
                quoted.weeks_saved
            ),
            ctx.session.budget >= quoted.cost,
            ButtonTone::Positive,
            ctx.mouse(),
        )
    {
        actions.push(UiAction::TreatInjuries(member.id.clone()));
    }
}

fn draw_kit(rect: Rect, ctx: &UiContext<'_>, member: &CrewMember, power: i32) {
    section_title(rect, "Kit");
    draw_text_right(
        &format!("Power {}", power),
        rect.right(),
        rect.y + 16.0,
        TextStyle::new(15.0, dark::TEXT_DIM),
    );

    let slot_w = (rect.w - 4.0 * 8.0) / 5.0;
    for (index, slot) in EquipmentSlot::ALL.iter().enumerate() {
        let slot_rect = Rect::new(
            rect.x + index as f32 * (slot_w + 8.0),
            rect.y + 28.0,
            slot_w,
            56.0,
        );
        let item = member
            .equipment
            .get(*slot)
            .and_then(|id| ctx.data.equipment.get(id));

        draw_surface(
            slot_rect,
            &SurfaceStyle::new(Color::new(0.09, 0.10, 0.13, 1.0)).with_border(
                1.0,
                if item.is_some() {
                    Color::new(0.45, 0.55, 0.72, 0.7)
                } else {
                    Color::new(0.35, 0.38, 0.45, 0.35)
                },
            ),
        );
        draw_ui_text_ex(
            slot.label(),
            slot_rect.x + 10.0,
            slot_rect.y + 20.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
        draw_ui_text_ex(
            item.map(|def| def.name.as_str()).unwrap_or("— empty —"),
            slot_rect.x + 10.0,
            slot_rect.y + 42.0,
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
    }

    draw_text_block(
        &format!("Special: {}", member.special_ability),
        rect.x,
        rect.y + 92.0,
        rect.w,
        34.0,
        15.0,
        3.0,
        dark::TEXT_DIM,
    );

    if !member.personality_traits.is_empty() {
        draw_ui_text_ex(
            &member.personality_traits.join(" · "),
            rect.x,
            rect.y + 132.0,
            TextStyle::new(14.0, Color::new(0.62, 0.68, 0.82, 1.0)).params(),
        );
    }

    draw_chemistry(rect, ctx, member);
}

/// How this hand gets on with everybody else on the payroll (GDD 5.5).
fn draw_chemistry(rect: Rect, ctx: &UiContext<'_>, member: &CrewMember) {
    let mut lines: Vec<(String, i32)> = ctx
        .session
        .crew
        .iter()
        .filter(|other| other.id != member.id)
        .map(|other| {
            (
                other.name.clone(),
                ctx.session.chemistry.get(&member.id, &other.id),
            )
        })
        .filter(|(_, value)| *value != 0)
        .collect();
    lines.sort_by_key(|(name, value)| (-value, name.clone()));

    if lines.is_empty() {
        return;
    }

    draw_ui_text_ex(
        "Chemistry",
        rect.x,
        rect.y + 158.0,
        TextStyle::new(15.0, dark::TEXT_BRIGHT).params(),
    );
    let summary = lines
        .iter()
        .take(4)
        .map(|(name, value)| {
            // A partnership is the one worth calling out: it buys dice at every
            // door and costs a premium on the cut (GDD 5.5).
            let mood = if ctx.session.chemistry.refuses(&member.id, name) {
                "refuses "
            } else if *value >= crate::rules::chemistry::PARTNERSHIP {
                "pair +"
            } else if *value > 0 {
                "+"
            } else {
                ""
            };
            format!("{} {}{}", name, mood, value)
        })
        .collect::<Vec<_>>()
        .join("   ");
    draw_ui_text_ex(
        &summary,
        rect.x,
        rect.y + 178.0,
        TextStyle::new(13.0, Color::new(0.66, 0.72, 0.84, 1.0)).params(),
    );
}

fn section_title(rect: Rect, text: &str) {
    draw_ui_text_ex(
        text,
        rect.x,
        rect.y + 16.0,
        TextStyle::new(17.0, dark::TEXT_BRIGHT).params(),
    );
}
