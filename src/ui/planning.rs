//! The planning screen — the richest screen in the game, by design.
//!
//! Left: the job as a run of connected doors. Right: every hand on the payroll
//! ranked for the focused door, with the full arithmetic behind each of them.
//! All the player's skill is spent here; after commit they only watch (GDD 2).

use super::chrome::{draw_panel, empty_notice, panel_style, title_style};
use super::{content_rect, UiAction, UiContext};
use crate::model::{Encounter, HeistTarget};
use crate::sim::plan::{candidates, Candidate, PlanDraft};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

const DOORS_WIDTH: f32 = 470.0;
const NODE_HEIGHT: f32 = 74.0;
const NODE_GAP: f32 = 22.0;

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let Some(draft) = ctx.draft else {
        let rect = content_rect();
        draw_surface_with_title(rect, Some("Planning"), &panel_style(), title_style());
        empty_notice(rect, "Pick a mark on the board to start a plan.");
        return;
    };
    let Some(target) = ctx.data.targets.get(&draft.target_id) else {
        return;
    };

    draw_route(ctx, draft, target, actions);
    draw_candidates(ctx, draft, target, actions);
}

fn doors_rect() -> Rect {
    let content = content_rect();
    Rect::new(content.x, content.y, DOORS_WIDTH, content.h)
}

fn candidates_rect() -> Rect {
    let content = content_rect();
    Rect::new(
        content.x + DOORS_WIDTH + 14.0,
        content.y,
        content.w - DOORS_WIDTH - 14.0,
        content.h,
    )
}

fn is_cased(ctx: &UiContext<'_>, target_id: &str) -> bool {
    ctx.session
        .board_entry(target_id)
        .map(|entry| entry.cased)
        .unwrap_or(false)
}

/// The route: doors in order, joined by a corridor. A first sketch of the
/// procedural floorplan M3 will draw properly.
fn draw_route(
    ctx: &UiContext<'_>,
    draft: &PlanDraft,
    target: &HeistTarget,
    actions: &mut Vec<UiAction>,
) {
    let content = draw_panel(doors_rect(), &format!("{} — the route", target.name));
    let cased = is_cased(ctx, &target.id);
    let mouse = ctx.mouse();

    for (index, door_id) in draft.doors.iter().enumerate() {
        let Some(encounter) = ctx.data.encounters.get(door_id) else {
            continue;
        };
        let y = content.y + index as f32 * (NODE_HEIGHT + NODE_GAP);
        let rect = Rect::new(content.x + 26.0, y, content.w - 26.0, NODE_HEIGHT);
        if rect.bottom() > content.bottom() {
            break;
        }

        if index > 0 {
            draw_line(
                content.x + 13.0,
                y - NODE_GAP,
                content.x + 13.0,
                y + NODE_HEIGHT * 0.5,
                2.0,
                Color::new(0.34, 0.42, 0.55, 0.8),
            );
        }
        draw_line(
            content.x + 13.0,
            y + NODE_HEIGHT * 0.5,
            rect.x,
            y + NODE_HEIGHT * 0.5,
            2.0,
            Color::new(0.34, 0.42, 0.55, 0.8),
        );
        draw_circle(
            content.x + 13.0,
            y + NODE_HEIGHT * 0.5,
            5.0,
            node_color(ctx, draft, index),
        );

        if draw_node(ctx, rect, draft, index, encounter, cased, mouse) {
            actions.push(UiAction::FocusDoor(index));
        }
    }

    draw_route_footer(ctx, content, draft, actions);
}

fn node_color(ctx: &UiContext<'_>, draft: &PlanDraft, index: usize) -> Color {
    let _ = ctx;
    if draft.assigned(index).is_some() {
        Color::new(0.46, 0.76, 0.52, 1.0)
    } else {
        Color::new(0.60, 0.62, 0.68, 1.0)
    }
}

fn draw_node(
    ctx: &UiContext<'_>,
    rect: Rect,
    draft: &PlanDraft,
    index: usize,
    encounter: &Encounter,
    cased: bool,
    mouse: Vec2,
) -> bool {
    let focused = draft.focus == index;
    let hovered = rect.contains_point(mouse);
    let fill = if focused {
        Color::new(0.15, 0.19, 0.26, 1.0)
    } else if hovered {
        Color::new(0.12, 0.14, 0.18, 1.0)
    } else {
        Color::new(0.09, 0.10, 0.13, 1.0)
    };
    draw_surface(
        rect,
        &SurfaceStyle::new(fill)
            .with_left_accent(4.0, node_color(ctx, draft, index))
            .with_border(
                1.0,
                if focused {
                    dark::ACCENT
                } else {
                    Color::new(0.45, 0.50, 0.60, 0.28)
                },
            ),
    );

    draw_ui_text_ex(
        &format!("{}. {}", index + 1, encounter.name),
        rect.x + 12.0,
        rect.y + 22.0,
        TextStyle::new(16.0, dark::TEXT_BRIGHT).params(),
    );
    draw_ui_text_ex(
        &format!(
            "{} · {}",
            encounter.primary_skill.label(),
            encounter.complexity.label()
        ),
        rect.x + 12.0,
        rect.y + 42.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );
    draw_text_right(
        &if cased {
            format!("DC {}", encounter.difficulty)
        } else {
            "DC ??".to_owned()
        },
        rect.right() - 12.0,
        rect.y + 22.0,
        TextStyle::new(
            15.0,
            if cased {
                dark::TEXT_BRIGHT
            } else {
                dark::TEXT_DIM
            },
        ),
    );

    match draft.assigned(index) {
        Some(member_id) => {
            let name = ctx
                .session
                .member(member_id)
                .map(|member| member.name.clone())
                .unwrap_or_else(|| member_id.to_owned());
            draw_ui_text_ex(
                &name,
                rect.x + 12.0,
                rect.y + 63.0,
                TextStyle::new(14.0, Color::new(0.56, 0.80, 0.60, 1.0)).params(),
            );
            if cased {
                if let Some(chance) = assigned_chance(ctx, draft, encounter, member_id) {
                    draw_text_right(
                        &format!("{:.0}%", chance * 100.0),
                        rect.right() - 12.0,
                        rect.y + 63.0,
                        TextStyle::new(14.0, chance_color(chance)),
                    );
                }
            }
        }
        None => {
            draw_ui_text_ex(
                "— nobody assigned —",
                rect.x + 12.0,
                rect.y + 63.0,
                TextStyle::new(14.0, Color::new(0.86, 0.62, 0.40, 1.0)).params(),
            );
        }
    }

    hovered && is_mouse_button_released(MouseButton::Left)
}

fn assigned_chance(
    ctx: &UiContext<'_>,
    draft: &PlanDraft,
    encounter: &Encounter,
    member_id: &str,
) -> Option<f32> {
    let target = ctx.data.targets.get(&draft.target_id)?;
    let member = ctx.session.member(member_id)?;
    Some(
        crate::sim::plan::candidate_check(
            ctx.session,
            ctx.data,
            target,
            encounter,
            member,
            &draft.crew_on_job(),
        )
        .success_chance(),
    )
}

fn draw_route_footer(
    ctx: &UiContext<'_>,
    content: Rect,
    draft: &PlanDraft,
    actions: &mut Vec<UiAction>,
) {
    let mouse = ctx.mouse();
    let y = content.bottom() - 40.0;
    let width = (content.w - 20.0) / 3.0;

    if button_rect_tone_at(
        Rect::new(content.x, y, width, 38.0),
        "Back",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::AbandonPlan);
    }
    if button_rect_tone_at(
        Rect::new(content.x + width + 10.0, y, width, 38.0),
        "Let them pick",
        true,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::AutoFillPlan);
    }
    let open = draft.unfilled();
    let label = if draft.is_complete() {
        "Commit".to_owned()
    } else if open == 1 {
        "1 door open".to_owned()
    } else {
        format!("{} doors open", open)
    };
    if button_rect_tone_at(
        Rect::new(content.x + (width + 10.0) * 2.0, y, width, 38.0),
        &label,
        draft.is_complete(),
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::CommitPlan);
    }
}

fn draw_candidates(
    ctx: &UiContext<'_>,
    draft: &PlanDraft,
    target: &HeistTarget,
    actions: &mut Vec<UiAction>,
) {
    let Some(door_id) = draft.focused_door() else {
        return;
    };
    let Some(encounter) = ctx.data.encounters.get(door_id) else {
        return;
    };

    let content = draw_panel(candidates_rect(), &format!("Who takes {}?", encounter.name));
    let cased = is_cased(ctx, &target.id);

    draw_text_block(
        &if cased {
            format!(
                "{} Difficulty class {}. {}",
                encounter.description, encounter.difficulty, encounter.failure_consequence
            )
        } else {
            format!(
                "{} Uncased — the file has no difficulty on it, and the crew goes in blind.",
                encounter.description
            )
        },
        content.x,
        content.y,
        content.w,
        44.0,
        15.0,
        3.0,
        dark::TEXT_DIM,
    );

    let ranked = candidates(ctx.session, ctx.data, target, encounter, draft);
    let list = Rect::new(
        content.x,
        content.y + 50.0,
        content.w,
        content.bottom() - content.y - 90.0,
    );
    let layout = GridLayout::new(list.x, list.y, list.w, 8.0, 1, 76.0);
    let mouse = ctx.mouse();

    for (index, candidate) in ranked.iter().enumerate() {
        let (x, y, w, h) = layout.get_item_rect(index, 0.0);
        let rect = Rect::new(x, y, w, h);
        if rect.bottom() > list.bottom() {
            break;
        }

        let assigned = draft.assigned(draft.focus) == Some(candidate.member_id.as_str());
        if draw_candidate(rect, candidate, assigned, cased, mouse) && !candidate.unfit {
            actions.push(UiAction::AssignDoor {
                door: draft.focus,
                member_id: candidate.member_id.clone(),
            });
        }
    }

    if button_rect_tone_at(
        Rect::new(
            content.right() - 180.0,
            content.bottom() - 38.0,
            180.0,
            36.0,
        ),
        "Leave this door open",
        draft.assigned(draft.focus).is_some(),
        ButtonTone::Danger,
        mouse,
    ) {
        actions.push(UiAction::ClearDoor(draft.focus));
    }
}

fn draw_candidate(
    rect: Rect,
    candidate: &Candidate,
    assigned: bool,
    cased: bool,
    mouse: Vec2,
) -> bool {
    let hovered = !candidate.unfit && rect.contains_point(mouse);
    let fill = if assigned {
        Color::new(0.14, 0.22, 0.17, 1.0)
    } else if hovered {
        Color::new(0.12, 0.14, 0.18, 1.0)
    } else {
        Color::new(0.085, 0.095, 0.12, 1.0)
    };
    let accent = if candidate.unfit {
        Color::new(0.55, 0.35, 0.32, 1.0)
    } else if cased {
        chance_color(candidate.success_chance())
    } else {
        Color::new(0.50, 0.56, 0.68, 1.0)
    };
    draw_surface(
        rect,
        &SurfaceStyle::new(fill)
            .with_left_accent(4.0, accent)
            .with_border(
                1.0,
                if assigned {
                    dark::ACCENT
                } else {
                    Color::new(0.45, 0.50, 0.60, 0.25)
                },
            ),
    );

    let name_color = if candidate.unfit {
        dark::TEXT_DIM
    } else {
        dark::TEXT_BRIGHT
    };
    draw_ui_text_ex(
        &candidate.member_name,
        rect.x + 12.0,
        rect.y + 22.0,
        TextStyle::new(17.0, name_color).params(),
    );

    let mut note = candidate.specialty.clone();
    if candidate.unfit {
        note.push_str(" · unfit for work");
    } else if candidate.doubled_up {
        note.push_str(" · already on another door");
    }
    draw_ui_text_ex(
        &note,
        rect.x + 12.0,
        rect.y + 42.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    // The whole point of the screen: every modifier, by name, before commit.
    draw_ui_text_ex(
        &modifier_line(candidate),
        rect.x + 12.0,
        rect.y + 63.0,
        TextStyle::new(13.0, Color::new(0.62, 0.68, 0.80, 1.0)).params(),
    );

    draw_text_right(
        &format!("{:+}", candidate.check.bonus()),
        rect.right() - 14.0,
        rect.y + 24.0,
        TextStyle::new(20.0, name_color),
    );
    if cased && !candidate.unfit {
        draw_text_right(
            &format!(
                "{:.0}% · needs {}+",
                candidate.success_chance() * 100.0,
                candidate.check.roll_needed()
            ),
            rect.right() - 14.0,
            rect.y + 48.0,
            TextStyle::new(13.0, chance_color(candidate.success_chance())),
        );
    }

    hovered && is_mouse_button_released(MouseButton::Left)
}

fn modifier_line(candidate: &Candidate) -> String {
    candidate
        .check
        .significant()
        .map(|entry| format!("{} {}", entry.label, entry.signed()))
        .collect::<Vec<_>>()
        .join("  ")
}

fn chance_color(chance: f32) -> Color {
    if chance >= 0.8 {
        Color::new(0.46, 0.78, 0.52, 1.0)
    } else if chance >= 0.55 {
        Color::new(0.85, 0.76, 0.40, 1.0)
    } else {
        Color::new(0.90, 0.46, 0.38, 1.0)
    }
}
