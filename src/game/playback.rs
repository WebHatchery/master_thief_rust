//! Watching the job happen.
//!
//! The dice are already cast: `run_job` resolves the whole thing from the run's
//! seeded RNG at the moment of commit, and this replays that report at a pace a
//! person can read. Presentation never touches the outcome — which is the only
//! way the dice can be given real weight without making them a lie (GDD 9).

use crate::rules::Outcome;
use crate::sim::{DoorOutcome, JobReport};
use macroquad_toolkit::timing::Timeline;

/// The beats of one door. The roll lands, then the modifiers total up against
/// the difficulty class, and only then does the verdict read out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorPhase {
    /// The crew reaches the door and the assignment is named.
    Approach,
    /// The die is in the air.
    Roll,
    /// Modifiers stack onto the roll, one at a time.
    Tally,
    /// The total stands against the DC and the outcome reads out.
    Verdict,
}

const APPROACH: f32 = 0.55;
const ROLL: f32 = 0.70;
const TALLY: f32 = 1.05;
const VERDICT: f32 = 0.95;

/// How much faster the run goes while the player holds the skip key.
const FAST_FORWARD: f32 = 8.0;

pub struct RunPlayback {
    report: JobReport,
    door: usize,
    timeline: Timeline<DoorPhase>,
    finished: bool,
}

impl RunPlayback {
    pub fn new(report: JobReport) -> Self {
        let finished = report.doors.is_empty();
        Self {
            report,
            door: 0,
            timeline: door_timeline(),
            finished,
        }
    }

    pub fn update(&mut self, dt: f32, fast_forward: bool) {
        if self.finished {
            return;
        }

        let scale = if fast_forward { FAST_FORWARD } else { 1.0 };
        self.timeline.advance(dt * scale);

        while self.timeline.finished() && !self.finished {
            self.door += 1;
            if self.door >= self.report.doors.len() {
                self.door = self.report.doors.len().saturating_sub(1);
                self.finished = true;
            } else {
                self.timeline = door_timeline();
            }
        }
    }

    /// Jump to the end. The report is unchanged — only the watching is skipped.
    pub fn skip_to_end(&mut self) {
        self.door = self.report.doors.len().saturating_sub(1);
        self.timeline.skip_to_end();
        self.finished = true;
    }

    pub fn finished(&self) -> bool {
        self.finished
    }

    pub fn report(&self) -> &JobReport {
        &self.report
    }

    pub fn into_report(self) -> JobReport {
        self.report
    }

    pub fn door_index(&self) -> usize {
        self.door
    }

    pub fn current_door(&self) -> Option<&DoorOutcome> {
        self.report.doors.get(self.door)
    }

    pub fn phase(&self) -> DoorPhase {
        if self.finished {
            return DoorPhase::Verdict;
        }
        self.timeline
            .current()
            .map(|(phase, _)| *phase)
            .unwrap_or(DoorPhase::Verdict)
    }

    /// Progress through the current beat, 0..1.
    pub fn phase_progress(&self) -> f32 {
        if self.finished {
            return 1.0;
        }
        self.timeline
            .current()
            .map(|(_, progress)| progress)
            .unwrap_or(1.0)
    }

    /// Has the die landed yet? Before this the face shown is still tumbling.
    pub fn roll_landed(&self) -> bool {
        !matches!(self.phase(), DoorPhase::Approach | DoorPhase::Roll)
    }

    /// How many of the check's modifier lines have stacked up so far.
    pub fn revealed_modifiers(&self) -> usize {
        let Some(door) = self.current_door() else {
            return 0;
        };
        let total = door.result.check.significant().count();

        match self.phase() {
            DoorPhase::Approach | DoorPhase::Roll => 0,
            DoorPhase::Tally => ((self.phase_progress() * total as f32).ceil() as usize).min(total),
            DoorPhase::Verdict => total,
        }
    }

    /// The total as it stands mid-tally: the roll plus whatever has landed.
    pub fn running_total(&self) -> i32 {
        let Some(door) = self.current_door() else {
            return 0;
        };
        if !self.roll_landed() {
            return 0;
        }

        let revealed = self.revealed_modifiers();
        door.result.roll
            + door
                .result
                .check
                .significant()
                .take(revealed)
                .map(|entry| entry.value)
                .sum::<i32>()
    }

    /// The outcome of each door as far as the watching has got — doors still
    /// ahead read as unresolved, so the floorplan lights up in order.
    pub fn outcomes_so_far(&self) -> Vec<Option<Outcome>> {
        self.report
            .doors
            .iter()
            .enumerate()
            .map(|(index, door)| {
                let settled = index < self.door
                    || (index == self.door
                        && (self.finished || self.phase() == DoorPhase::Verdict));
                settled.then_some(door.result.outcome)
            })
            .collect()
    }

    /// The outcome to punctuate this frame, once, as the verdict lands.
    pub fn verdict_just_landed(&self, previous_phase: DoorPhase, previous_door: usize) -> bool {
        self.phase() == DoorPhase::Verdict
            && (previous_phase != DoorPhase::Verdict || previous_door != self.door)
    }
}

fn door_timeline() -> Timeline<DoorPhase> {
    Timeline::new(vec![
        (DoorPhase::Approach, APPROACH),
        (DoorPhase::Roll, ROLL),
        (DoorPhase::Tally, TALLY),
        (DoorPhase::Verdict, VERDICT),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::sim;
    use crate::state::GameSession;

    fn report(seed: u64) -> JobReport {
        let data = GameData::load().unwrap();
        let mut session = GameSession::new(&data.config, &data, seed);
        let target = data
            .targets
            .get(&session.board[0].target_id)
            .unwrap()
            .clone();
        let plan = sim::auto_assign(&session, &data, &target);
        sim::run_job(&mut session, &data, &plan)
    }

    fn run_to_end(playback: &mut RunPlayback) {
        for _ in 0..10_000 {
            if playback.finished() {
                return;
            }
            playback.update(1.0 / 60.0, false);
        }
        panic!("playback never finished");
    }

    #[test]
    fn a_run_opens_on_the_first_door_approaching() {
        let playback = RunPlayback::new(report(101));
        assert_eq!(playback.door_index(), 0);
        assert_eq!(playback.phase(), DoorPhase::Approach);
        assert!(!playback.finished());
    }

    #[test]
    fn the_beats_arrive_in_order() {
        let mut playback = RunPlayback::new(report(102));
        let mut seen = vec![playback.phase()];

        for _ in 0..40 {
            playback.update(0.1, false);
            let phase = playback.phase();
            if seen.last() != Some(&phase) {
                seen.push(phase);
            }
            if playback.door_index() > 0 {
                break;
            }
        }

        assert_eq!(
            &seen[..4],
            &[
                DoorPhase::Approach,
                DoorPhase::Roll,
                DoorPhase::Tally,
                DoorPhase::Verdict
            ]
        );
    }

    #[test]
    fn the_die_is_still_in_the_air_until_the_roll_beat_ends() {
        let mut playback = RunPlayback::new(report(103));
        assert!(!playback.roll_landed());

        playback.update(APPROACH + ROLL * 0.5, false);
        assert!(!playback.roll_landed());

        playback.update(ROLL, false);
        assert!(playback.roll_landed());
    }

    #[test]
    fn modifiers_stack_one_at_a_time_and_finish_complete() {
        let mut playback = RunPlayback::new(report(104));
        assert_eq!(playback.revealed_modifiers(), 0);

        playback.update(APPROACH + ROLL + 0.001, false);
        let early = playback.revealed_modifiers();
        playback.update(TALLY * 0.5, false);
        let mid = playback.revealed_modifiers();

        assert!(early <= mid);
        playback.update(TALLY, false);

        let total = playback
            .current_door()
            .unwrap()
            .result
            .check
            .significant()
            .count();
        assert_eq!(playback.revealed_modifiers(), total);
    }

    #[test]
    fn the_running_total_arrives_at_the_number_the_engine_rolled() {
        let mut playback = RunPlayback::new(report(105));
        playback.update(APPROACH + ROLL + TALLY + 0.01, false);

        let door = playback.current_door().unwrap();
        // `significant()` hides zero-valued modifiers, so the tally lands on the
        // roll plus every line the player was actually shown.
        let shown: i32 = door.result.check.significant().map(|e| e.value).sum();
        assert_eq!(playback.running_total(), door.result.roll + shown);
    }

    #[test]
    fn the_floorplan_only_lights_doors_that_have_resolved() {
        let mut playback = RunPlayback::new(report(106));
        assert!(playback.outcomes_so_far().iter().all(|o| o.is_none()));

        playback.update(APPROACH + ROLL + TALLY + 0.01, false);
        let lit = playback.outcomes_so_far();
        assert!(lit[0].is_some(), "the door being read should be lit");
        assert!(
            lit.iter().skip(1).all(|o| o.is_none()),
            "doors ahead must stay dark"
        );
    }

    #[test]
    fn a_run_reaches_its_last_door_and_stops() {
        let mut playback = RunPlayback::new(report(107));
        let doors = playback.report().doors.len();
        run_to_end(&mut playback);

        assert!(playback.finished());
        assert_eq!(playback.door_index(), doors - 1);
        assert!(playback.outcomes_so_far().iter().all(|o| o.is_some()));
    }

    #[test]
    fn holding_the_skip_key_gets_there_sooner_and_changes_nothing_else() {
        let mut slow = RunPlayback::new(report(108));
        let mut fast = RunPlayback::new(report(108));

        for _ in 0..60 {
            slow.update(1.0 / 60.0, false);
            fast.update(1.0 / 60.0, true);
        }

        assert!(fast.door_index() >= slow.door_index());
        assert_eq!(
            fast.report().doors.len(),
            slow.report().doors.len(),
            "fast-forward must not skip a door"
        );
    }

    #[test]
    fn skipping_to_the_end_leaves_the_report_intact() {
        let mut playback = RunPlayback::new(report(109));
        let expected: Vec<Outcome> = playback
            .report()
            .doors
            .iter()
            .map(|door| door.result.outcome)
            .collect();

        playback.skip_to_end();
        assert!(playback.finished());

        let report = playback.into_report();
        let actual: Vec<Outcome> = report.doors.iter().map(|d| d.result.outcome).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn a_job_with_no_doors_is_finished_the_moment_it_starts() {
        let mut empty = report(110);
        empty.doors.clear();
        let playback = RunPlayback::new(empty);

        assert!(playback.finished());
        assert!(playback.current_door().is_none());
        assert_eq!(playback.running_total(), 0);
    }
}
