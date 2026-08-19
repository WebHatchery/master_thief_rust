//! The floorplan: a building drawn from the job's data, with no art in it.
//!
//! The rooms come from a binary split of the panel rather than a grid, so a
//! four-door job and a five-door job are different buildings rather than the
//! same boxes rearranged, and the split ratios are drawn from a seed derived
//! from the mark's own id — the Velvet Room always has the Velvet Room's shape,
//! on the planning screen and again on the run.
//!
//! Layout and painting are split from the screen deliberately. Everything here
//! goes through [`Painter`], so the same routine that draws the plan on screen
//! draws it into a [`Buffer`](macroquad_toolkit::paint::Buffer) that a test can
//! measure — which is how a procedural drawing gets checked without a person
//! looking at it (GDD 5.4, 14).

pub use crate::model::Skill;
use crate::rules::Outcome;
use macroquad::prelude::*;
use macroquad_toolkit::paint::Painter;
use macroquad_toolkit::rng::SeededRng;

/// Corridor thickness, in pixels.
const CORRIDOR: f32 = 7.0;
/// Wall thickness drawn around each room.
const WALL: f32 = 2.0;
/// The narrowest a *cell* may get before the split stops being worth it. The
/// room inside it is smaller again by the inset, so this is deliberately well
/// above the size a room needs to stay readable.
const MIN_ROOM: f32 = 150.0;
/// Space left between a room and its cell, as a share of the cell.
const INSET_MIN: f32 = 0.05;
const INSET_MAX: f32 = 0.10;
/// The width a room is guaranteed to reach on the panels this game actually
/// uses. `MIN_ROOM` is what a split *aims* for and mostly achieves; a subtree
/// can still be handed a cell too small to halve twice, and then the last split
/// takes the least-bad option. This is the floor the planning screen may rely
/// on — enough for a door number, a short name, a skill and an assignment — and
/// a test holds the layout to it across every seed and door count.
pub const READABLE_ROOM: f32 = 100.0;
/// How far a split may wander from an even halving.
const SPLIT_JITTER: f32 = 0.14;

/// A laid-out building: one room per door, joined in the order the crew meets
/// them, with a stub of corridor marking the way in.
#[derive(Debug, Clone, PartialEq)]
pub struct Floorplan {
    pub bounds: Rect,
    pub rooms: Vec<Rect>,
    pub corridors: Vec<Rect>,
    /// The way in, drawn from the edge of the panel to the first room.
    pub entry: Option<Rect>,
}

impl Floorplan {
    pub fn room(&self, index: usize) -> Option<Rect> {
        self.rooms.get(index).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.rooms.is_empty()
    }

    /// Which room, if any, a point falls inside.
    pub fn room_at(&self, point: Vec2) -> Option<usize> {
        self.rooms.iter().position(|room| room.contains(point))
    }
}

/// A stable seed for a mark's building. Not `DefaultHasher`: that is explicitly
/// allowed to change between Rust releases, and a building that redraws itself
/// after a toolchain update is a bug nobody would think to look for.
pub fn seed_for(target_id: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in target_id.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Lay `count` rooms into `bounds`. Deterministic in both arguments and the
/// seed: the same mark always draws the same building.
pub fn layout(bounds: Rect, count: usize, seed: u64) -> Floorplan {
    if count == 0 || bounds.w <= 0.0 || bounds.h <= 0.0 {
        return Floorplan {
            bounds,
            rooms: Vec::new(),
            corridors: Vec::new(),
            entry: None,
        };
    }

    let mut rng = SeededRng::new(seed ^ (count as u64).wrapping_mul(0x9E37_79B9));
    let mut cells = Vec::with_capacity(count);
    partition(bounds, count, &mut rng, &mut cells);

    let rooms: Vec<Rect> = cells.iter().map(|cell| inset(*cell, &mut rng)).collect();
    let corridors = corridors_between(&rooms);
    let entry = entry_stub(bounds, rooms.first());

    Floorplan {
        bounds,
        rooms,
        corridors,
        entry,
    }
}

/// Split a cell in two until there is one per room, always cutting the longer
/// side so nothing ends up a corridor pretending to be a room.
fn partition(cell: Rect, count: usize, rng: &mut SeededRng, out: &mut Vec<Rect>) {
    if count <= 1 {
        out.push(cell);
        return;
    }

    let first = count / 2;
    let second = count - first;
    let wobble = rng.range_f32(-SPLIT_JITTER, SPLIT_JITTER);

    // Cut near the middle rather than in proportion to how many rooms each side
    // holds: a three-room cell cut one-third along leaves a sliver nobody could
    // put a name in, while cutting it in half leaves one large room and two
    // smaller ones, which is what a building looks like anyway. Room sizes end
    // up varied through the depth of the tree instead.
    //
    // The ratio is then pulled into whatever range keeps *both* halves at least
    // `MIN_ROOM` across, so the guarantee holds by construction rather than by
    // hoping the jitter behaves.
    let feasible = |length: f32| {
        (length >= MIN_ROOM * 2.0).then(|| {
            let margin = MIN_ROOM / length;
            (0.5 + wobble).clamp(margin, 1.0 - margin)
        })
    };

    let (horizontal, ratio) = match (feasible(cell.w), feasible(cell.h)) {
        // Both fit: cut the longer side, which keeps cells squarish.
        (Some(across), Some(along)) => {
            if cell.w >= cell.h {
                (true, across)
            } else {
                (false, along)
            }
        }
        (Some(across), None) => (true, across),
        (None, Some(along)) => (false, along),
        // Neither fits: the panel cannot hold this many rooms at this size, so
        // halve the longer side and accept smaller rooms rather than refusing
        // to draw a door.
        (None, None) => (cell.w >= cell.h, 0.5),
    };

    let (a, b) = if horizontal {
        let cut = (cell.w * ratio).round();
        (
            Rect::new(cell.x, cell.y, cut, cell.h),
            Rect::new(cell.x + cut, cell.y, cell.w - cut, cell.h),
        )
    } else {
        let cut = (cell.h * ratio).round();
        (
            Rect::new(cell.x, cell.y, cell.w, cut),
            Rect::new(cell.x, cell.y + cut, cell.w, cell.h - cut),
        )
    };

    // In-order traversal keeps consecutive rooms adjacent, which is what makes
    // the corridor between them read as a walk rather than a leap.
    partition(a, first, rng, out);
    partition(b, second, rng, out);
}

fn inset(cell: Rect, rng: &mut SeededRng) -> Rect {
    let x = cell.w * rng.range_f32(INSET_MIN, INSET_MAX);
    let y = cell.h * rng.range_f32(INSET_MIN, INSET_MAX);
    Rect::new(
        (cell.x + x).round(),
        (cell.y + y).round(),
        (cell.w - x * 2.0).round().max(8.0),
        (cell.h - y * 2.0).round().max(8.0),
    )
}

/// An L of corridor between each pair of rooms: out sideways, then along.
/// Corridors are drawn under the rooms, so one crossing a room simply reads as
/// the room's own doorway.
fn corridors_between(rooms: &[Rect]) -> Vec<Rect> {
    let mut corridors = Vec::new();
    let half = CORRIDOR * 0.5;

    for pair in rooms.windows(2) {
        let from = centre(pair[0]);
        let to = centre(pair[1]);

        let (left, right) = if from.x <= to.x {
            (from.x, to.x)
        } else {
            (to.x, from.x)
        };
        corridors.push(Rect::new(
            left,
            from.y - half,
            (right - left).max(CORRIDOR),
            CORRIDOR,
        ));

        let (top, bottom) = if from.y <= to.y {
            (from.y, to.y)
        } else {
            (to.y, from.y)
        };
        corridors.push(Rect::new(
            to.x - half,
            top,
            CORRIDOR,
            (bottom - top).max(CORRIDOR),
        ));
    }

    corridors
}

/// The way in: a stub from the nearest edge of the panel to the first room.
fn entry_stub(bounds: Rect, first: Option<&Rect>) -> Option<Rect> {
    let room = first?;
    let y = room.y + room.h * 0.5 - CORRIDOR * 0.5;
    let from_left = room.x - bounds.x;
    let from_right = bounds.right() - room.right();

    Some(if from_left <= from_right {
        Rect::new(bounds.x, y, from_left.max(CORRIDOR), CORRIDOR)
    } else {
        Rect::new(room.right(), y, from_right.max(CORRIDOR), CORRIDOR)
    })
}

fn centre(rect: Rect) -> Vec2 {
    vec2(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5)
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
    pub fn assigned(assigned: bool) -> Self {
        Self {
            assigned,
            active: false,
            outcome: None,
        }
    }

    pub fn resolved(outcome: Outcome) -> Self {
        Self {
            assigned: true,
            active: false,
            outcome: Some(outcome),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Unknown,
    Cased,
    Locked,
    Selected,
    Assigned,
    Ready,
    InProgress,
    Success,
    Failure,
    CriticalSuccess,
    CriticalFailure,
    Skipped,
}

mod symbols;
pub use symbols::{
    draw_door_silhouette, draw_factor_glyph, draw_node_state, draw_route_ink, draw_skill_glyph,
};

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
            unassigned: Color::new(0.11, 0.12, 0.155, 1.0),
            assigned: Color::new(0.14, 0.19, 0.26, 1.0),
            active: Color::new(0.20, 0.30, 0.42, 1.0),
            passed: Color::new(0.13, 0.30, 0.18, 1.0),
            scraped: Color::new(0.30, 0.27, 0.13, 1.0),
            failed: Color::new(0.32, 0.14, 0.15, 1.0),
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

    fn room_wall(&self, state: &RoomState) -> Color {
        match state.outcome {
            Some(Outcome::CriticalSuccess) | Some(Outcome::Success) => {
                Color::new(0.40, 0.68, 0.46, 1.0)
            }
            Some(Outcome::Neutral) => Color::new(0.72, 0.66, 0.36, 1.0),
            Some(Outcome::Failure) | Some(Outcome::CriticalFailure) => {
                Color::new(0.78, 0.36, 0.34, 1.0)
            }
            None if state.active => Color::new(0.56, 0.72, 0.94, 1.0),
            _ => self.wall,
        }
    }
}

/// Draw the building. Corridors first so the rooms sit on top of them, then
/// each room as a wall with a floor inset inside it. Text is the screen's job,
/// not the painter's.
pub fn paint<P: Painter>(
    painter: &mut P,
    plan: &Floorplan,
    states: &[RoomState],
    palette: &FloorplanPalette,
) {
    for corridor in plan.entry.iter().chain(plan.corridors.iter()) {
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
        painter.rect(
            vec2(room.x, room.y),
            vec2(room.w, room.h),
            palette.room_wall(&state),
        );
        painter.rect(
            vec2(room.x + WALL, room.y + WALL),
            vec2(
                (room.w - WALL * 2.0).max(1.0),
                (room.h - WALL * 2.0).max(1.0),
            ),
            palette.room_fill(&state),
        );

        // "The crew is here", marked in the corner rather than the middle: the
        // middle of a room is where its label goes.
        if state.active {
            let size = 12.0;
            painter.rect(
                vec2(room.right() - size - 10.0, room.bottom() - size - 10.0),
                vec2(size, size),
                palette.room_wall(&state),
            );
        }
    }
}

#[cfg(test)]
mod tests;
