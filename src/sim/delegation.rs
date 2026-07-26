//! What delegation cost you.
//!
//! GDD 5.3: delegation is a discount, not a shortcut. The results screen has to
//! say plainly where the crew's own choice differed from the best one available,
//! because that is what teaches the planning screen — a player who never sees
//! the gap has no reason to believe the screen is worth their time.

use crate::data::GameData;
use crate::model::HeistTarget;
use crate::state::GameSession;

use super::job::JobPlan;
use super::plan::candidate_check;

/// One door where a better hand was standing right there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegationMiss {
    pub encounter_name: String,
    pub chosen: String,
    pub chosen_bonus: i32,
    pub better: String,
    pub better_bonus: i32,
}

impl DelegationMiss {
    pub fn gap(&self) -> i32 {
        self.better_bonus - self.chosen_bonus
    }
}

/// Compare a plan against the best assignment available for each door. Called
/// before the job runs, while the crew is still in the state the plan assumed.
pub fn audit(
    session: &GameSession,
    data: &GameData,
    target: &HeistTarget,
    plan: &JobPlan,
) -> Vec<DelegationMiss> {
    let crew_on_job = plan.crew_on_job();
    let mut misses = Vec::new();

    for assignment in &plan.assignments {
        let Some(encounter) = data.encounters.get(&assignment.encounter_id) else {
            continue;
        };
        let Some(chosen) = session.member(&assignment.member_id) else {
            continue;
        };

        let chosen_bonus =
            candidate_check(session, data, target, encounter, chosen, &crew_on_job).bonus();

        // The comparison ignores the reuse penalty the auto-assigner applies to
        // itself: the question is who *could* have taken this door, not who the
        // greedy pass was willing to spend.
        let best = session
            .available_crew()
            .filter(|member| {
                !crew_on_job
                    .iter()
                    .any(|other| session.chemistry.refuses(&member.id, other))
            })
            .map(|member| {
                (
                    member,
                    candidate_check(session, data, target, encounter, member, &crew_on_job).bonus(),
                )
            })
            .max_by_key(|(member, bonus)| (*bonus, std::cmp::Reverse(member.id.clone())));

        if let Some((better, better_bonus)) = best {
            if better_bonus > chosen_bonus && better.id != chosen.id {
                misses.push(DelegationMiss {
                    encounter_name: encounter.name.clone(),
                    chosen: chosen.name.clone(),
                    chosen_bonus,
                    better: better.name.clone(),
                    better_bonus,
                });
            }
        }
    }

    misses
}

/// A one-line summary for the results ledger.
pub fn summarise(misses: &[DelegationMiss]) -> String {
    if misses.is_empty() {
        return "The crew picked the best hand for every door — a plan would not have beaten it."
            .to_owned();
    }

    let worst = misses.iter().map(|miss| miss.gap()).max().unwrap_or(0);
    format!(
        "The crew settled on {} door{} a better hand was free for — up to {} points left on the table.",
        misses.len(),
        if misses.len() == 1 { "" } else { "s" },
        worst
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{auto_assign, plan::PlanDraft};

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
    fn a_plan_that_takes_the_best_hand_everywhere_has_nothing_to_report() {
        let (data, session, target) = setup(51);
        let mut draft = PlanDraft::new(&target, &data);

        for door in 0..draft.doors.len() {
            let encounter = data.encounters.get(&draft.doors[door]).unwrap();
            let best = session
                .available_crew()
                .max_by_key(|member| {
                    candidate_check(&session, &data, &target, encounter, member, &[]).bonus()
                })
                .unwrap()
                .id
                .clone();
            draft.assign(door, best);
        }

        let plan = draft.to_job_plan().unwrap();
        assert!(audit(&session, &data, &target, &plan).is_empty());
    }

    #[test]
    fn putting_the_wrong_hand_on_a_door_is_reported_with_the_gap() {
        let (data, session, target) = setup(52);
        let mut draft = PlanDraft::new(&target, &data);

        // Deliberately put the worst available hand on every door.
        for door in 0..draft.doors.len() {
            let encounter = data.encounters.get(&draft.doors[door]).unwrap();
            let worst = session
                .available_crew()
                .min_by_key(|member| {
                    candidate_check(&session, &data, &target, encounter, member, &[]).bonus()
                })
                .unwrap()
                .id
                .clone();
            draft.assign(door, worst);
        }

        let plan = draft.to_job_plan().unwrap();
        let misses = audit(&session, &data, &target, &plan);

        assert!(
            !misses.is_empty(),
            "a deliberately bad plan reported nothing"
        );
        for miss in &misses {
            assert!(miss.gap() > 0);
            assert_ne!(miss.chosen, miss.better);
        }
    }

    #[test]
    fn delegation_reports_only_the_doors_it_actually_gave_away() {
        let (data, session, target) = setup(53);
        let plan = auto_assign(&session, &data, &target);
        let misses = audit(&session, &data, &target, &plan);

        assert!(
            misses.len() < plan.assignments.len(),
            "the auto-assigner cannot be wrong about every single door"
        );
    }

    #[test]
    fn the_summary_says_something_useful_either_way() {
        let quiet = summarise(&[]);
        assert!(quiet.contains("best hand"));

        let noisy = summarise(&[DelegationMiss {
            encounter_name: "The Bouncer".to_owned(),
            chosen: "Otis Kemp".to_owned(),
            chosen_bonus: 7,
            better: "Birdie Lang".to_owned(),
            better_bonus: 14,
        }]);
        assert!(noisy.contains("1 door "), "{}", noisy);
        assert!(noisy.contains('7'), "the gap should be named: {}", noisy);
    }

    #[test]
    fn the_audit_never_recommends_somebody_who_would_refuse_to_be_there() {
        let (data, mut session, target) = setup(54);
        let hands: Vec<String> = session.crew_ids();
        session
            .chemistry
            .set(&hands[0], &hands[1], crate::rules::chemistry::REFUSAL);

        let plan = auto_assign(&session, &data, &target);
        let crew = plan.crew_on_job();
        for miss in audit(&session, &data, &target, &plan) {
            let suggested = session
                .crew
                .iter()
                .find(|member| member.name == miss.better)
                .unwrap();
            assert!(
                !crew
                    .iter()
                    .any(|other| session.chemistry.refuses(&suggested.id, other)),
                "suggested a hand who refuses the company"
            );
        }
    }
}
