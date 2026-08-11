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
            .available_crew(&data.config.condition)
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
mod tests;
