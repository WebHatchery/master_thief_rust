//! Planning a job: the draft the player builds before committing to it.
//!
//! This is the pre-commit side of the same engine the run uses. Every number
//! shown on the planning screen comes from [`crate::rules::encounter::build_check`],
//! so what the player reads before committing is exactly what the die is added
//! to afterwards (pillar 2).

use super::job::{Assignment, JobPlan};
use crate::data::GameData;
use crate::model::{CrewMember, Encounter, HeistTarget};
use crate::rules::encounter::{build_check, CheckBreakdown, CheckInputs};
use crate::rules::environment::environment_entries;
use crate::rules::outcome::ModifierEntry;
use crate::state::GameSession;

/// The environment, the city's attention, and the company a hand is keeping —
/// every modifier that comes from outside the member and their kit, named.
/// Shared by the planning screen and the run so neither can drift from the
/// other.
pub fn situational_modifiers(
    data: &GameData,
    target: &HeistTarget,
    encounter: &Encounter,
    session: &GameSession,
    member_id: &str,
    crew_on_job: &[String],
) -> Vec<ModifierEntry> {
    let mut extras = environment_entries(&target.environment, encounter.primary_skill, |id| {
        data.environment.get(id)
    });

    let heat = session.heat_dc_penalty(&data.config);
    if heat > 0 {
        extras.push(ModifierEntry::new("City heat", -heat));
    }

    // A mark nobody took has had time to notice it is worth taking. What the
    // waiting bought in payout, it charges back at every door (GDD 5.4).
    let ripe = session
        .board_entry(&target.id)
        .map(|entry| entry.door_penalty(&data.config.board))
        .unwrap_or(0);
    if ripe > 0 {
        extras.push(ModifierEntry::new("Mark has ripened", -ripe));
    }

    // Tools that have been through too many doors without a bench. Named like
    // everything else, so the shop bill is visible in the dice (GDD 3, 9).
    if let Some(member) = session.member(member_id) {
        let worn = super::kit::wear_penalty(session, member, &data.config.kit);
        if worn > 0 {
            extras.push(ModifierEntry::new("Worn kit", -worn));
        }
    }

    // The city's file on how this outfit works. Charged per trade rather than
    // per mark: a crew who keep going through the wires find every set of wires
    // in the city harder, whichever building they are in (GDD 5.4).
    if let Some(entry) = session
        .scrutiny
        .entry(encounter.primary_skill, &data.config.scrutiny)
    {
        extras.push(entry);
    }

    // A tail is heat the crew can see out of the window, and it costs the same
    // on every door until it gets bored (GDD 5.6).
    if session.surveillance_weeks > 0 {
        extras.push(ModifierEntry::new(
            "Under surveillance",
            -data.config.law.surveillance_penalty,
        ));
    }

    if let Some(entry) = session.chemistry.modifier(member_id, crew_on_job) {
        extras.push(entry);
    }
    extras
}

/// One crew member considered for one door.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub member_id: String,
    pub member_name: String,
    pub specialty: String,
    pub check: CheckBreakdown,
    /// True when this hand is already down for another door on this job.
    pub doubled_up: bool,
    /// True when injuries have put them out of the running entirely.
    pub unfit: bool,
    /// True when they are past the working threshold: still selectable, and
    /// worse on the die and likelier to come back hurt for it (GDD 5.6).
    pub spent: bool,
    /// Somebody already on this job will not stand beside them (GDD 5.5).
    pub refused_by: Vec<String>,
}

impl Candidate {
    /// Can this hand be put on the door at all? A spent hand can — that is the
    /// decision, and taking it away from the fixer is what made resting free.
    pub fn selectable(&self) -> bool {
        !self.unfit && self.refused_by.is_empty()
    }

    /// What the planning screen has to say about them before anybody commits.
    /// The die tells the player about the check; nothing on the breakdown can
    /// tell them somebody is likelier to come back hurt, so this does (pillar 2).
    pub fn warning(&self) -> Option<&'static str> {
        if self.unfit {
            Some("too hurt to work")
        } else if self.spent {
            Some("spent — worse odds, and gets hurt easier")
        } else {
            None
        }
    }
}

impl Candidate {
    pub fn success_chance(&self) -> f32 {
        self.check.success_chance()
    }
}

/// The check one member would make on one door, computed without rolling.
/// `crew_on_job` is everybody else down for this job, which is what chemistry
/// reads.
pub fn candidate_check(
    session: &GameSession,
    data: &GameData,
    target: &HeistTarget,
    encounter: &Encounter,
    member: &CrewMember,
    crew_on_job: &[String],
) -> CheckBreakdown {
    let loadout = session.loadout(member, data);
    build_check(CheckInputs {
        member,
        loadout: &loadout,
        encounter,
        tuning: &data.config.condition,
        extra: &situational_modifiers(data, target, encounter, session, &member.id, crew_on_job),
    })
}

/// Every hand on the payroll, ranked for one door, best first. Unfit crew are
/// listed last rather than hidden — a player should see who they cannot use and
/// why (GDD 2, pillar 2).
pub fn candidates(
    session: &GameSession,
    data: &GameData,
    target: &HeistTarget,
    encounter: &Encounter,
    draft: &PlanDraft,
) -> Vec<Candidate> {
    let crew_on_job = draft.crew_on_job();
    let mut candidates: Vec<Candidate> = session
        .crew
        .iter()
        .map(|member| Candidate {
            member_id: member.id.clone(),
            member_name: member.name.clone(),
            specialty: member.specialty.clone(),
            check: candidate_check(session, data, target, encounter, member, &crew_on_job),
            doubled_up: draft.assigned_elsewhere(&member.id, encounter),
            unfit: !data.config.condition.can_work(&member.condition),
            spent: data.config.condition.is_spent(member.condition.fatigue),
            refused_by: session
                .chemistry
                .refusals(&member.id, crew_on_job.iter().map(|id| id.as_str()))
                .into_iter()
                .map(|id| {
                    session
                        .member(&id)
                        .map(|other| other.name.clone())
                        .unwrap_or(id)
                })
                .collect(),
        })
        .collect();

    // Spent hands sort below the crew who are fit for it and above the ones who
    // cannot go at all — they are a last resort, listed as one, not hidden.
    candidates.sort_by(|a, b| {
        a.selectable()
            .cmp(&b.selectable())
            .reverse()
            .then(a.spent.cmp(&b.spent))
            .then(b.check.bonus().cmp(&a.check.bonus()))
            .then(a.member_name.cmp(&b.member_name))
    });
    candidates
}

/// A plan under construction: one slot per door, filled in any order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanDraft {
    pub target_id: String,
    /// Encounter ids in the order the crew will meet them.
    pub doors: Vec<String>,
    pub assignments: Vec<Option<String>>,
    /// The door whose candidate list is open.
    pub focus: usize,
    /// True while every filled door is the crew's own pick and the fixer has
    /// not argued with any of it. Letting them fill the board and committing it
    /// unchanged *is* delegation, whichever button started it — otherwise the
    /// label is a formality the player can step around (GDD 5.3).
    pub crew_planned: bool,
    /// The standing order: how many doors may go wrong before the crew leave.
    /// `None` is "push on", and it is the default because it is what the game
    /// did before there was an order to give.
    pub walk_after: Option<u32>,
}

impl PlanDraft {
    pub fn new(target: &HeistTarget, data: &GameData) -> Self {
        let doors: Vec<String> = data
            .encounters_for(target)
            .into_iter()
            .map(|encounter| encounter.id.clone())
            .collect();
        let assignments = vec![None; doors.len()];

        Self {
            target_id: target.id.clone(),
            doors,
            assignments,
            focus: 0,
            crew_planned: false,
            walk_after: None,
        }
    }

    /// Step the standing order round: push on, then leave after one door goes
    /// wrong, then after two, and back. Cycling rather than a slider because
    /// there are only ever a few useful answers and a mark with three doors
    /// makes most of the range meaningless.
    pub fn cycle_nerve(&mut self) {
        let most = self.doors.len().max(1) as u32;
        self.walk_after = match self.walk_after {
            None => Some(1),
            Some(limit) if limit + 1 < most => Some(limit + 1),
            Some(_) => None,
        };
        // Telling the crew when to leave is the fixer turning up, so the plan
        // stops being theirs. Otherwise "let them pick, then give them an
        // order" would take the delegation discount on the cut *and* the nerve
        // — which is the same dodge the crew_planned flag exists to close
        // (GDD 5.3: the label follows the work, not the button).
        self.crew_planned = false;
    }

    /// The standing order, short enough to sit on a quarter-width button beside
    /// Commit. The results screen carries the long version afterwards; what this
    /// has to do is fit and be unmistakable at a glance.
    pub fn nerve_label(&self) -> String {
        match self.walk_after {
            None => "Push on regardless".to_owned(),
            Some(1) => "Walk after 1 fail".to_owned(),
            Some(limit) => format!("Walk after {} fails", limit),
        }
    }

    /// Start from the crew's own best guess, which the player can then argue
    /// with. This is the same greedy fit delegation uses.
    pub fn from_auto(session: &GameSession, data: &GameData, target: &HeistTarget) -> Self {
        let mut draft = Self::new(target, data);
        let auto = super::job::auto_assign(session, data, target);

        for assignment in &auto.assignments {
            if let Some(index) = draft
                .doors
                .iter()
                .position(|door| *door == assignment.encounter_id)
            {
                if draft.assignments[index].is_none() {
                    draft.assignments[index] = Some(assignment.member_id.clone());
                }
            }
        }
        draft.crew_planned = true;
        draft
    }

    /// Putting somebody on a door makes the plan the fixer's, not the crew's.
    pub fn assign(&mut self, door: usize, member_id: impl Into<String>) {
        if door < self.assignments.len() {
            self.assignments[door] = Some(member_id.into());
            self.crew_planned = false;
        }
    }

    pub fn clear(&mut self, door: usize) {
        if door < self.assignments.len() {
            self.assignments[door] = None;
            self.crew_planned = false;
        }
    }

    pub fn focus_on(&mut self, door: usize) {
        if door < self.doors.len() {
            self.focus = door;
        }
    }

    pub fn assigned(&self, door: usize) -> Option<&str> {
        self.assignments.get(door).and_then(|id| id.as_deref())
    }

    /// Everybody currently down for this job, each named once.
    pub fn crew_on_job(&self) -> Vec<String> {
        let mut crew: Vec<String> = self.assignments.iter().flatten().cloned().collect();
        crew.sort();
        crew.dedup();
        crew
    }

    pub fn focused_door(&self) -> Option<&str> {
        self.doors.get(self.focus).map(|id| id.as_str())
    }

    /// Is this hand already down for a *different* door than the one asked about?
    fn assigned_elsewhere(&self, member_id: &str, encounter: &Encounter) -> bool {
        self.doors
            .iter()
            .zip(&self.assignments)
            .any(|(door, assigned)| door != &encounter.id && assigned.as_deref() == Some(member_id))
    }

    pub fn unfilled(&self) -> usize {
        self.assignments
            .iter()
            .filter(|slot| slot.is_none())
            .count()
    }

    pub fn is_complete(&self) -> bool {
        !self.doors.is_empty() && self.unfilled() == 0
    }

    /// Turn a complete draft into something the run can execute.
    pub fn to_job_plan(&self) -> Option<JobPlan> {
        if !self.is_complete() {
            return None;
        }

        Some(JobPlan {
            target_id: self.target_id.clone(),
            assignments: self
                .doors
                .iter()
                .zip(&self.assignments)
                .filter_map(|(door, member)| {
                    member.as_ref().map(|member_id| Assignment {
                        encounter_id: door.clone(),
                        member_id: member_id.clone(),
                    })
                })
                .collect(),
            delegated: self.crew_planned,
            walk_after: self.walk_after,
        })
    }
}

#[cfg(test)]
mod tests;
