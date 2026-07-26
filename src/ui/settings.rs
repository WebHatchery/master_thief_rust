//! The settings panel, reachable from anywhere and covering the screen.

use super::{UiAction, UiContext, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use crate::prefs::RunPacing;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

pub fn panel_rect() -> Rect {
    Rect::new(
        LOGICAL_WIDTH * 0.5 - 260.0,
        LOGICAL_HEIGHT * 0.5 - 190.0,
        520.0,
        380.0,
    )
}

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    full_screen_overlay(0.72);

    let rect = panel_rect();
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.08, 0.09, 0.12, 1.0))
            .with_border(1.0, dark::ACCENT)
            .with_header(44.0, Color::new(0.11, 0.13, 0.17, 1.0))
            .with_header_divider(1.0, Color::new(0.34, 0.40, 0.52, 0.5)),
    );
    draw_text_centered_in_box_ex(
        "Settings",
        rect.x,
        rect.y,
        rect.w,
        44.0,
        TextStyle::new(20.0, dark::TEXT_BRIGHT),
    );

    let mouse = ctx.mouse();
    let prefs = ctx.prefs;
    let content = Rect::new(rect.x + 28.0, rect.y + 66.0, rect.w - 56.0, rect.h - 110.0);

    // How much of the run the player sits through — a pacing choice and an
    // accessibility one at the same time (GDD 9).
    row(content, 0, "The run", prefs.pacing.label());
    if button_rect_tone_at(
        control(content, 0),
        "Change",
        true,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::SetPacing(prefs.pacing.next()));
    }

    row(content, 1, "Sound", if prefs.sound { "On" } else { "Off" });
    if button_rect_tone_at(
        control(content, 1),
        if prefs.sound { "Mute" } else { "Unmute" },
        true,
        if prefs.sound {
            ButtonTone::Danger
        } else {
            ButtonTone::Positive
        },
        mouse,
    ) {
        actions.push(UiAction::SetSound(!prefs.sound));
    }

    row(
        content,
        2,
        "Volume",
        &format!("{:.0}%", prefs.volume * 100.0),
    );
    let volume_row = control(content, 2);
    let half = (volume_row.w - 8.0) * 0.5;
    if button_rect_tone_at(
        Rect::new(volume_row.x, volume_row.y, half, volume_row.h),
        "Quieter",
        prefs.volume > 0.0,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::SetVolume(prefs.volume - 0.1));
    }
    if button_rect_tone_at(
        Rect::new(volume_row.x + half + 8.0, volume_row.y, half, volume_row.h),
        "Louder",
        prefs.volume < 1.0,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::SetVolume(prefs.volume + 0.1));
    }

    row(
        content,
        3,
        "Hints",
        if prefs.hints { "Shown" } else { "Hidden" },
    );
    if button_rect_tone_at(
        control(content, 3),
        if prefs.hints { "Hide" } else { "Show" },
        true,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::ShowHints(!prefs.hints));
    }

    draw_text_block(
        pacing_note(prefs.pacing),
        content.x,
        content.y + 200.0,
        content.w,
        56.0,
        14.0,
        4.0,
        dark::TEXT_DIM,
    );

    if button_rect_tone_at(
        Rect::new(rect.x + 28.0, rect.bottom() - 52.0, rect.w - 56.0, 38.0),
        "Back to the job",
        true,
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::CloseSettings);
    }
}

fn pacing_note(pacing: RunPacing) -> &'static str {
    match pacing {
        RunPacing::Full => {
            "Every beat of every door: the die in the air, the modifiers stacking, the verdict."
        }
        RunPacing::Brisk => "The same beats at a little over twice the speed.",
        RunPacing::Instant => {
            "Committing goes straight to the results. Nothing about the outcome changes — the dice were already cast."
        }
    }
}

fn row(content: Rect, index: usize, label: &str, value: &str) {
    let y = content.y + index as f32 * 46.0;
    draw_ui_text_ex(
        label,
        content.x,
        y + 20.0,
        TextStyle::new(17.0, dark::TEXT).params(),
    );
    draw_ui_text_ex(
        value,
        content.x + 120.0,
        y + 20.0,
        TextStyle::new(17.0, dark::TEXT_BRIGHT).params(),
    );
}

fn control(content: Rect, index: usize) -> Rect {
    Rect::new(
        content.right() - 190.0,
        content.y + index as f32 * 46.0 + 2.0,
        190.0,
        26.0,
    )
}
