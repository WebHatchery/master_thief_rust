//! The floorplan: a building drawn from the job's data, with no art in it.
//!
//! Layout and painting are split from the screen deliberately. Everything here
//! goes through [`Painter`], so the same routine that draws the plan on screen
//! draws it into a [`Buffer`](macroquad_toolkit::paint::Buffer) that a test can
//! measure — which is how a procedural drawing gets checked without a person
//! looking at it (GDD 5.4, 14).

use crate::rules::Outcome;
use macroquad::prelude::*;
use macroquad_toolkit::paint::Painter;

/// Rooms per row before the route doubles back.
const ROOMS_PER_ROW: usize = 3;
/// Share of a cell left as space around its room.
const ROOM_INSET: f32 = 0.16;
/// Corridor thickness, in pixels.
const CORRIDOR: f32 = 6.0;
/// Tallest a room may be relative to its width.
const ROOM_ASPECT: f32 = 0.92;

/// A laid-out building: one room per door, joined in the order the crew meets
/// them.
#[derive(Debug, Clone, PartialEq)]
pub struct Floorplan {
    pub bounds: Rect,
    pub rooms: Vec<Rect>,
    pub corridors: Vec<Rect>,
}

impl Floorplan {
    pub fn room(&self, index: usize) -> Option<Rect> {
        self.rooms.get(index).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.rooms.is_empty()
    }
}

/// Lay `count` rooms into `bounds` as a serpentine route: left to right, drop a
/// row, right to left. Deterministic — the same job always draws the same
/// building.
pub fn layout(bounds: Rect, count: usize) -> Floorplan {
    if count == 0 || bounds.w <= 0.0 || bounds.h <= 0.0 {
        return Floorplan {
            bounds,
            rooms: Vec::new(),
            corridors: Vec::new(),
        };
    }

    let columns = count.min(ROOMS_PER_ROW);
    let rows = count.div_ceil(columns);
    let cell_w = bounds.w / columns as f32;
    let cell_h = bounds.h / rows as f32;
    let inset_x = cell_w * ROOM_INSET;
    let inset_y = cell_h * ROOM_INSET;

    // A room that fills a tall cell reads as a tower, not a room. Cap the
    // height against the width and centre what is left in the cell, so a
    // three-door job draws three rooms rather than three columns.
    let room_w = cell_w - inset_x * 2.0;
    let room_h = (cell_h - inset_y * 2.0).min(room_w * ROOM_ASPECT);
    let slack_y = (cell_h - room_h) * 0.5;

    let mut rooms = Vec::with_capacity(count);
    for index in 0..count {
        let row = index / columns;
        let step = index % columns;
        // Odd rows run backwards, so the route reads as one continuous walk.
        let column = if row.is_multiple_of(2) {
            step
        } else {
            columns - 1 - step
        };

        rooms.push(Rect::new(
            bounds.x + column as f32 * cell_w + inset_x,
            bounds.y + row as f32 * cell_h + slack_y,
            room_w,
            room_h,
        ));
    }

    let corridors = corridors_between(&rooms, columns);
    Floorplan {
        bounds,
        rooms,
        corridors,
    }
}

fn corridors_between(rooms: &[Rect], columns: usize) -> Vec<Rect> {
    let mut corridors = Vec::new();

    for index in 1..rooms.len() {
        let from = rooms[index - 1];
        let to = rooms[index];
        let same_row = (index - 1) / columns == index / columns;

        if same_row {
            let (left, right) = if from.x < to.x {
                (from, to)
            } else {
                (to, from)
            };
            let y = from.y + from.h * 0.5 - CORRIDOR * 0.5;
            corridors.push(Rect::new(
                left.right(),
                y,
                (right.x - left.right()).max(1.0),
                CORRIDOR,
            ));
        } else {
            // The route turns: drop straight down the shared column.
            let x = from.x + from.w * 0.5 - CORRIDOR * 0.5;
            corridors.push(Rect::new(
                x,
                from.bottom(),
                CORRIDOR,
                (to.y - from.bottom()).max(1.0),
            ));
        }
    }

    corridors
}

/// How one room should read: unassigned, staffed, currently being resolved, or
/// finished with an outcome.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoomState {
    pub assigned: bool,
    pub active: bool,
    pub outcome: Option<Outcome>,
}

impl RoomState {
    pub fn resolved(outcome: Outcome) -> Self {
        Self {
            assigned: true,
            active: false,
            outcome: Some(outcome),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FloorplanPalette {
    pub wall: Color,
    pub corridor: Color,
    pub unassigned: Color,
    pub assigned: Color,
    pub active: Color,
    pub passed: Color,
    pub scraped: Color,
    pub failed: Color,
}

impl Default for FloorplanPalette {
    fn default() -> Self {
        Self {
            wall: Color::new(0.32, 0.38, 0.50, 1.0),
            corridor: Color::new(0.22, 0.26, 0.34, 1.0),
            unassigned: Color::new(0.13, 0.14, 0.18, 1.0),
            assigned: Color::new(0.16, 0.22, 0.30, 1.0),
            active: Color::new(0.24, 0.34, 0.46, 1.0),
            passed: Color::new(0.16, 0.34, 0.20, 1.0),
            scraped: Color::new(0.34, 0.31, 0.14, 1.0),
            failed: Color::new(0.36, 0.16, 0.16, 1.0),
        }
    }
}

impl FloorplanPalette {
    fn room_fill(&self, state: &RoomState) -> Color {
        match state.outcome {
            Some(Outcome::CriticalSuccess) | Some(Outcome::Success) => self.passed,
            Some(Outcome::Neutral) => self.scraped,
            Some(Outcome::Failure) | Some(Outcome::CriticalFailure) => self.failed,
            None if state.active => self.active,
            None if state.assigned => self.assigned,
            None => self.unassigned,
        }
    }
}

/// Draw the building. Walls, corridors, floors, and a marker on the room the
/// crew is standing in. Text is the screen's job, not the painter's.
pub fn paint<P: Painter>(
    painter: &mut P,
    plan: &Floorplan,
    states: &[RoomState],
    palette: &FloorplanPalette,
) {
    for corridor in &plan.corridors {
        painter.rect(
            vec2(corridor.x, corridor.y),
            vec2(corridor.w, corridor.h),
            palette.corridor,
        );
    }

    for (index, room) in plan.rooms.iter().enumerate() {
        let state = states.get(index).copied().unwrap_or_default();

        // Wall, then floor inset inside it — two rects rather than an outline,
        // because the painter deals in filled shapes only.
        painter.rect(vec2(room.x, room.y), vec2(room.w, room.h), palette.wall);
        painter.rect(
            vec2(room.x + 2.0, room.y + 2.0),
            vec2((room.w - 4.0).max(1.0), (room.h - 4.0).max(1.0)),
            palette.room_fill(&state),
        );

        if state.active {
            painter.circle(
                vec2(room.x + room.w * 0.5, room.y + room.h * 0.5),
                (room.h * 0.16).max(3.0),
                palette.wall,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macroquad_toolkit::paint::Buffer;

    fn bounds() -> Rect {
        Rect::new(0.0, 0.0, 320.0, 180.0)
    }

    fn painted(count: usize, states: &[RoomState]) -> Buffer {
        let mut buffer = Buffer::new(320, 180);
        let plan = layout(buffer.bounds(), count);
        paint(&mut buffer, &plan, states, &FloorplanPalette::default());
        buffer
    }

    #[test]
    fn a_single_door_is_one_room_and_no_corridor() {
        let plan = layout(bounds(), 1);
        assert_eq!(plan.rooms.len(), 1);
        assert!(plan.corridors.is_empty());
    }

    #[test]
    fn every_door_gets_a_room_and_every_gap_a_corridor() {
        for count in 1..=6 {
            let plan = layout(bounds(), count);
            assert_eq!(plan.rooms.len(), count, "{} doors", count);
            assert_eq!(plan.corridors.len(), count - 1, "{} doors", count);
        }
    }

    #[test]
    fn no_room_escapes_the_panel_it_was_given() {
        let area = Rect::new(40.0, 20.0, 300.0, 200.0);
        for count in 1..=6 {
            for room in layout(area, count).rooms {
                assert!(room.x >= area.x - 0.01, "{} doors", count);
                assert!(room.y >= area.y - 0.01, "{} doors", count);
                assert!(room.right() <= area.right() + 0.01, "{} doors", count);
                assert!(room.bottom() <= area.bottom() + 0.01, "{} doors", count);
            }
        }
    }

    #[test]
    fn rooms_never_overlap_each_other() {
        for count in 2..=6 {
            let rooms = layout(bounds(), count).rooms;
            for (i, a) in rooms.iter().enumerate() {
                for b in rooms.iter().skip(i + 1) {
                    assert!(!a.overlaps(b), "{} doors: {:?} vs {:?}", count, a, b);
                }
            }
        }
    }

    #[test]
    fn the_route_doubles_back_rather_than_running_off_the_edge() {
        // Four doors wrap to a second row, and the fourth sits under the third.
        let plan = layout(bounds(), 4);
        assert!(plan.rooms[3].y > plan.rooms[0].y, "row two sits lower");
        assert!(
            (plan.rooms[3].x - plan.rooms[2].x).abs() < 0.01,
            "the turn happens under the last room of row one"
        );
    }

    #[test]
    fn a_layout_is_the_same_every_time_it_is_asked_for() {
        assert_eq!(layout(bounds(), 5), layout(bounds(), 5));
    }

    #[test]
    fn an_empty_job_draws_nothing_rather_than_panicking() {
        let plan = layout(bounds(), 0);
        assert!(plan.is_empty());

        let mut buffer = Buffer::new(64, 64);
        paint(&mut buffer, &plan, &[], &FloorplanPalette::default());
        assert_eq!(buffer.coverage(), 0.0);
    }

    #[test]
    fn the_drawing_actually_covers_the_panel() {
        let buffer = painted(3, &[RoomState::default(); 3]);
        let coverage = buffer.coverage();
        assert!(
            (0.2..0.7).contains(&coverage),
            "floorplan covers {:.0}% of its panel",
            coverage * 100.0
        );
    }

    #[test]
    fn a_resolved_run_looks_different_from_an_unplanned_one() {
        let blank = painted(3, &[RoomState::default(); 3]);
        let resolved = painted(
            3,
            &[
                RoomState::resolved(Outcome::Success),
                RoomState::resolved(Outcome::Neutral),
                RoomState::resolved(Outcome::CriticalFailure),
            ],
        );

        assert!(
            blank.monochrome_difference(&resolved) > 0.01,
            "outcome colours never reached the floor"
        );
        assert_eq!(
            blank.silhouette_difference(&resolved),
            0.0,
            "outcomes must not move the walls"
        );
    }

    #[test]
    fn the_active_room_is_marked() {
        let idle = painted(3, &[RoomState::default(); 3]);
        let mut states = [RoomState::default(); 3];
        states[1].active = true;
        let marked = painted(3, &states);

        assert!(idle.monochrome_difference(&marked) > 0.0);
    }

    #[test]
    fn a_longer_job_draws_a_different_building() {
        let three = painted(3, &[RoomState::default(); 3]);
        let six = painted(6, &[RoomState::default(); 6]);
        assert!(three.silhouette_difference(&six) > 0.05);
        assert_ne!(three.fingerprint(), six.fingerprint());
    }

    /// The golden image. If this fails, the floorplan changed shape — which is
    /// fine when it was meant to, and a bug when it was not. Re-record the
    /// fingerprint deliberately, never reflexively.
    #[test]
    fn the_three_door_floorplan_matches_its_recorded_fingerprint() {
        let buffer = painted(3, &[RoomState::default(); 3]);
        assert_eq!(buffer.fingerprint(), 9_972_975_190_272_715_713);
    }
}
