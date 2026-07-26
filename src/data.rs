//! The paperwork: every balance value, dossier, and narrative line, embedded.
//!
//! Nothing in here is hardcoded in Rust. Add a target, a tool, or an outcome
//! line by editing `assets/data/*.json`.

pub mod outcomes;

use crate::model::{CrewMember, Encounter, EnvironmentModifier, EquipmentDef, HeistTarget};
use macroquad_toolkit::data_loader::{load_embedded_json_labeled, DataRegistry};
use outcomes::OutcomeTables;
use serde::{Deserialize, Serialize};

const GAME_CONFIG_JSON: &str = include_str!("../assets/data/game_config.json");
const CHARACTERS_JSON: &str = include_str!("../assets/data/characters.json");
const EQUIPMENT_JSON: &str = include_str!("../assets/data/equipment.json");
const ENCOUNTERS_JSON: &str = include_str!("../assets/data/encounters.json");
const TARGETS_JSON: &str = include_str!("../assets/data/targets.json");
const ENVIRONMENT_JSON: &str = include_str!("../assets/data/environment.json");
const OUTCOMES_JSON: &str = include_str!("../assets/data/outcomes.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub game_name: String,
    pub display_name: String,
    pub save_slot: String,
    pub version: String,

    /// Cash the fixer opens the campaign with.
    pub starting_budget: i64,
    /// Crew ids on the payroll at week one.
    pub starting_crew: Vec<String>,
    /// Equipment ids already in the lockup at week one.
    #[serde(default)]
    pub starting_inventory: Vec<String>,

    /// Marks visible on the board at once.
    pub targets_on_board: usize,
    /// Candidates in the hiring pool each week.
    pub recruit_pool_size: usize,
    /// Cost, in cash, of casing one mark.
    pub casing_cost: i64,

    /// Fatigue removed by a week of rest, before the constitution bonus.
    pub rest_recovery: i32,
    /// Fatigue above which a member is unfit for work.
    pub fatigue_work_threshold: i32,
    /// Loyalty a week of idleness costs, and rest restores.
    pub idle_loyalty_drift: i32,

    /// Heat shed each week the crew stays quiet.
    pub heat_decay_per_week: i32,
    /// Heat below which the city is not watching.
    pub heat_safe_threshold: i32,
    /// Every this much heat above the threshold adds +1 to every DC.
    pub heat_dc_step: i32,
    /// Notoriety added by a failed job on top of the target's own.
    pub failure_notoriety: i32,

    /// Reputation earned per clean job, scaled by the mark's difficulty band.
    pub reputation_per_job: i32,
    /// Share of the payout the crew takes before the fixer sees any.
    pub crew_cut: f32,
}

#[derive(Debug, Clone)]
pub struct GameData {
    pub config: GameConfig,
    pub crew_pool: DataRegistry<CrewMember>,
    pub equipment: DataRegistry<EquipmentDef>,
    pub encounters: DataRegistry<Encounter>,
    pub targets: DataRegistry<HeistTarget>,
    pub environment: DataRegistry<EnvironmentModifier>,
    pub outcomes: OutcomeTables,
}

impl GameData {
    pub fn load() -> Result<Self, String> {
        let config = load_embedded_json_labeled("game_config", GAME_CONFIG_JSON)?;
        let crew_pool = registry("characters", CHARACTERS_JSON)?;
        let equipment = registry("equipment", EQUIPMENT_JSON)?;
        let encounters = registry("encounters", ENCOUNTERS_JSON)?;
        let targets = registry("targets", TARGETS_JSON)?;
        let environment = registry("environment", ENVIRONMENT_JSON)?;
        let outcomes = load_embedded_json_labeled("outcomes", OUTCOMES_JSON)?;

        Ok(Self {
            config,
            crew_pool,
            equipment,
            encounters,
            targets,
            environment,
            outcomes,
        })
    }

    /// Resolve a target's encounter sequence. An id with no template is dropped
    /// — [`Self::dangling_encounter_ids`] is what fails the content test.
    pub fn encounters_for(&self, target: &HeistTarget) -> Vec<&Encounter> {
        target
            .encounters
            .iter()
            .filter_map(|id| self.encounters.get(id))
            .collect()
    }

    /// Encounter ids named by a target that no template defines.
    pub fn dangling_encounter_ids(&self) -> Vec<String> {
        let mut dangling = Vec::new();
        for (_, target) in self.targets.iter() {
            for id in &target.encounters {
                if !self.encounters.contains(id) {
                    dangling.push(format!("{} -> {}", target.id, id));
                }
            }
        }
        dangling.sort();
        dangling
    }

    /// Encounter templates no target ever uses, and which are not marked as
    /// complications. Dead content is a bug (GDD 14).
    pub fn unreachable_encounter_ids(&self) -> Vec<String> {
        let mut unreachable: Vec<String> = self
            .encounters
            .iter()
            .filter(|(id, encounter)| {
                !encounter.complication_only
                    && !self
                        .targets
                        .iter()
                        .any(|(_, target)| target.encounters.contains(id))
            })
            .map(|(id, _)| id.clone())
            .collect();
        unreachable.sort();
        unreachable
    }

    /// Environment ids named by a target that `environment.json` never defines.
    pub fn dangling_environment_ids(&self) -> Vec<String> {
        let mut dangling = Vec::new();
        for (_, target) in self.targets.iter() {
            for id in crate::rules::environment::unknown_ids(&target.environment, |id| {
                self.environment.get(id)
            }) {
                dangling.push(format!("{} -> {}", target.id, id));
            }
        }
        dangling.sort();
        dangling
    }
}

fn registry<T>(label: &str, json: &str) -> Result<DataRegistry<T>, String>
where
    T: serde::de::DeserializeOwned + Clone,
{
    DataRegistry::from_embedded_json(json, "id").map_err(|err| format!("{}: {}", label, err))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Skill;

    #[test]
    fn every_data_file_parses() {
        let data = GameData::load().expect("embedded data must parse");

        assert!(!data.config.game_name.is_empty());
        assert!(!data.crew_pool.is_empty());
        assert!(!data.equipment.is_empty());
        assert!(!data.encounters.is_empty());
        assert!(!data.targets.is_empty());
        assert!(!data.environment.is_empty());
    }

    #[test]
    fn the_starting_crew_exists_on_the_books() {
        let data = GameData::load().unwrap();
        for id in &data.config.starting_crew {
            assert!(data.crew_pool.contains(id), "unknown starting crew: {}", id);
        }
        for id in &data.config.starting_inventory {
            assert!(data.equipment.contains(id), "unknown start item: {}", id);
        }
    }

    #[test]
    fn no_target_names_a_door_that_does_not_exist() {
        let data = GameData::load().unwrap();
        assert!(
            data.dangling_encounter_ids().is_empty(),
            "dangling encounters: {:?}",
            data.dangling_encounter_ids()
        );
        assert!(
            data.dangling_environment_ids().is_empty(),
            "dangling environment ids: {:?}",
            data.dangling_environment_ids()
        );
    }

    #[test]
    fn no_encounter_template_is_stranded() {
        let data = GameData::load().unwrap();
        assert!(
            data.unreachable_encounter_ids().is_empty(),
            "unreachable encounters: {:?}",
            data.unreachable_encounter_ids()
        );
    }

    #[test]
    fn every_outcome_band_has_a_line_for_every_skill() {
        let data = GameData::load().unwrap();
        for outcome in crate::rules::Outcome::ALL {
            for skill in Skill::ALL {
                assert!(
                    !data.outcomes.lines(outcome, skill).is_empty(),
                    "no {} lines for {}",
                    outcome.key(),
                    skill.key()
                );
            }
        }
    }

    #[test]
    fn every_target_sequence_is_a_playable_length() {
        let data = GameData::load().unwrap();
        for (id, target) in data.targets.iter() {
            let count = target.encounters.len();
            assert!((2..=6).contains(&count), "{} has {} encounters", id, count);
            assert!(target.potential_payout > 0, "{} pays nothing", id);
        }
    }
}
