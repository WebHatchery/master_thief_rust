//! The run: the floorplan lighting up door by door, and the dice that do it.

use super::chrome::{draw_panel, empty_notice, panel_style, title_style};
use super::floorplan::{self, FloorplanPalette, RoomState};
use super::{content_rect, UiAction, UiContext};
use crate::game::playback::{DoorPhase, RunPlayback};
use crate::rules::Outcome;
use macroquad::prelude::*;
use macroquad_toolkit::paint::ScreenPainter;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

const DICE_WIDTH: f32 = 430.0;

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let Some(playback) = ctx.playback else {
        let rect = content_rect();
        draw_surface_with_title(rect, Some("The Run"), &panel_style(), title_style());
        empty_notice(rect, "No job is running.");
        return;
    };

    draw_building(ctx, playback);
    draw_run_effects(ctx, playback);
    draw_dice(ctx, playback, actions);
}

fn building_rect() -> Rect {
    let content = content_rect();
    Rect::new(
        content.x,
        content.y,
        content.w - DICE_WIDTH - 14.0,
        content.h,
    )
}

fn dice_rect() -> Rect {
    let content = content_rect();
    Rect::new(
        content.right() - DICE_WIDTH,
        content.y,
        DICE_WIDTH,
        content.h,
    )
}

/// Where the plan puts a given room, in logical UI space. `Game` uses this to
/// hang the critical-hit punctuation over the right door.
pub fn room_center(target_id: &str, index: usize, count: usize) -> Option<Vec2> {
    let content = draw_area(building_rect());
    let plan = floorplan::layout(content, count, floorplan::seed_for(target_id));
    plan.room(index)
        .map(|room| vec2(room.x + room.w * 0.5, room.y + room.h * 0.5))
}

fn draw_area(panel: Rect) -> Rect {
    Rect::new(
        panel.x + 16.0,
        panel.y + 52.0,
        panel.w - 32.0,
        panel.h - 92.0,
    )
}

fn draw_building(ctx: &UiContext<'_>, playback: &RunPlayback) {
    let report = playback.report();
    let content = draw_panel(building_rect(), &report.target_name);
    let area = draw_area(building_rect());
    let plan = floorplan::layout(
        area,
        report.doors.len(),
        floorplan::seed_for(&report.target_id),
    );

    let outcomes = playback.outcomes_so_far();
    let states: Vec<RoomState> = outcomes
        .iter()
        .enumerate()
        .map(|(index, outcome)| RoomState {
            assigned: true,
            active: index == playback.door_index() && outcome.is_none(),
            outcome: *outcome,
        })
        .collect();

    floorplan::paint(
        &mut ScreenPainter,
        &plan,
        &states,
        &FloorplanPalette::default(),
    );
    floorplan::draw_route_ink(&plan, Color::new(0.24, 0.72, 0.78, 0.72));

    for (index, room) in plan.rooms.iter().enumerate() {
        let Some(door) = report.doors.get(index) else {
            continue;
        };
        floorplan::draw_node_state(
            *room,
            run_node_state(
                outcomes.get(index).copied().flatten(),
                index == playback.door_index() && !playback.finished(),
            ),
        );
        floorplan::draw_skill_glyph(
            door.result.check.skill,
            vec2(room.right() - 48.0, room.y + 18.0),
            9.0,
        );
        floorplan::draw_door_silhouette(*room, &door.encounter_name);
        draw_ui_text_ex(
            &format!("{}. {}", index + 1, door.encounter_name),
            room.x + 10.0,
            room.y + 22.0,
            TextStyle::new(14.0, dark::TEXT_BRIGHT).params(),
        );
        draw_ui_text_ex(
            &door.result.check.member_name,
            room.x + 10.0,
            room.y + 40.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
        if !door.result.check.member_id.is_empty() {
            let portrait_state = run_portrait_state(ctx, &door.result.check.member_id, door);
            ctx.artwork.draw_portrait_state(
                &door.result.check.member_id,
                Rect::new(room.right() - 70.0, room.bottom() - 30.0, 24.0, 24.0),
                true,
                portrait_state,
            );
            draw_circle_lines(
                room.right() - 58.0,
                room.bottom() - 18.0,
                12.0,
                1.5,
                Color::new(0.45, 0.76, 0.63, 0.95),
            );
        }
        if let Some(outcome) = outcomes[index] {
            draw_ui_text_ex(
                outcome.label(),
                room.x + 10.0,
                room.bottom() - 12.0,
                TextStyle::new(13.0, outcome_color(outcome)).params(),
            );
        }
    }

    let cleared = outcomes.iter().filter(|o| o.is_some()).count();
    draw_ui_text_ex(
        &format!("Door {} of {}", cleared.max(1), report.doors.len()),
        content.x,
        content.bottom() + 6.0,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );
}

fn run_portrait_state(
    ctx: &UiContext<'_>,
    member_id: &str,
    door: &crate::sim::DoorOutcome,
) -> crate::artwork::PortraitState {
    if door.injury.is_some() {
        return crate::artwork::PortraitState::Injured;
    }
    if ctx.session.member(member_id).is_some_and(|member| {
        member.condition.fatigue >= ctx.data.config.condition.fatigue_work_threshold
    }) {
        return crate::artwork::PortraitState::Exhausted;
    }
    crate::artwork::PortraitState::Neutral
}

fn draw_run_effects(ctx: &UiContext<'_>, playback: &RunPlayback) {
    let reduced_motion = ctx.prefs.pacing.skips_the_run();
    let building = building_rect();
    let area = draw_area(building);
    let report = playback.report();
    let plan = floorplan::layout(
        area,
        report.doors.len(),
        floorplan::seed_for(&report.target_id),
    );
    let progress = playback.phase_progress().clamp(0.0, 1.0);

    if let Some(room) = plan.room(playback.door_index()) {
        match playback.phase() {
            DoorPhase::Approach => draw_paper_snap(room, progress, reduced_motion),
            DoorPhase::Tally => draw_scan_sweep(building, progress, reduced_motion),
            DoorPhase::Verdict => draw_brass_stamp(room, reduced_motion),
            DoorPhase::Roll => {}
        }
    }
    draw_paper_flecks(building, progress, reduced_motion);

    if let Some(door) = playback.current_door() {
        if playback.phase() == DoorPhase::Verdict
            && matches!(
                door.result.outcome,
                Outcome::Failure | Outcome::CriticalFailure
            )
        {
            draw_alarm_edge(building, reduced_motion);
        }
    }
}

fn draw_paper_snap(room: Rect, progress: f32, reduced_motion: bool) {
    let alpha = if reduced_motion {
        0.20
    } else {
        0.62 * (1.0 - progress * 0.35)
    };
    let paper = Color::new(0.88, 0.82, 0.66, alpha);
    draw_line(
        room.x - 8.0,
        room.y,
        room.x - 8.0 + progress * 14.0,
        room.y,
        2.0,
        paper,
    );
    draw_line(
        room.right() + 8.0,
        room.bottom(),
        room.right() + 8.0 - progress * 14.0,
        room.bottom(),
        2.0,
        paper,
    );
    draw_line(
        room.x,
        room.y - 8.0,
        room.x,
        room.y - 8.0 + progress * 12.0,
        1.5,
        paper,
    );
}

fn draw_scan_sweep(building: Rect, progress: f32, reduced_motion: bool) {
    let area = draw_area(building);
    let x = area.x + area.w * progress;
    let alpha = if reduced_motion { 0.18 } else { 0.68 };
    draw_line(
        x,
        area.y,
        x,
        area.bottom(),
        if reduced_motion { 1.0 } else { 2.0 },
        Color::new(0.24, 0.82, 0.86, alpha),
    );
    draw_line(
        x + 5.0,
        area.y + 10.0,
        x + 5.0,
        area.bottom() - 10.0,
        1.0,
        Color::new(0.24, 0.82, 0.86, alpha * 0.35),
    );
}

fn draw_brass_stamp(room: Rect, reduced_motion: bool) {
    let alpha = if reduced_motion { 0.28 } else { 0.68 };
    let brass = Color::new(0.78, 0.56, 0.22, alpha);
    let center = vec2(room.right() - 22.0, room.y + 22.0);
    draw_circle_lines(center.x, center.y, 14.0, 2.0, brass);
    draw_line(
        center.x - 7.0,
        center.y,
        center.x - 2.0,
        center.y + 5.0,
        2.0,
        brass,
    );
    draw_line(
        center.x - 2.0,
        center.y + 5.0,
        center.x + 8.0,
        center.y - 6.0,
        2.0,
        brass,
    );
}

fn draw_paper_flecks(building: Rect, progress: f32, reduced_motion: bool) {
    let alpha = if reduced_motion { 0.10 } else { 0.26 };
    let drift = if reduced_motion {
        0.0
    } else {
        (get_time() as f32 * 0.8 + progress * 4.0).sin() * 5.0
    };
    let paper = Color::new(0.84, 0.76, 0.58, alpha);
    for (index, x) in [12.0, building.w * 0.47, building.w - 22.0]
        .iter()
        .enumerate()
    {
        let y = building.bottom() - 25.0 + drift * (index as f32 * 0.25);
        draw_rectangle(building.x + *x, y, 7.0, 2.0, paper);
    }
}

fn draw_alarm_edge(building: Rect, reduced_motion: bool) {
    let pulse = if reduced_motion {
        0.16
    } else {
        0.22 + 0.20 * ((get_time() as f32 * 4.0).sin() * 0.5 + 0.5)
    };
    let alarm = Color::new(0.88, 0.20, 0.18, pulse);
    draw_rectangle_lines(
        building.x + 2.0,
        building.y + 2.0,
        building.w - 4.0,
        building.h - 4.0,
        2.0,
        alarm,
    );
    draw_circle(building.x + 10.0, building.y + 10.0, 5.0, alarm);
    draw_circle(building.right() - 10.0, building.y + 10.0, 5.0, alarm);
}

fn run_node_state(outcome: Option<Outcome>, active: bool) -> floorplan::NodeState {
    if active {
        floorplan::NodeState::InProgress
    } else {
        match outcome {
            Some(Outcome::CriticalSuccess) => floorplan::NodeState::CriticalSuccess,
            Some(Outcome::Success) => floorplan::NodeState::Success,
            Some(Outcome::Neutral) => floorplan::NodeState::Assigned,
            Some(Outcome::Failure) => floorplan::NodeState::Failure,
            Some(Outcome::CriticalFailure) => floorplan::NodeState::CriticalFailure,
            None => floorplan::NodeState::Ready,
        }
    }
}

fn draw_dice(ctx: &UiContext<'_>, playback: &RunPlayback, actions: &mut Vec<UiAction>) {
    let Some(door) = playback.current_door() else {
        return;
    };
    let content = draw_panel(dice_rect(), &door.encounter_name);
    let check = &door.result.check;

    draw_ui_text_ex(
        &format!("{} · {}", check.member_name, check.skill.label()),
        content.x,
        content.y + 16.0,
        TextStyle::new(16.0, dark::TEXT).params(),
    );
    draw_text_right(
        &format!("DC {}", check.dc),
        content.right(),
        content.y + 16.0,
        TextStyle::new(18.0, dark::TEXT_BRIGHT),
    );

    draw_die(
        Rect::new(content.x, content.y + 32.0, 108.0, 108.0),
        playback,
        door.result.roll,
        ctx.prefs.pacing.skips_the_run(),
    );

    // The modifiers stack up beside the die, one at a time, in as many
    // columns as it takes to show all of them (pillar 2).
    super::chrome::draw_modifier_grid(
        Rect::new(
            content.x + 124.0,
            content.y + 28.0,
            content.w - 124.0,
            114.0,
        ),
        check.significant(),
        playback.revealed_modifiers(),
        13.0,
    );

    draw_total(
        Rect::new(content.x, content.y + 152.0, content.w, 54.0),
        playback,
        check.dc,
    );
    draw_verdict(
        Rect::new(
            content.x,
            content.y + 214.0,
            content.w,
            content.bottom() - content.y - 260.0,
        ),
        playback,
        door,
    );

    draw_run_controls(ctx, content, playback, actions);
}

/// The die itself: tumbling while it is in the air, then landed and legible.
fn draw_die(rect: Rect, playback: &RunPlayback, roll: i32, reduced_motion: bool) {
    let landed = playback.roll_landed();
    let phase = playback.phase();
    let progress = playback.phase_progress();

    // A face that changes while the die is in the air. Derived from the
    // animation clock, never from the run's RNG — presentation must not consume
    // a draw the sim owns.
    let face = if landed {
        roll
    } else if phase == DoorPhase::Roll {
        1 + ((progress * 34.0) as i32 % 20)
    } else {
        20
    };

    let settle = if landed || reduced_motion {
        0.0
    } else {
        (1.0 - progress) * 5.0
    };
    let body = Rect::new(rect.x + settle, rect.y - settle * 0.6, rect.w, rect.h);
    let tone = if landed {
        Color::new(0.88, 0.82, 0.68, 1.0)
    } else {
        Color::new(0.10, 0.13, 0.18, 1.0)
    };
    let ink = if landed {
        Color::new(0.08, 0.10, 0.12, 1.0)
    } else {
        dark::TEXT_BRIGHT
    };

    draw_surface(
        body,
        &SurfaceStyle::new(tone)
            .with_border(2.0, Color::new(0.76, 0.54, 0.22, 0.95))
            .with_top_highlight(3.0, Color::new(0.98, 0.90, 0.68, 0.42)),
    );
    draw_text_centered_in_box_ex(
        &face.to_string(),
        body.x,
        body.y + 6.0,
        body.w,
        body.h,
        TextStyle::new(32.0, ink),
    );
    draw_text_centered_in_box_ex(
        "d20",
        body.x,
        body.bottom() - 22.0,
        body.w,
        18.0,
        TextStyle::new(13.0, dark::TEXT_DIM),
    );

    if landed && roll == 20 {
        let cyan = Color::new(0.24, 0.82, 0.86, if reduced_motion { 0.42 } else { 0.88 });
        draw_circle_lines(body.right() - 18.0, body.y + 18.0, 9.0, 1.5, cyan);
        draw_line(
            body.right() - 23.0,
            body.y + 18.0,
            body.right() - 19.0,
            body.y + 22.0,
            2.0,
            cyan,
        );
        draw_line(
            body.right() - 19.0,
            body.y + 22.0,
            body.right() - 13.0,
            body.y + 14.0,
            2.0,
            cyan,
        );
    } else if landed && roll == 1 {
        let red = Color::new(0.82, 0.22, 0.20, if reduced_motion { 0.40 } else { 0.86 });
        draw_line(
            body.x + 20.0,
            body.y + 8.0,
            body.x + 43.0,
            body.y + 45.0,
            2.0,
            red,
        );
        draw_line(
            body.x + 43.0,
            body.y + 45.0,
            body.x + 33.0,
            body.y + 75.0,
            2.0,
            red,
        );
        draw_line(
            body.x + 43.0,
            body.y + 45.0,
            body.x + 76.0,
            body.y + 93.0,
            2.0,
            red,
        );
    }
}

fn draw_total(rect: Rect, playback: &RunPlayback, dc: i32) {
    if !playback.roll_landed() {
        return;
    }

    let total = playback.running_total();
    let complete = matches!(playback.phase(), DoorPhase::Verdict);
    let color = if !complete {
        dark::TEXT
    } else if total >= dc {
        Color::new(0.52, 0.84, 0.58, 1.0)
    } else {
        Color::new(0.90, 0.46, 0.40, 1.0)
    };

    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.09, 0.10, 0.13, 1.0))
            .with_border(1.0, Color::new(0.42, 0.48, 0.58, 0.4)),
    );
    draw_ui_text_ex(
        "Total",
        rect.x + 12.0,
        rect.y + 34.0,
        TextStyle::new(16.0, dark::TEXT_DIM).params(),
    );
    draw_text_right(
        &format!("{} vs DC {}", total, dc),
        rect.right() - 12.0,
        rect.y + 36.0,
        TextStyle::new(26.0, color),
    );
}

fn draw_verdict(rect: Rect, playback: &RunPlayback, door: &crate::sim::DoorOutcome) {
    if playback.phase() != DoorPhase::Verdict {
        return;
    }

    let outcome = door.result.outcome;
    draw_ui_text_ex(
        outcome.label(),
        rect.x,
        rect.y + 20.0,
        TextStyle::new(22.0, outcome_color(outcome)).params(),
    );
    // A critical says what it did, in the encounter's own words, and then what
    // that did to the rest of the plan (GDD 5.2). Everything else just reads
    // its narrative line.
    let mut line = door.narrative.clone();
    if let Some(effect) = &door.critical_effect {
        line.push(' ');
        line.push_str(effect);
    }
    if let Some(note) = door.structural_note() {
        line.push(' ');
        line.push_str(note);
    }
    draw_text_block(
        &line,
        rect.x,
        rect.y + 30.0,
        rect.w,
        (rect.h - 34.0).max(20.0),
        15.0,
        4.0,
        dark::TEXT,
    );

    if let Some(injury) = &door.injury {
        draw_ui_text_ex(
            &injury.description,
            rect.x,
            rect.bottom() - 4.0,
            TextStyle::new(14.0, Color::new(0.90, 0.44, 0.38, 1.0)).params(),
        );
    }
}

fn draw_run_controls(
    ctx: &UiContext<'_>,
    content: Rect,
    playback: &RunPlayback,
    actions: &mut Vec<UiAction>,
) {
    let mouse = ctx.mouse();
    let y = content.bottom() - 38.0;
    let width = (content.w - 10.0) / 2.0;

    if button_rect_tone_at(
        Rect::new(content.x, y, width, 36.0),
        "Skip ahead",
        !playback.finished(),
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::SkipRun);
    }
    if button_rect_tone_at(
        Rect::new(content.x + width + 10.0, y, width, 36.0),
        "Read the results",
        playback.finished(),
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::FinishRun);
    }

    if !playback.finished() {
        draw_ui_text_ex(
            "Hold Space to fast-forward",
            content.x,
            y - 8.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
    }
}

pub fn outcome_color(outcome: Outcome) -> Color {
    match outcome {
        Outcome::CriticalSuccess => Color::new(0.42, 0.84, 0.52, 1.0),
        Outcome::Success => Color::new(0.46, 0.72, 0.50, 1.0),
        Outcome::Neutral => Color::new(0.82, 0.76, 0.42, 1.0),
        Outcome::Failure => Color::new(0.90, 0.56, 0.34, 1.0),
        Outcome::CriticalFailure => Color::new(0.88, 0.34, 0.34, 1.0),
    }
}
