//! Header, tabs, footer, and the small shared pieces every screen borrows.

use super::{Screen, UiAction, UiContext, LOGICAL_WIDTH};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn panel_style() -> SurfaceStyle {
    SurfaceStyle::new(Color::new(0.07, 0.075, 0.095, 0.97))
        .with_border(1.0, Color::new(0.34, 0.40, 0.52, 0.6))
        .with_header(40.0, Color::new(0.10, 0.115, 0.145, 1.0))
        .with_header_divider(1.0, Color::new(0.34, 0.40, 0.52, 0.4))
}

pub fn title_style() -> TextStyle<'static> {
    TextStyle::new(18.0, dark::TEXT)
}

pub fn draw_panel(rect: Rect, title: &str) -> Rect {
    draw_surface_with_title(rect, Some(title), &panel_style(), title_style());
    Rect::new(rect.x + 16.0, rect.y + 52.0, rect.w - 32.0, rect.h - 68.0)
}

pub fn draw_header(ctx: &UiContext<'_>) {
    let rect = Rect::new(18.0, 16.0, LOGICAL_WIDTH - 36.0, 64.0);
    let style = SurfaceStyle::new(Color::new(0.08, 0.09, 0.12, 0.96))
        .with_border(1.0, dark::ACCENT)
        .with_top_highlight(2.0, Color::new(0.55, 0.72, 0.95, 0.75));
    draw_surface(rect, &style);

    draw_texture_ex(
        &ctx.artwork.wordmark,
        rect.x + 16.0,
        rect.y + 13.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(190.0, 48.0)),
            ..Default::default()
        },
    );

    let session = ctx.session;
    let badges: [(String, Color); 6] = [
        (
            format!("Week {}", session.week),
            Color::new(0.18, 0.24, 0.32, 1.0),
        ),
        (
            format_money(session.budget),
            Color::new(0.16, 0.28, 0.20, 1.0),
        ),
        (payroll_label(ctx), payroll_color(ctx)),
        (
            format!("Rep {}", session.reputation),
            Color::new(0.18, 0.26, 0.34, 1.0),
        ),
        (
            format!("Notoriety {}", session.notoriety),
            Color::new(0.30, 0.22, 0.18, 1.0),
        ),
        (heat_label(ctx), heat_color(ctx)),
    ];

    let mut x = rect.right() - 18.0;
    for (label, fill) in badges.iter().rev() {
        let width = badge_width(label);
        x -= width;
        draw_badge(
            Rect::new(x, rect.y + 18.0, width, 28.0),
            label,
            *fill,
            dark::TEXT,
        );
        x -= 8.0;
    }
}

fn badge_width(label: &str) -> f32 {
    (label.len() as f32 * 8.4 + 20.0).max(72.0)
}

/// The standing weekly bill, and how many quiet weeks it still buys. This is
/// the clock the week loop runs on, so it sits beside the money on every screen.
fn payroll_label(ctx: &UiContext<'_>) -> String {
    let due = crate::sim::weekly_outgoings(ctx.session, &ctx.data.config.payroll);
    let weeks = crate::sim::weeks_of_runway(ctx.session, &ctx.data.config.payroll);
    format!(
        "{}/wk · {}wk left",
        format_compact_money(due),
        weeks.min(99)
    )
}

fn payroll_color(ctx: &UiContext<'_>) -> Color {
    match crate::sim::weeks_of_runway(ctx.session, &ctx.data.config.payroll) {
        0..=1 => Color::new(0.44, 0.18, 0.18, 1.0),
        2..=4 => Color::new(0.38, 0.30, 0.16, 1.0),
        _ => Color::new(0.18, 0.24, 0.24, 1.0),
    }
}

fn heat_label(ctx: &UiContext<'_>) -> String {
    let penalty = ctx.session.heat_dc_penalty(&ctx.data.config);
    if penalty > 0 {
        format!("Heat {} (+{} DC)", ctx.session.heat, penalty)
    } else {
        format!("Heat {}", ctx.session.heat)
    }
}

fn heat_color(ctx: &UiContext<'_>) -> Color {
    if ctx.session.heat_dc_penalty(&ctx.data.config) > 0 {
        Color::new(0.42, 0.18, 0.18, 1.0)
    } else {
        Color::new(0.22, 0.20, 0.28, 1.0)
    }
}

pub fn draw_tabs(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let labels: Vec<&str> = Screen::TABS.iter().map(|screen| screen.label()).collect();
    // Planning is not a tab; while it is open no tab reads as active.
    let active = Screen::TABS
        .iter()
        .position(|screen| *screen == ctx.screen)
        .unwrap_or(usize::MAX);

    let clicked = tab_bar_styled_at(
        Rect::new(18.0, 90.0, 500.0, 38.0),
        &labels,
        active,
        TabOrientation::Horizontal,
        &TabStyle::default(),
        ctx.mouse(),
    );

    if let Some(index) = clicked {
        actions.push(UiAction::ShowScreen(Screen::TABS[index]));
    }
}

pub fn draw_footer(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let rect = Rect::new(18.0, 650.0, LOGICAL_WIDTH - 36.0, 54.0);
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.055, 0.06, 0.075, 0.96))
            .with_border(1.0, Color::new(0.34, 0.40, 0.52, 0.45)),
    );

    let mouse = ctx.mouse();
    let mut x = rect.x + 14.0;
    let buttons: [(&str, bool, ButtonTone, UiAction); 6] = [
        (
            "Advance Week",
            true,
            ButtonTone::Primary,
            UiAction::AdvanceWeek,
        ),
        ("Save", true, ButtonTone::Positive, UiAction::Save),
        ("Load", ctx.save_exists, ButtonTone::Primary, UiAction::Load),
        (
            "New Campaign",
            true,
            ButtonTone::Secondary,
            UiAction::NewGame,
        ),
        (
            "Delete Save",
            ctx.save_exists,
            ButtonTone::Danger,
            UiAction::DeleteSave,
        ),
        (
            "Settings",
            true,
            ButtonTone::Secondary,
            UiAction::OpenSettings,
        ),
    ];

    for (label, enabled, tone, action) in buttons {
        let width = 132.0;
        if button_rect_tone_at(
            Rect::new(x, rect.y + 9.0, width, 36.0),
            label,
            enabled,
            tone,
            mouse,
        ) {
            actions.push(action);
        }
        x += width + 10.0;
    }

    let _ = x;
}

/// The narrowest a modifier column can usefully be before the label and its
/// value collide.
const MIN_MODIFIER_COLUMN: f32 = 118.0;

/// Every modifier on a check, named, laid out in as many columns as the space
/// allows and never silently dropped.
///
/// Pillar 2 says the player sees "every modifier by name". That was easy when a
/// door carried four of them and they fitted on one line. A door now routinely
/// carries eight to fourteen — skill, focus, kit, proficiency, fatigue,
/// loyalty, injuries, environment, city heat, a ripened mark, worn kit, a tail,
/// chemistry — and the single joined line every screen used overflowed its
/// panel, while the run's tally quietly stopped after the fifth and left the
/// running total disagreeing with the arithmetic on screen.
///
/// `limit` caps how many are revealed, for the run's one-at-a-time tally; pass
/// `usize::MAX` to show them all. Returns how many were drawn, so a caller can
/// tell whether the space was really enough.
pub fn draw_modifier_grid<'a>(
    rect: Rect,
    entries: impl Iterator<Item = &'a crate::rules::outcome::ModifierEntry>,
    limit: usize,
    size: f32,
) -> usize {
    let line_height = size + 5.0;
    let rows = ((rect.h / line_height).floor() as usize).max(1);
    let shown: Vec<&crate::rules::outcome::ModifierEntry> = entries.take(limit).collect();
    if shown.is_empty() {
        return 0;
    }

    // Grow columns rather than clip: a modifier the player cannot see is a
    // modifier the game is hiding from them. Where even that will not fit,
    // the last cell says how many are missing — silently dropping them is what
    // broke this in the first place.
    let max_columns = ((rect.w / MIN_MODIFIER_COLUMN).floor() as usize).max(1);
    let columns = shown.len().div_ceil(rows).clamp(1, max_columns);
    let capacity = rows * columns;
    let overflowed = shown.len() > capacity;
    let visible = if overflowed {
        capacity - 1
    } else {
        shown.len()
    };
    let column_w = rect.w / columns as f32;

    if overflowed {
        let index = capacity - 1;
        draw_ui_text_ex(
            &format!("+{} more", shown.len() - visible),
            rect.x + (index / rows) as f32 * column_w,
            rect.y + (index % rows + 1) as f32 * line_height,
            TextStyle::new(size, dark::TEXT_DIM).params(),
        );
    }

    for (index, entry) in shown.iter().take(visible).enumerate() {
        let column = index / rows;
        let row = index % rows;
        let x = rect.x + column as f32 * column_w;
        let y = rect.y + (row + 1) as f32 * line_height;

        let chip = Rect::new(x, y - line_height + 2.0, column_w - 8.0, line_height - 2.0);
        let chip_tone = if entry.value > 0 {
            Color::new(0.16, 0.28, 0.27, 0.72)
        } else if entry.value < 0 {
            Color::new(0.28, 0.17, 0.18, 0.72)
        } else {
            Color::new(0.13, 0.16, 0.21, 0.72)
        };
        draw_rectangle(chip.x, chip.y, chip.w, chip.h, chip_tone);
        draw_rectangle_lines(
            chip.x,
            chip.y,
            chip.w,
            chip.h,
            1.0,
            Color::new(0.42, 0.48, 0.58, 0.35),
        );

        draw_ui_text_ex(
            &entry.label,
            x,
            y,
            TextStyle::new(size, dark::TEXT_DIM).params(),
        );
        draw_text_right(
            &entry.signed(),
            x + column_w - 10.0,
            y,
            TextStyle::new(
                size,
                if entry.value >= 0 {
                    Color::new(0.56, 0.80, 0.60, 1.0)
                } else {
                    Color::new(0.88, 0.52, 0.44, 1.0)
                },
            ),
        );
    }
    visible
}

/// A left-aligned label with a right-aligned value on one line.
pub fn stat_row(rect: Rect, label: &str, value: &str, size: f32, color: Color) {
    draw_ui_text_ex(
        label,
        rect.x,
        rect.y + rect.h * 0.72,
        TextStyle::new(size, dark::TEXT_DIM).params(),
    );
    draw_text_right(
        value,
        rect.right(),
        rect.y + rect.h * 0.72,
        TextStyle::new(size, color),
    );
}

/// A selectable row in one of the left-hand lists.
pub fn list_card(rect: Rect, selected: bool, accent: Color, mouse: Vec2) -> bool {
    let hovered = rect.contains_point(mouse);
    let fill = if selected {
        Color::new(0.15, 0.19, 0.26, 1.0)
    } else if hovered {
        Color::new(0.12, 0.14, 0.18, 1.0)
    } else {
        Color::new(0.09, 0.10, 0.13, 1.0)
    };
    let style = SurfaceStyle::new(fill)
        .with_left_accent(4.0, accent)
        .with_border(
            1.0,
            if selected {
                dark::ACCENT
            } else {
                Color::new(0.45, 0.50, 0.60, 0.28)
            },
        );
    draw_surface(rect, &style);

    hovered && is_mouse_button_released(MouseButton::Left)
}

pub fn rarity_color(rarity: crate::model::Rarity) -> Color {
    use crate::model::Rarity;
    match rarity {
        Rarity::Common => Color::new(0.62, 0.65, 0.70, 1.0),
        Rarity::Uncommon => Color::new(0.42, 0.74, 0.48, 1.0),
        Rarity::Rare => Color::new(0.40, 0.62, 0.92, 1.0),
        Rarity::Epic => Color::new(0.70, 0.48, 0.92, 1.0),
        Rarity::Legendary => Color::new(0.95, 0.72, 0.30, 1.0),
    }
}

pub fn difficulty_color(band: crate::model::DifficultyBand) -> Color {
    use crate::model::DifficultyBand;
    match band {
        DifficultyBand::Easy => Color::new(0.42, 0.74, 0.48, 1.0),
        DifficultyBand::Medium => Color::new(0.85, 0.76, 0.36, 1.0),
        DifficultyBand::Hard => Color::new(0.92, 0.52, 0.30, 1.0),
        DifficultyBand::Extreme => Color::new(0.88, 0.32, 0.32, 1.0),
    }
}

pub fn empty_notice(rect: Rect, text: &str) {
    draw_text_centered_in_box_ex(
        text,
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        TextStyle::new(17.0, dark::TEXT_DIM),
    );
}
