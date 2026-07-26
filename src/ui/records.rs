//! The records screen: what the outfit did, what it is owed, and what it cost.

use super::chrome::{difficulty_color, draw_panel, empty_notice, stat_row};
use super::{content_rect, UiAction, UiContext};
use crate::state::JobRecord;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::series::Series;
use macroquad_toolkit::ui::draw_ui_text_ex;

/// How many weeks of the two curves the chart holds.
const CHART_RESOLUTION: usize = 48;
const HISTORY_WIDTH: f32 = 560.0;

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let _ = actions;
    draw_history(ctx);
    draw_standing(ctx);
}

fn history_rect() -> Rect {
    let content = content_rect();
    Rect::new(content.x, content.y, HISTORY_WIDTH, content.h)
}

fn standing_rect() -> Rect {
    let content = content_rect();
    Rect::new(
        content.x + HISTORY_WIDTH + 14.0,
        content.y,
        content.w - HISTORY_WIDTH - 14.0,
        content.h,
    )
}

fn draw_history(ctx: &UiContext<'_>) {
    let content = draw_panel(history_rect(), "Job History");
    let history = &ctx.session.history;

    if history.is_empty() {
        empty_notice(content, "No jobs on the books yet.");
        return;
    }

    // Newest first: the last thing that happened is the thing being looked for.
    let layout = GridLayout::new(content.x, content.y, content.w, 6.0, 1, 52.0);
    for (index, record) in history.iter().rev().enumerate() {
        let (x, y, w, h) = layout.get_item_rect(index, 0.0);
        let rect = Rect::new(x, y, w, h);
        if rect.bottom() > content.bottom() - 16.0 {
            draw_ui_text_ex(
                &format!("+{} earlier jobs", history.len() - index),
                content.x,
                content.bottom() + 4.0,
                TextStyle::new(13.0, dark::TEXT_DIM).params(),
            );
            break;
        }
        draw_record(rect, record);
    }
}

fn draw_record(rect: Rect, record: &JobRecord) {
    let tone = if record.success {
        Color::new(0.46, 0.76, 0.52, 1.0)
    } else {
        Color::new(0.88, 0.42, 0.38, 1.0)
    };
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.085, 0.095, 0.12, 1.0))
            .with_left_accent(3.0, tone)
            .with_border(1.0, Color::new(0.42, 0.48, 0.58, 0.28)),
    );

    draw_ui_text_ex(
        &format!("Week {} · {}", record.week, record.target_name),
        rect.x + 12.0,
        rect.y + 21.0,
        TextStyle::new(15.0, dark::TEXT_BRIGHT).params(),
    );
    draw_ui_text_ex(
        &format!(
            "{} · {}/{} doors · {}",
            record.difficulty.label(),
            record.doors_passed,
            record.doors_total,
            if record.delegated {
                "delegated"
            } else {
                "planned"
            }
        ),
        rect.x + 12.0,
        rect.y + 40.0,
        TextStyle::new(13.0, difficulty_color(record.difficulty)).params(),
    );
    draw_text_right(
        &format_compact_money(record.payout),
        rect.right() - 12.0,
        rect.y + 21.0,
        TextStyle::new(15.0, tone),
    );
    draw_text_right(
        &format!("rep {} · not {}", record.reputation, record.notoriety),
        rect.right() - 12.0,
        rect.y + 40.0,
        TextStyle::new(12.0, dark::TEXT_DIM),
    );
}

fn draw_standing(ctx: &UiContext<'_>) {
    let content = draw_panel(standing_rect(), "The Standing");
    let session = ctx.session;
    let tally = &session.tally;

    let win_rate = if tally.jobs_run > 0 {
        tally.jobs_won as f32 / tally.jobs_run as f32
    } else {
        0.0
    };
    let rows: [(String, String); 8] = [
        ("Week".to_owned(), session.week.to_string()),
        (
            "Jobs run".to_owned(),
            format!("{} ({} clean)", tally.jobs_run, tally.clean_sweeps),
        ),
        (
            "Success rate".to_owned(),
            format!("{:.0}%", win_rate * 100.0),
        ),
        (
            "Taken, all told".to_owned(),
            format_money(tally.payout_total),
        ),
        (
            "Best single job".to_owned(),
            format_money(tally.payout_best),
        ),
        (
            "Naturals".to_owned(),
            format!(
                "{} up, {} down",
                tally.critical_successes, tally.critical_failures
            ),
        ),
        (
            "Injuries taken".to_owned(),
            tally.injuries_taken.to_string(),
        ),
        ("Quiet weeks".to_owned(), tally.quiet_weeks.to_string()),
    ];

    for (index, (label, value)) in rows.iter().enumerate() {
        stat_row(
            Rect::new(content.x, content.y + index as f32 * 23.0, content.w, 20.0),
            label,
            value,
            15.0,
            dark::TEXT,
        );
    }

    draw_curves(
        Rect::new(content.x, content.y + 210.0, content.w, 118.0),
        ctx,
    );
    draw_awards(
        Rect::new(
            content.x,
            content.y + 344.0,
            content.w,
            content.bottom() - content.y - 352.0,
        ),
        ctx,
    );

    draw_ui_text_ex(
        &format!("Seed {}", session.seed),
        content.x,
        content.bottom() + 4.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );
}

/// Reputation against notoriety, week by week. The two curves pulling apart is
/// the campaign's whole shape (GDD 5.6).
fn draw_curves(rect: Rect, ctx: &UiContext<'_>) {
    draw_ui_text_ex(
        "Reputation and Notoriety",
        rect.x,
        rect.y - 6.0,
        TextStyle::new(15.0, dark::TEXT_BRIGHT).params(),
    );

    let plot = Rect::new(rect.x, rect.y + 6.0, rect.w, rect.h - 6.0);
    draw_surface(
        plot,
        &SurfaceStyle::new(Color::new(0.07, 0.08, 0.10, 1.0))
            .with_border(1.0, Color::new(0.34, 0.40, 0.52, 0.4)),
    );

    let mut reputation = Series::new(CHART_RESOLUTION);
    let mut notoriety = Series::new(CHART_RESOLUTION);
    for record in &ctx.session.history {
        reputation.push(record.reputation as f32);
        notoriety.push(record.notoriety as f32);
    }

    if reputation.is_empty() {
        empty_notice(plot, "Run a job and the curves start.");
        return;
    }

    let ceiling = reputation
        .max()
        .unwrap_or(1.0)
        .max(notoriety.max().unwrap_or(1.0))
        .max(1.0);

    plot_series(
        plot,
        &reputation,
        ceiling,
        Color::new(0.42, 0.70, 0.92, 1.0),
    );
    plot_series(plot, &notoriety, ceiling, Color::new(0.92, 0.52, 0.36, 1.0));

    draw_ui_text_ex(
        &format!("{:.0}", ceiling),
        plot.x + 6.0,
        plot.y + 14.0,
        TextStyle::new(12.0, dark::TEXT_DIM).params(),
    );
    draw_text_right(
        "reputation / notoriety",
        plot.right() - 6.0,
        plot.bottom() - 6.0,
        TextStyle::new(12.0, dark::TEXT_DIM),
    );
}

fn plot_series(plot: Rect, series: &Series, ceiling: f32, color: Color) {
    let buckets = series.buckets();
    if buckets.len() < 2 {
        return;
    }

    let step = plot.w / (buckets.len() - 1) as f32;
    for index in 1..buckets.len() {
        let previous = buckets[index - 1].last;
        let current = buckets[index].last;
        draw_line(
            plot.x + (index - 1) as f32 * step,
            plot.bottom() - (previous / ceiling) * (plot.h - 8.0) - 4.0,
            plot.x + index as f32 * step,
            plot.bottom() - (current / ceiling) * (plot.h - 8.0) - 4.0,
            2.0,
            color,
        );
    }
}

fn draw_awards(rect: Rect, ctx: &UiContext<'_>) {
    let (unlocked, total) = ctx.session.achievements.progress();
    draw_ui_text_ex(
        "Achievements",
        rect.x,
        rect.y,
        TextStyle::new(15.0, dark::TEXT_BRIGHT).params(),
    );
    draw_text_right(
        &format!("{}/{}", unlocked, total),
        rect.right(),
        rect.y,
        TextStyle::new(15.0, dark::TEXT_DIM),
    );
    meter(
        Rect::new(rect.x, rect.y + 8.0, rect.w, 14.0),
        unlocked as f32,
        total.max(1) as f32,
        Color::new(0.46, 0.72, 0.52, 1.0),
        None,
    );

    // The most recent unlocks, then whatever is closest to happening next.
    let mut unlocked_names: Vec<&str> = ctx
        .session
        .achievements
        .iter()
        .filter(|a| a.unlocked)
        .map(|a| a.name.as_str())
        .collect();
    unlocked_names.reverse();

    let rows = (((rect.h - 60.0) / 16.0) as usize).max(1);
    for (index, name) in unlocked_names.iter().enumerate() {
        if index >= rows * 2 {
            break;
        }
        let column = index / rows;
        draw_ui_text_ex(
            name,
            rect.x + column as f32 * (rect.w * 0.5),
            rect.y + 40.0 + (index % rows) as f32 * 16.0,
            TextStyle::new(13.0, Color::new(0.56, 0.82, 0.60, 1.0)).params(),
        );
    }

    if let Some(next) = closest_locked(ctx) {
        draw_ui_text_ex(
            &format!("Next: {} — {}", next.0, next.1),
            rect.x,
            rect.bottom() - 8.0,
            TextStyle::new(12.0, dark::TEXT_DIM).params(),
        );
    }
}

/// The locked achievement the campaign is furthest along towards.
fn closest_locked(ctx: &UiContext<'_>) -> Option<(String, String)> {
    ctx.data
        .awards
        .iter()
        .filter(|award| !ctx.session.achievements.is_unlocked(&award.id))
        .max_by(|a, b| {
            let progress =
                |award: &crate::sim::AwardDef| award.progress(&ctx.session.tally, ctx.session);
            progress(a)
                .partial_cmp(&progress(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|award| (award.name.clone(), award.description.clone()))
}
