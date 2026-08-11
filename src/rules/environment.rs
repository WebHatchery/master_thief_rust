//! Environmental modifiers — time of day, weather, and site quirks.
//!
//! The original rolled `Math.random() > 0.5` for day-or-night *inside* the
//! resolution loop. Here the environment is a property of the job, fixed at
//! planning time and visible before the player commits (GDD 5.2).

use super::outcome::ModifierEntry;
use crate::model::{Environment, EnvironmentModifier, Skill};

/// One named entry per environment id that actually moves the check, in the
/// fixed order time-of-day, weather, then site factors.
pub fn environment_entries<'a>(
    environment: &Environment,
    skill: Skill,
    lookup: impl Fn(&str) -> Option<&'a EnvironmentModifier>,
) -> Vec<ModifierEntry> {
    environment
        .modifier_ids()
        .filter_map(lookup)
        .filter_map(|modifier| {
            let value = modifier.value_for(skill);
            (value != 0).then(|| ModifierEntry::new(modifier.name.clone(), value))
        })
        .collect()
}

/// The net environmental swing on one skill.
pub fn environment_total<'a>(
    environment: &Environment,
    skill: Skill,
    lookup: impl Fn(&str) -> Option<&'a EnvironmentModifier>,
) -> i32 {
    environment_entries(environment, skill, lookup)
        .iter()
        .map(|entry| entry.value)
        .sum()
}

/// Every environment id that failed to resolve. Content tests use this to catch
/// a target naming a factor nobody authored.
pub fn unknown_ids<'a>(
    environment: &Environment,
    lookup: impl Fn(&str) -> Option<&'a EnvironmentModifier>,
) -> Vec<String> {
    environment
        .modifier_ids()
        .filter(|id| lookup(id).is_none())
        .map(|id| id.to_owned())
        .collect()
}

#[cfg(test)]
mod tests;
