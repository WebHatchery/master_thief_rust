//! The records screen: what the outfit did, what it is owed, and what it cost.

use super::chrome::{difficulty_color, draw_panel, empty_notice, stat_row};
use super::iconography::{draw_icon, Icon};
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
    let rows: [(String, String); 10] = [
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
            "Injuries · quiet weeks".to_owned(),
            format!("{} · {}", tally.injuries_taken, tally.quiet_weeks),
        ),
        // The two decisions the week turns on that nothing else here counts:
        // when the fixer lost their nerve, and when they spent somebody who
        // should have been resting (GDD 5.2, 5.6).
        (
            "Called off · worked spent".to_owned(),
            format!(
                "{} ({} doors left) · {}",
                tally.jobs_called_off, tally.doors_left_standing, tally.doors_worked_spent
            ),
        ),
        // The other half of the ledger: the outfit is not only what it takes.
        (
            "Paid out in wages".to_owned(),
            format!(
                "{}{}",
                format_money(tally.wages_paid),
                if tally.weeks_missed_payroll > 0 {
                    format!(" ({} short)", tally.weeks_missed_payroll)
                } else {
                    String::new()
                }
            ),
        ),
        (
            "Lost to the city".to_owned(),
            format!(
                "{} · {} taken, {} left",
                format_money(tally.cash_seized + tally.bribes_paid + tally.bails_paid),
                tally.arrests,
                tally.walkouts
            ),
        ),
    ];

    for (index, (label, value)) in rows.iter().enumerate() {
        stat_row(
            // Ten rows now, not nine: a tighter pitch keeps the block clear of
            // the chart below without stealing room from the awards list.
            Rect::new(content.x, content.y + index as f32 * 20.0, content.w, 19.0),
            label,
            value,
            15.0,
            dark::TEXT,
        );
    }

    draw_curves(
        Rect::new(content.x, content.y + 212.0, content.w, 96.0),
        ctx,
    );
    draw_awards(
        Rect::new(
            content.x,
            content.y + 320.0,
            content.w,
            content.bottom() - content.y - 328.0,
        ),
        ctx,
    );

    if let Some(done) = &session.retired {
        draw_ui_text_ex(
            &done.headline(),
            content.x,
            content.bottom() - 12.0,
            TextStyle::new(15.0, Color::new(0.56, 0.82, 0.58, 1.0)).params(),
        );
    }

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

    draw_badge_grid(Rect::new(rect.x, rect.y + 28.0, rect.w, 25.0), ctx);
    draw_campaign_stamps(Rect::new(rect.x, rect.y + 54.0, rect.w, 20.0), ctx);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AchievementBadgeState {
    Locked,
    Earned,
    Notable,
}

/// The registry is deliberately split into four stable lookup batches: three
/// complete shelves of 21 and a final shelf containing the remaining IDs.
pub fn achievement_batch(index: usize) -> usize {
    (index / 21).min(3)
}

fn draw_badge_grid(rect: Rect, ctx: &UiContext<'_>) {
    let size = 24.0;
    let gap = 12.0;
    let mut slot = 0;
    for batch in 0..4 {
        for (index, award) in ctx
            .data
            .awards
            .iter()
            .enumerate()
            .filter(|(index, _)| achievement_batch(*index) == batch)
            .take(4)
        {
            let x = rect.x + slot as f32 * (size + gap);
            if x + size > rect.right() {
                break;
            }
            let state = if !ctx.session.achievements.is_unlocked(&award.id) {
                AchievementBadgeState::Locked
            } else if index % 21 == 0 {
                AchievementBadgeState::Notable
            } else {
                AchievementBadgeState::Earned
            };
            draw_achievement_badge(Rect::new(x, rect.y, size, size), state, batch);
            slot += 1;
        }
    }
}

/// The same renderer is used by records at 96, 48, and 24 logical pixels.
pub fn draw_achievement_badge(rect: Rect, state: AchievementBadgeState, batch: usize) {
    let size = rect.w.min(rect.h);
    let center = vec2(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5);
    let radius = size * 0.40;
    let locked = Color::new(0.36, 0.40, 0.48, 0.70);
    let earned = Color::new(0.45, 0.76, 0.63, 0.98);
    let brass = Color::new(0.78, 0.56, 0.22, 0.98);
    let tone = match state {
        AchievementBadgeState::Locked => locked,
        AchievementBadgeState::Earned => earned,
        AchievementBadgeState::Notable => brass,
    };

    if matches!(state, AchievementBadgeState::Notable) {
        draw_circle_lines(center.x, center.y, radius + size * 0.10, 1.5, brass);
    }
    draw_circle_lines(center.x, center.y, radius, (size * 0.055).max(1.0), tone);

    if matches!(state, AchievementBadgeState::Locked) {
        for offset in [-0.24, 0.0, 0.24] {
            draw_line(
                center.x - radius * 0.65 + size * offset,
                center.y + radius * 0.65,
                center.x + radius * 0.65 + size * offset,
                center.y - radius * 0.65,
                (size * 0.045).max(1.0),
                locked,
            );
        }
    } else {
        let points = match batch {
            0 => [(0.0, -0.32), (-0.30, 0.25), (0.30, 0.25)],
            1 => [(-0.28, -0.18), (0.28, -0.18), (0.0, 0.34)],
            2 => [(-0.30, 0.24), (0.0, -0.34), (0.30, 0.24)],
            _ => [(-0.30, 0.0), (0.0, -0.30), (0.30, 0.0)],
        };
        for (x, y) in points {
            draw_circle(
                center.x + x * size,
                center.y + y * size,
                (size * 0.07).max(1.0),
                tone,
            );
        }
        draw_line(
            center.x - radius * 0.42,
            center.y,
            center.x - radius * 0.08,
            center.y + radius * 0.34,
            (size * 0.065).max(1.0),
            tone,
        );
        draw_line(
            center.x - radius * 0.08,
            center.y + radius * 0.34,
            center.x + radius * 0.48,
            center.y - radius * 0.36,
            (size * 0.065).max(1.0),
            tone,
        );
    }
}

#[derive(Debug, Clone, Copy)]
struct CampaignStamp {
    label: &'static str,
    icon: Icon,
    tone: Color,
}

fn draw_campaign_stamps(rect: Rect, ctx: &UiContext<'_>) {
    let tally = &ctx.session.tally;
    let mut stamps = Vec::new();
    if tally.jobs_run > 0 {
        stamps.push(CampaignStamp {
            label: "First job",
            icon: Icon::Door,
            tone: Color::new(0.46, 0.72, 0.92, 1.0),
        });
    }
    if tally.clean_sweeps > 0 {
        stamps.push(CampaignStamp {
            label: "Clean",
            icon: Icon::Success,
            tone: Color::new(0.45, 0.76, 0.63, 1.0),
        });
    }
    if tally.jobs_lost > 0 {
        stamps.push(CampaignStamp {
            label: "Botched",
            icon: Icon::Warning,
            tone: Color::new(0.88, 0.61, 0.25, 1.0),
        });
    }
    if ctx.session.retired.is_some() {
        stamps.push(CampaignStamp {
            label: "Retired",
            icon: Icon::Retire,
            tone: Color::new(0.78, 0.56, 0.22, 1.0),
        });
    }
    if tally.heat_peak > 0 {
        stamps.push(CampaignStamp {
            label: "Heat peak",
            icon: Icon::Heat,
            tone: Color::new(0.88, 0.34, 0.30, 1.0),
        });
    }
    if tally.jobs_run >= 10 {
        stamps.push(CampaignStamp {
            label: "Long run",
            icon: Icon::Payroll,
            tone: Color::new(0.55, 0.67, 0.88, 1.0),
        });
    }

    if stamps.is_empty() {
        return;
    }
    let width = rect.w / stamps.len() as f32;
    for (index, stamp) in stamps.iter().enumerate() {
        let x = rect.x + index as f32 * width;
        draw_circle_lines(x + 11.0, rect.y + 13.0, 11.0, 1.2, stamp.tone);
        draw_icon(
            stamp.icon,
            Rect::new(x, rect.y + 2.0, 22.0, 22.0),
            stamp.tone,
        );
        draw_ui_text_ex(
            stamp.label,
            x + 26.0,
            rect.y + 17.0,
            TextStyle::new(11.0, stamp.tone).params(),
        );
    }
}

#[cfg(test)]
mod tests;
