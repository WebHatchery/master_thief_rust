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
    /// True when fatigue or injury has put them out of the running entirely.
    pub unfit: bool,
    /// Somebody already on this job will not stand beside them (GDD 5.5).
    pub refused_by: Vec<String>,
}

impl Candidate {
    /// Can this hand be put on the door at all?
    pub fn selectable(&self) -> bool {
        !self.unfit && self.refused_by.is_empty()
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
            unfit: !member.condition.is_fit_for_work(),
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

    candidates.sort_by(|a, b| {
        a.selectable()
            .cmp(&b.selectable())
            .reverse()
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(seed: u64) -> (GameData, GameSession, HeistTarget) {
        let data = GameData::load().unwrap();
        let session = GameSession::new(&data.config, &data, seed);
        let target = data
            .targets
            .get(&session.board[0].target_id)
            .unwrap()
            .clone();
        (data, session, target)
    }

    #[test]
    fn a_fresh_draft_has_one_empty_slot_per_door() {
        let (data, _, target) = setup(11);
        let draft = PlanDraft::new(&target, &data);

        assert_eq!(draft.doors.len(), target.encounters.len());
        assert_eq!(draft.unfilled(), target.encounters.len());
        assert!(!draft.is_complete());
        assert!(draft.to_job_plan().is_none());
    }

    #[test]
    fn filling_every_door_makes_the_plan_committable() {
        let (data, session, target) = setup(12);
        let mut draft = PlanDraft::new(&target, &data);
        let hand = session.crew[0].id.clone();

        for door in 0..draft.doors.len() {
            draft.assign(door, hand.clone());
        }

        assert!(draft.is_complete());
        let plan = draft.to_job_plan().unwrap();
        assert_eq!(plan.assignments.len(), draft.doors.len());
        assert!(!plan.delegated, "a hand-made plan is never delegated");
    }

    #[test]
    fn clearing_a_door_takes_the_plan_back_out_of_reach() {
        let (data, session, target) = setup(13);
        let mut draft = PlanDraft::from_auto(&session, &data, &target);
        assert!(draft.is_complete());

        draft.clear(1);
        assert!(!draft.is_complete());
        assert_eq!(draft.unfilled(), 1);
    }

    #[test]
    fn the_auto_draft_fills_the_same_doors_delegation_would() {
        let (data, session, target) = setup(14);
        let draft = PlanDraft::from_auto(&session, &data, &target);
        let auto = super::super::job::auto_assign(&session, &data, &target);

        assert!(draft.is_complete());
        for assignment in &auto.assignments {
            let index = draft
                .doors
                .iter()
                .position(|door| *door == assignment.encounter_id)
                .unwrap();
            assert_eq!(draft.assigned(index), Some(assignment.member_id.as_str()));
        }
    }

    #[test]
    fn candidates_are_ranked_best_first_and_name_every_modifier() {
        let (data, session, target) = setup(15);
        let draft = PlanDraft::new(&target, &data);
        let encounter = data.encounters.get(&draft.doors[0]).unwrap();

        let ranked = candidates(&session, &data, &target, encounter, &draft);
        assert_eq!(ranked.len(), session.crew.len());
        for pair in ranked.windows(2) {
            assert!(pair[0].check.bonus() >= pair[1].check.bonus());
        }
        assert!(ranked[0].check.entries.iter().any(|e| e.value != 0));
    }

    #[test]
    fn a_hand_already_down_for_another_door_is_flagged_not_hidden() {
        let (data, session, target) = setup(16);
        let mut draft = PlanDraft::new(&target, &data);
        let hand = session.crew[0].id.clone();
        draft.assign(0, hand.clone());

        let second = data.encounters.get(&draft.doors[1]).unwrap();
        let ranked = candidates(&session, &data, &target, second, &draft);
        let entry = ranked.iter().find(|c| c.member_id == hand).unwrap();
        assert!(entry.doubled_up);

        let first = data.encounters.get(&draft.doors[0]).unwrap();
        let same_door = candidates(&session, &data, &target, first, &draft);
        let entry = same_door.iter().find(|c| c.member_id == hand).unwrap();
        assert!(
            !entry.doubled_up,
            "their own door does not count as doubling"
        );
    }

    #[test]
    fn unfit_crew_sink_to_the_bottom_of_the_list() {
        let (data, mut session, target) = setup(17);
        session.crew[0].condition.fatigue = 95;
        let draft = PlanDraft::new(&target, &data);
        let encounter = data.encounters.get(&draft.doors[0]).unwrap();

        let ranked = candidates(&session, &data, &target, encounter, &draft);
        assert!(ranked.last().unwrap().unfit);
        assert!(!ranked[0].unfit);
    }

    #[test]
    fn a_hand_made_plan_can_never_be_beaten_by_delegation() {
        // GDD 5.3: delegation is a discount, not a shortcut. Picking the best
        // candidate for every door must total at least what the greedy
        // auto-assigner reaches, on every mark the crew can currently open.
        let data = GameData::load().unwrap();
        let session = GameSession::new(&data.config, &data, 21);

        for target in session.eligible_targets(&data) {
            let draft = PlanDraft::new(target, &data);
            let best: i32 = draft
                .doors
                .iter()
                .filter_map(|door| data.encounters.get(door))
                .map(|encounter| {
                    candidates(&session, &data, target, encounter, &draft)
                        .first()
                        .map(|candidate| candidate.check.bonus())
                        .unwrap_or(0)
                })
                .sum();

            let delegated: i32 = super::super::job::auto_assign(&session, &data, target)
                .assignments
                .iter()
                .filter_map(|assignment| {
                    let encounter = data.encounters.get(&assignment.encounter_id)?;
                    let member = session.member(&assignment.member_id)?;
                    Some(candidate_check(&session, &data, target, encounter, member, &[]).bonus())
                })
                .sum();

            assert!(
                best >= delegated,
                "{}: delegation reached {} where a hand-made plan reaches {}",
                target.id,
                delegated,
                best
            );
        }
    }

    #[test]
    fn letting_the_crew_pick_and_committing_it_is_delegation() {
        // The dodge this closes: "Let them pick" produced exactly the
        // assignment delegation produces, and committing it recorded a
        // hand-made plan. The label has to follow the work, not the button.
        let (data, session, target) = setup(31);
        let draft = PlanDraft::from_auto(&session, &data, &target);

        assert!(draft.crew_planned);
        assert!(
            draft.to_job_plan().expect("a full draft commits").delegated,
            "the crew's own plan committed as the fixer's"
        );
    }

    #[test]
    fn arguing_with_one_door_makes_the_plan_yours() {
        let (data, session, target) = setup(32);
        let mut draft = PlanDraft::from_auto(&session, &data, &target);
        let hand = session.crew[0].id.clone();

        draft.assign(0, hand);
        assert!(!draft.crew_planned);
        assert!(!draft.to_job_plan().unwrap().delegated);
    }

    #[test]
    fn a_draft_built_by_hand_was_never_theirs() {
        let (data, session, target) = setup(33);
        let mut draft = PlanDraft::new(&target, &data);
        let hand = session.crew[0].id.clone();

        assert!(!draft.crew_planned);
        for door in 0..draft.doors.len() {
            draft.assign(door, hand.clone());
        }
        assert!(!draft.to_job_plan().unwrap().delegated);
    }

    #[test]
    fn a_ripened_mark_charges_for_itself_on_the_planning_screen() {
        // The payout the board advertises and the doors the crew will meet have
        // to move together, and both before commit (pillar 2).
        let (data, mut session, target) = setup(23);
        let draft = PlanDraft::new(&target, &data);
        let encounter = data.encounters.get(&draft.doors[0]).unwrap();

        let fresh = candidate_check(&session, &data, &target, encounter, &session.crew[0], &[]);
        assert!(!fresh
            .entries
            .iter()
            .any(|entry| entry.label == "Mark has ripened"));

        if let Some(entry) = session
            .board
            .iter_mut()
            .find(|entry| entry.target_id == target.id)
        {
            entry.ripeness = 2;
        }
        let ripe = candidate_check(&session, &data, &target, encounter, &session.crew[0], &[]);

        let named = ripe
            .entries
            .iter()
            .find(|entry| entry.label == "Mark has ripened")
            .expect("the waiting is charged by name");
        assert_eq!(named.value, -2 * data.config.board.ripeness_door_penalty);
        assert!(ripe.bonus() < fresh.bonus());
    }

    #[test]
    fn a_tail_is_named_on_the_planning_screen_before_anybody_commits() {
        // Pillar 2: no hidden difficulty, ever. A penalty the city applied last
        // week has to be readable on the breakdown this week.
        let (data, mut session, target) = setup(19);
        let draft = PlanDraft::new(&target, &data);
        let encounter = data.encounters.get(&draft.doors[0]).unwrap();
        let member = &session.crew[0];

        let clear = candidate_check(&session, &data, &target, encounter, member, &[]);
        assert!(!clear
            .entries
            .iter()
            .any(|entry| entry.label == "Under surveillance"));

        session.surveillance_weeks = 2;
        let member = &session.crew[0];
        let tailed = candidate_check(&session, &data, &target, encounter, member, &[]);

        let entry = tailed
            .entries
            .iter()
            .find(|entry| entry.label == "Under surveillance")
            .expect("a tail shows up by name");
        assert_eq!(entry.value, -data.config.law.surveillance_penalty);
        assert_eq!(
            clear.bonus() - tailed.bonus(),
            data.config.law.surveillance_penalty
        );
    }

    #[test]
    fn the_planning_screen_and_the_run_agree_on_the_arithmetic() {
        let (data, session, target) = setup(18);
        let draft = PlanDraft::from_auto(&session, &data, &target);
        let encounter = data.encounters.get(&draft.doors[0]).unwrap();
        let member = session.member(draft.assigned(0).unwrap()).unwrap();

        let crew = draft.crew_on_job();
        let planned = candidate_check(&session, &data, &target, encounter, member, &crew);
        let at_run_time = candidate_check(&session, &data, &target, encounter, member, &crew);
        assert_eq!(planned, at_run_time);
    }
}
