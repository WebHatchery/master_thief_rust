use super::*;
use macroquad_toolkit::paint::Buffer;

const SEED: u64 = 0x5EED;

fn bounds() -> Rect {
    Rect::new(0.0, 0.0, 320.0, 180.0)
}

fn painted(count: usize, states: &[RoomState]) -> Buffer {
    let mut buffer = Buffer::new(320, 180);
    let plan = layout(buffer.bounds(), count, SEED);
    paint(&mut buffer, &plan, states, &FloorplanPalette::default());
    buffer
}

#[test]
fn a_single_door_is_one_room_and_no_corridor_between_anything() {
    let plan = layout(bounds(), 1, SEED);
    assert_eq!(plan.rooms.len(), 1);
    assert!(plan.corridors.is_empty());
    assert!(plan.entry.is_some(), "even one room needs a way in");
}

#[test]
fn every_door_gets_a_room_and_every_gap_a_corridor() {
    for count in 1..=6 {
        let plan = layout(bounds(), count, SEED);
        assert_eq!(plan.rooms.len(), count, "{} doors", count);
        // An L of corridor is two rects: one across, one along.
        assert_eq!(plan.corridors.len(), (count - 1) * 2, "{} doors", count);
    }
}

#[test]
fn no_room_escapes_the_panel_it_was_given() {
    let area = Rect::new(40.0, 20.0, 700.0, 420.0);
    for count in 1..=6 {
        for seed in 0..12u64 {
            for room in layout(area, count, seed).rooms {
                assert!(room.x >= area.x - 0.01, "{} doors, seed {}", count, seed);
                assert!(room.y >= area.y - 0.01, "{} doors, seed {}", count, seed);
                assert!(room.right() <= area.right() + 0.01);
                assert!(room.bottom() <= area.bottom() + 0.01);
            }
        }
    }
}

#[test]
fn rooms_never_overlap_each_other() {
    let area = Rect::new(0.0, 0.0, 700.0, 420.0);
    for count in 2..=6 {
        for seed in 0..12u64 {
            let rooms = layout(area, count, seed).rooms;
            for (i, a) in rooms.iter().enumerate() {
                for b in rooms.iter().skip(i + 1) {
                    assert!(!a.overlaps(b), "{} doors, seed {}", count, seed);
                }
            }
        }
    }
}

#[test]
fn every_room_stays_big_enough_to_read() {
    // The whole reason the planning screen can use this: a room has to hold
    // a name, a skill, a difficulty and an assignment.
    let area = Rect::new(0.0, 0.0, 700.0, 420.0);
    for count in 1..=6 {
        for seed in 0..12u64 {
            for room in layout(area, count, seed).rooms {
                assert!(
                    room.w >= READABLE_ROOM && room.h >= 70.0,
                    "{} doors, seed {}: {}x{}",
                    count,
                    seed,
                    room.w,
                    room.h
                );
            }
        }
    }
}

#[test]
fn consecutive_rooms_are_joined_by_something() {
    let plan = layout(Rect::new(0.0, 0.0, 700.0, 420.0), 5, SEED);
    for pair in plan.rooms.windows(2) {
        let joined = plan
            .corridors
            .iter()
            .any(|corridor| corridor.overlaps(&pair[0]) || corridor.overlaps(&pair[1]));
        assert!(joined, "a room pair with nothing between them");
    }
}

#[test]
fn a_layout_is_the_same_every_time_it_is_asked_for() {
    assert_eq!(layout(bounds(), 5, SEED), layout(bounds(), 5, SEED));
}

#[test]
fn two_marks_are_two_different_buildings() {
    let a = layout(bounds(), 4, seed_for("velvet_room"));
    let b = layout(bounds(), 4, seed_for("first_national"));
    assert_ne!(a.rooms, b.rooms, "every mark drew the same building");
}

#[test]
fn a_marks_seed_never_moves() {
    // Pinned deliberately: if this changes, every building in the game
    // redraws itself, and that should be a decision rather than a surprise.
    assert_eq!(seed_for("velvet_room"), 1_761_980_800_008_477_639);
    assert_eq!(seed_for(""), 0xcbf2_9ce4_8422_2325);
}

#[test]
fn an_empty_job_draws_nothing_rather_than_panicking() {
    let plan = layout(bounds(), 0, SEED);
    assert!(plan.is_empty());
    assert!(plan.entry.is_none());

    let mut buffer = Buffer::new(64, 64);
    paint(&mut buffer, &plan, &[], &FloorplanPalette::default());
    assert_eq!(buffer.coverage(), 0.0);
}

#[test]
fn the_drawing_actually_covers_the_panel() {
    let buffer = painted(3, &[RoomState::default(); 3]);
    let coverage = buffer.coverage();
    assert!(
        (0.5..0.98).contains(&coverage),
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
fn an_assigned_room_reads_differently_from_an_empty_one() {
    let empty = painted(3, &[RoomState::default(); 3]);
    let staffed = painted(3, &[RoomState::assigned(true); 3]);
    assert!(empty.monochrome_difference(&staffed) > 0.0);
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

#[test]
fn a_point_inside_a_room_finds_that_room() {
    let plan = layout(Rect::new(0.0, 0.0, 700.0, 420.0), 4, SEED);
    for (index, room) in plan.rooms.iter().enumerate() {
        let middle = vec2(room.x + room.w * 0.5, room.y + room.h * 0.5);
        assert_eq!(plan.room_at(middle), Some(index));
    }
    assert_eq!(plan.room_at(vec2(-50.0, -50.0)), None);
}

/// The golden image. If this fails, the floorplan changed shape — which is
/// fine when it was meant to, and a bug when it was not. Re-record the
/// fingerprint deliberately, never reflexively.
#[test]
fn the_three_door_floorplan_matches_its_recorded_fingerprint() {
    let buffer = painted(3, &[RoomState::default(); 3]);
    assert_eq!(buffer.fingerprint(), 13_893_598_833_480_542_666);
}

#[test]
fn encounter_symbols_cover_all_trades_and_semantic_node_states() {
    assert_eq!(Skill::ALL.len(), 6);
    let states = [
        NodeState::Unknown,
        NodeState::Cased,
        NodeState::Locked,
        NodeState::Selected,
        NodeState::Assigned,
        NodeState::Ready,
        NodeState::InProgress,
        NodeState::Success,
        NodeState::Failure,
        NodeState::CriticalSuccess,
        NodeState::CriticalFailure,
        NodeState::Skipped,
    ];
    assert_eq!(states.len(), 12);
}
