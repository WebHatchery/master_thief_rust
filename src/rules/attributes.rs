//! Attribute modifiers, derived stats, skill totals, and the level curve.
//!
//! Ported from the original `characterCalculations.ts`, with the stats nothing
//! consumed pruned away (GDD 0, mechanic carry-over table).

use crate::model::{Attributes, CrewMember, DerivedStats, Loadout, Progression, Skill, Skills};
use serde::{Deserialize, Serialize};

/// The standard `(score - 10) / 2`, rounded *down* — including for negatives,
/// which is why this uses Euclidean division rather than Rust's truncation.
pub fn attribute_modifier(score: i32) -> i32 {
    (score - 10).div_euclid(2)
}

/// Health and initiative. The original also computed stamina, carrying capacity
/// and a critical chance/multiplier that no rule ever read; GDD 0 says to keep
/// only what a rule consumes, and stamina turned out to be consumed by nothing
/// but its own assertion.
pub fn derived_stats(attributes: &Attributes, level: i32) -> DerivedStats {
    let con = attribute_modifier(attributes.constitution);
    let dex = attribute_modifier(attributes.dexterity);
    let wis = attribute_modifier(attributes.wisdom);

    DerivedStats {
        health: 10 + con + level * (2 + con),
        initiative: dex + wis,
    }
}

/// Skill totals: training ranks, plus the skill's attribute pair, plus half the
/// member's level, plus mastery on their own specialty. Never negative.
pub fn effective_skills(
    attributes: &Attributes,
    training: &Skills,
    progression: &Progression,
    specialty_skill: Skill,
) -> Skills {
    let level_bonus = progression.level / 2;
    let mut skills = Skills::default();

    for skill in Skill::ALL {
        let (first, second) = skill.attribute_pair();
        let pair_bonus =
            attribute_modifier(attributes.get(first)) + attribute_modifier(attributes.get(second));
        let mastery_bonus = if skill == specialty_skill {
            progression.mastery_level
        } else {
            0
        };
        let total = training.get(skill) + pair_bonus + level_bonus + mastery_bonus;
        skills.set(skill, total.max(0));
    }

    skills
}

/// The attributes a member fights with once their kit is accounted for.
pub fn equipped_attributes(member: &CrewMember, loadout: &Loadout<'_>) -> Attributes {
    let mut attributes = member.attributes;
    loadout.attribute_bonuses().apply_to(&mut attributes);
    attributes
}

/// A member's skill totals with equipment attribute *and* skill bonuses folded
/// in — what the crew screen shows.
pub fn equipped_skills(member: &CrewMember, loadout: &Loadout<'_>) -> Skills {
    let attributes = equipped_attributes(member, loadout);
    let mut skills = effective_skills(
        &attributes,
        &member.training,
        &member.progression,
        member.specialty_skill,
    );
    for skill in Skill::ALL {
        skills.add(skill, loadout.skill_bonus(skill));
    }
    skills
}

/// Experience needed to clear a given level: `level^2 * 100`.
pub fn experience_to_next(level: i32) -> i32 {
    level * level * 100
}

/// Cumulative experience spent reaching a level from level 1.
pub fn total_experience(level: i32) -> i32 {
    (1..level).map(experience_to_next).sum()
}

/// The D&D-style proficiency bonus every check gets for free.
pub fn proficiency_bonus(level: i32) -> i32 {
    (level - 1) / 4 + 2
}

/// How long it takes to get good at the one thing you are for. Authored in
/// `game_config.json`: the shape is a rule, the numbers are balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasteryTuning {
    /// Doors of your own trade needed for the first rank.
    pub doors_for_first: u32,
    /// Added to that requirement at every rank after.
    pub doors_step: u32,
    pub max_level: i32,
}

impl Default for MasteryTuning {
    fn default() -> Self {
        Self {
            doors_for_first: 3,
            doors_step: 2,
            max_level: 10,
        }
    }
}

impl MasteryTuning {
    /// Doors still owed before the next rank, at the rank they hold now.
    pub fn doors_for_next(&self, level: i32) -> u32 {
        self.doors_for_first + self.doors_step * level.max(0) as u32
    }

    pub fn is_capped(&self, level: i32) -> bool {
        level >= self.max_level
    }
}

/// One door of the member's own trade, cleared. Mastery was declared on every
/// dossier, read by [`effective_skills`], and written by nothing at all — a
/// hand's specialisation could never actually deepen. It advances here, and
/// only here: you get good at a trade by practising it successfully, so a
/// failed door teaches nothing and somebody else's door teaches nothing.
///
/// Returns true when that door earned a rank, so the run can say so.
pub fn work_the_trade(progression: &mut Progression, tuning: &MasteryTuning) -> bool {
    if tuning.is_capped(progression.mastery_level) {
        return false;
    }

    progression.specialty_doors += 1;
    let needed = tuning.doors_for_next(progression.mastery_level);
    if progression.specialty_doors < needed {
        return false;
    }

    progression.specialty_doors -= needed;
    progression.mastery_level += 1;
    if tuning.is_capped(progression.mastery_level) {
        progression.specialty_doors = 0;
    }
    true
}

/// Award experience, levelling as many times as it covers. Returns how many
/// levels were gained.
pub fn award_experience(progression: &mut Progression, amount: i32) -> i32 {
    progression.experience += amount;
    let mut levels = 0;

    while progression.experience >= progression.experience_to_next {
        progression.experience -= progression.experience_to_next;
        progression.level += 1;
        progression.experience_to_next = experience_to_next(progression.level);
        progression.attribute_points += 1;
        progression.skill_points += 2;
        levels += 1;
    }

    levels
}

/// A rough power rating, used for sorting the roster and for balance checks.
pub fn power_level(member: &CrewMember, loadout: &Loadout<'_>) -> i32 {
    let attributes = equipped_attributes(member, loadout);
    let skills = equipped_skills(member, loadout);
    attributes.total() + skills.total() + (loadout.len() as i32) * 5 + member.progression.level * 2
}

#[cfg(test)]
mod tests;
