//! The d20 core. One door, one specialist, one roll, and every modifier named.

use super::attributes::{
    attribute_modifier, effective_skills, equipped_attributes, proficiency_bonus,
};
use super::condition::{condition_entries, ConditionTuning};
use super::outcome::{ModifierEntry, Outcome};
use crate::model::{CrewMember, Encounter, EquipmentSlot, Loadout, RunEffect, Skill};
use serde::{Deserialize, Serialize};

/// Everything a check needs. `extra` carries modifiers the rules engine does
/// not own — environment, crew chemistry, city-wide heat — already named by
/// whoever computed them.
pub struct CheckInputs<'a> {
    pub member: &'a CrewMember,
    pub loadout: &'a Loadout<'a>,
    pub encounter: &'a Encounter,
    pub tuning: &'a ConditionTuning,
    pub extra: &'a [ModifierEntry],
}

/// A check, fully computed but not yet rolled. This is what the planning screen
/// shows before the player commits, and what the results screen shows after.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckBreakdown {
    pub member_id: String,
    pub member_name: String,
    pub encounter_id: String,
    pub skill: Skill,
    pub dc: i32,
    pub entries: Vec<ModifierEntry>,
}

impl CheckBreakdown {
    /// Everything added to the d20.
    pub fn bonus(&self) -> i32 {
        self.entries.iter().map(|entry| entry.value).sum()
    }

    /// Entries worth drawing. The skill total always shows, even at zero — a
    /// hand with no training in the trade is exactly what a player needs to see
    /// before they put them on the door.
    pub fn significant(&self) -> impl Iterator<Item = &ModifierEntry> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(index, entry)| *index == 0 || entry.is_significant())
            .map(|(_, entry)| entry)
    }

    /// The lowest natural roll that still gets through the door. Natural 1
    /// always fails, so it never reports below 2.
    pub fn roll_needed(&self) -> i32 {
        (self.dc - self.bonus()).clamp(2, 21)
    }

    /// Probability of at least scraping through, given the nat-1 / nat-20 rules.
    pub fn success_chance(&self) -> f32 {
        let needed = self.roll_needed();
        // Rolls 2..=19 must meet the target; 20 always succeeds; 1 always fails.
        let passing = (2..=19).filter(|roll| *roll >= needed).count() + 1;
        passing as f32 / 20.0
    }

    /// Probability of the two failure bands.
    pub fn failure_chance(&self) -> f32 {
        1.0 - self.success_chance()
    }
}

/// The outcome of one door, with the arithmetic that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterResult {
    pub check: CheckBreakdown,
    pub roll: i32,
    pub total: i32,
    pub outcome: Outcome,
    pub experience_gained: i32,
    pub stress_inflicted: i32,
    pub run_effect: RunEffect,
}

impl EncounterResult {
    pub fn passed(&self) -> bool {
        self.outcome.passed()
    }

    /// The margin by which the door was cleared or missed.
    pub fn margin(&self) -> i32 {
        self.total - self.check.dc
    }
}

/// Build a check without rolling it. Deterministic and side-effect free, so the
/// planning screen and the run can share one code path.
pub fn build_check(inputs: CheckInputs<'_>) -> CheckBreakdown {
    let CheckInputs {
        member,
        loadout,
        encounter,
        tuning,
        extra,
    } = inputs;

    let skill = encounter.primary_skill;
    let attributes = equipped_attributes(member, loadout);
    let skills = effective_skills(
        &attributes,
        &member.training,
        &member.progression,
        member.specialty_skill,
    );

    let mut entries = vec![ModifierEntry::new(
        format!("{} skill", skill.label()),
        skills.get(skill),
    )];

    if let Some(entry) = focus_attribute_entry(encounter, &attributes) {
        entries.push(entry);
    }

    let kit_bonus = loadout.skill_bonus(skill)
        + loadout
            .iter()
            .map(|(slot, _)| encounter.equipment_bonuses.get(slot))
            .sum::<i32>();
    entries.push(ModifierEntry::new("Equipment", kit_bonus));

    entries.push(ModifierEntry::new(
        "Proficiency",
        proficiency_bonus(member.progression.level),
    ));

    entries.extend(condition_entries(member, tuning));
    entries.extend(extra.iter().cloned());

    CheckBreakdown {
        member_id: member.id.clone(),
        member_name: member.name.clone(),
        encounter_id: encounter.id.clone(),
        skill,
        dc: encounter.difficulty,
        entries,
    }
}

/// An encounter that names a primary attribute leans on it *extra*. Where none
/// is named there is no attribute line at all: the skill total already folds in
/// its own attribute pair (GDD 5.1), and the original engine's habit of adding
/// the pair a second time made every door roughly five points easier than its
/// DC claimed — which the distribution soak caught immediately.
fn focus_attribute_entry(
    encounter: &Encounter,
    attributes: &crate::model::Attributes,
) -> Option<ModifierEntry> {
    let kind = encounter.primary_attribute?;
    Some(ModifierEntry::new(
        format!("{} focus", kind.short_label()),
        attribute_modifier(attributes.get(kind)),
    ))
}

/// Apply a d20 to a prepared check. The roll comes from the run's seeded RNG;
/// nothing here draws its own randomness.
pub fn resolve(check: CheckBreakdown, roll: i32) -> EncounterResult {
    resolve_with_effects(check, roll, RunEffect::None, RunEffect::None)
}

/// Resolve, carrying the encounter's critical run effects through to the result.
pub fn resolve_with_effects(
    check: CheckBreakdown,
    roll: i32,
    on_critical_success: RunEffect,
    on_critical_failure: RunEffect,
) -> EncounterResult {
    let total = roll + check.bonus();
    let outcome = Outcome::classify(roll, total, check.dc);
    let run_effect = match outcome {
        Outcome::CriticalSuccess => on_critical_success,
        Outcome::CriticalFailure => on_critical_failure,
        _ => RunEffect::None,
    };

    EncounterResult {
        experience_gained: outcome.experience_for(check.dc),
        stress_inflicted: outcome.stress_for(check.dc),
        total,
        outcome,
        run_effect,
        roll,
        check,
    }
}

/// The equipment slots an encounter rewards bringing, for the outfit screen.
pub fn rewarded_slots(encounter: &Encounter) -> Vec<(EquipmentSlot, i32)> {
    EquipmentSlot::ALL
        .into_iter()
        .map(|slot| (slot, encounter.equipment_bonuses.get(slot)))
        .filter(|(_, value)| *value != 0)
        .collect()
}

#[cfg(test)]
mod tests;
