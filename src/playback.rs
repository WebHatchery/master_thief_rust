//! Watching a resolved job happen.
//!
//! The dice are already cast: `run_job` resolves the whole thing from the
//! run's seeded RNG at the moment of commit, and this replays that report at a
//! pace a person can read. Presentation never touches the outcome — which is
//! the only way the dice can be given real weight without making them a lie
//! (GDD 9).

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

    /// Jump to the end. The report is unchanged — only the watching is
    /// skipped.
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
mod tests;
