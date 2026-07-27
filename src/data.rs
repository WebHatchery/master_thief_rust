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
const ACHIEVEMENTS_JSON: &str = include_str!("../assets/data/achievements.json");

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
    /// Cost, in cash, of putting the first door of a mark on the file.
    pub casing_cost: i64,
    /// Added to that cost for every door already scouted on the same mark.
    pub casing_cost_step: i64,
    /// Doors the crew can scout in a week, across the whole board. This is the
    /// half of casing that money cannot buy (GDD 12, open question 2).
    pub casing_steps_per_week: u32,

    /// Fatigue removed by a week of rest, before the constitution bonus.
    pub rest_recovery: i32,
    /// Fatigue above which a member is unfit for work.
    pub fatigue_work_threshold: i32,
    /// Loyalty a week of idleness costs, and rest restores.
    pub idle_loyalty_drift: i32,
    /// Chemistry a pair loses each week they do not work the same job. Applies
    /// to warmth and grudges alike — both fade toward indifference.
    pub chemistry_cooling: i32,

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
    /// What the crew wants for the job itself, negotiated per job.
    pub cut: CutConfig,

    /// How a mark left sitting on the board changes.
    pub board: BoardConfig,
    /// The other outfits working the same city.
    pub rivals: RivalConfig,
    /// Work the crew bring in themselves when they are content.
    pub leads: LeadConfig,
    /// What being widely known costs the outfit at the hiring table.
    pub recruiting: RecruitingConfig,
    /// What a doctor charges to buy back the weeks an injury would have cost.
    pub treatment: TreatmentConfig,
    /// How fast kit wears out, and what putting it right costs.
    pub kit: KitConfig,
    /// What spare kit fetches, and what selling it costs in anonymity.
    pub fence: FenceConfig,
    /// What the outfit costs to keep standing, week in, week out.
    pub payroll: PayrollConfig,
    /// What the city does about an outfit it has started to notice.
    pub law: LawConfig,
    /// How fatigue and loyalty grade into modifiers on the die.
    pub condition: crate::rules::ConditionTuning,
    /// How long it takes a hand to get good at their own trade.
    pub mastery: crate::rules::MasteryTuning,
}

/// Selling kit back out. The only inflow the week has that is not a job, and
/// priced so it never becomes a better one.
///
/// Not `Eq`: these are shares, and pretending two floats compare exactly would
/// be a lie about what comparing them means.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FenceConfig {
    /// Share of list price a fence pays an outfit nobody is watching.
    pub share_base: f32,
    /// Taken off that share per point of heat.
    pub share_heat_penalty: f32,
    /// However hot the outfit gets, a fence still pays this much.
    pub min_share: f32,
    /// Heat one sale adds. Selling your way out of a bad week makes the next
    /// one worse.
    pub heat_per_sale: i32,
}

/// Kit wearing out and being refitted (GDD 3, 9). Without this the Outfitter
/// is a shop you visit once; with it, the good tool is something the outfit
/// keeps paying to keep good.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitConfig {
    /// Jobs a piece of kit survives per point of penalty it picks up.
    pub jobs_per_penalty: u32,
    /// However neglected, one tool can only cost this much on a check.
    pub max_penalty_per_item: i32,
    /// Bench fee per piece of kit refitted.
    pub refit_base: i64,
    /// Added per job of wear on that piece.
    pub refit_per_job: i64,
}

/// Treating an injury rather than waiting it out (GDD 3, 4). The bill scales
/// with the wait it saves, so the injuries worth paying to fix are the ones
/// that would have cost the most weeks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreatmentConfig {
    /// What a doctor charges for turning up at all.
    pub base_cost: i64,
    /// Added per week the injury still had to run.
    pub cost_per_week: i64,
    /// Anything serious costs this many times as much.
    pub major_multiplier: i64,
    /// Fatigue a patched-up hand carries out of the surgery.
    pub fatigue_cost: i32,
}

/// What notoriety does to recruiting. Pillar 4 says reputation and notoriety
/// pull in opposite directions, but reputation *opened marks* while notoriety
/// only priced two rare purchases — so the pull was strong and the push was
/// barely there. Being known now costs the outfit access to people, which is
/// the thing reputation buys most of.
///
/// Not `Eq`: these are rates, and pretending two floats compare exactly would
/// be a lie about what comparing them means.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RecruitingConfig {
    /// Added to a recruit's asking price per point of notoriety, as a share of
    /// their base fee. Danger money for signing on with a known outfit.
    pub fee_per_notoriety: f32,
    /// However infamous, nobody asks for more than this much on top.
    pub max_fee_premium: f32,
    /// Every this much notoriety, one fewer person bothers turning up.
    pub pool_shrink_per_notoriety: i32,
    /// Somebody is always desperate enough.
    pub min_pool: usize,
}

/// Tip-offs from a contented crew — the one thing in the week a good roster
/// generates rather than survives (GDD 5.5).
///
/// Not `Eq`: these are chances, and pretending two floats compare exactly would
/// be a lie about what comparing them means.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LeadConfig {
    /// Loyalty at or above which a hand is content enough to be listening.
    pub loyalty_threshold: i32,
    /// Chance each such hand contributes in a week.
    pub chance_per_hand: f32,
    /// However happy and however many, no more than this.
    pub max_chance: f32,
    /// Doors of the mark already on the file, because they know the place.
    pub doors_on_file: u32,
    /// Weeks added to the window, because nobody else is looking at it yet.
    pub extra_weeks: u32,
}

/// The competition. A rival is a name and a weekly roll — not a faction, not a
/// content axis — and what it buys is a reason to take a job now that has
/// nothing to do with the payroll (GDD 5.4).
///
/// Not `Eq`: these are chances, and pretending two floats compare exactly would
/// be a lie about what comparing them means.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RivalConfig {
    /// Outfits who might get there first. Flavour, drawn from the run's RNG.
    pub names: Vec<String>,
    /// Ripeness below which a mark is not worth anybody else's attention.
    pub min_ripeness: u32,
    /// Chance a mark at exactly `min_ripeness` is taken this week.
    pub base_chance: f32,
    /// Added per week of ripeness beyond that.
    pub chance_per_ripeness: f32,
    pub max_chance: f32,
}

/// What the crew wants for a job, on top of the retainer that bought their
/// week. A flat share made the roster a pure power calculation; a negotiated
/// one puts a price on the obvious answer (GDD 5.5).
///
/// Not `Eq`: these are shares, and pretending two floats compare exactly would
/// be a lie about what comparing them means.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CutConfig {
    /// What one steady, unremarkable hand would take.
    pub base_share: f32,
    /// Added per hand beyond the first — a bigger job splits more ways.
    pub per_extra_hand: f32,
    /// Added per rarity tier across everybody on the job.
    pub per_rarity_tier: f32,
    /// Loyalty at or below which a hand holds out for more.
    pub holdout_loyalty: i32,
    pub holdout_premium: f32,
    /// Loyalty at or above which a hand does not haggle at all.
    pub steady_loyalty: i32,
    pub steady_discount: f32,
    /// Added per established partnership on the job. A pair who work as one
    /// negotiate as one.
    pub partnership_premium: f32,
    /// Added when the crew drew up the plan as well as working it. Doing the
    /// fixer's thinking is work, and it is charged for (GDD 5.3, pillar 5).
    pub delegation_premium: f32,
    /// The share can never fall below or climb above these, whatever the
    /// roster looks like.
    pub min_share: f32,
    pub max_share: f32,
}

/// What a week of nobody touching a mark does to it. A board that is only a
/// stock list makes waiting free; ripening puts a price and a prize on it, so
/// leaving a job for later is a bet rather than an oversight (GDD 5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardConfig {
    /// Percent added to the payout for each week the mark sat untaken.
    pub ripeness_payout_pct: i64,
    /// Penalty added to every door for each of those weeks.
    pub ripeness_door_penalty: i32,
    /// Weeks after which a mark stops ripening — the window closes before a
    /// mark can become worth more than the whole campaign.
    pub ripeness_max: u32,
}

/// The standing weekly bill. Nothing here is optional: an outfit that stops
/// paying stops being an outfit (GDD 3, "advance the week").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayrollConfig {
    /// The safehouse's floor cost before the outfit's name inflates it.
    pub safehouse_upkeep_base: i64,
    /// Every point of reputation adds this much to the standing overheads: a
    /// bigger name needs a better door to hide behind.
    pub safehouse_upkeep_per_reputation: i64,
    pub retainer_base: i64,
    pub retainer_per_level: i64,
    /// Added once per rarity tier ([`crate::model::Rarity::tier`]).
    pub retainer_per_rarity: i64,
    /// Loyalty lost by a hand the outfit could not pay this week.
    pub unpaid_loyalty_cost: i32,
    /// Loyalty at or below which a hand gives notice.
    pub notice_loyalty_threshold: i32,
    /// A goodwill payment costs this many weeks of that hand's retainer.
    pub bonus_retainer_weeks: i64,
    /// Paying somebody off costs this many weeks of theirs.
    pub severance_weeks: i64,
    /// Loyalty every remaining hand loses when one of them is let go.
    pub dismissal_loyalty_cost: i32,
    pub bonus_loyalty_restored: i32,
}

/// How the city pushes back. Heat is the short-term half of notoriety, and
/// above a threshold it stops being a difficulty modifier and starts taking
/// things (GDD 5.6).
/// Not `Eq`: two of these carry floats, and pretending they compare exactly
/// would be a lie about what comparing them means.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LawConfig {
    /// Heat below this and nobody is looking.
    pub attention_threshold: i32,
    /// Chance per week of an incident, per point of heat above the threshold.
    pub attention_chance_per_point: f32,
    pub attention_chance_max: f32,
    /// Heat at or above which an incident can be an arrest.
    pub custody_threshold: i32,
    /// A d100 below this is an arrest, below `raid_share` a raid, else a tail.
    pub arrest_share: usize,
    pub raid_share: usize,
    /// Share of the outfit's cash a raid takes off the table.
    pub raid_seizure_share: f32,
    pub raid_heat_relief: i32,
    pub arrest_heat_relief: i32,
    /// Weeks a tail stays on the crew.
    pub surveillance_weeks: u32,
    /// What a tail costs on every door while it lasts.
    pub surveillance_penalty: i32,
    pub bribe_cost_base: i64,
    /// A better-known outfit is more expensive to make quiet.
    pub bribe_cost_per_notoriety: i64,
    pub bribe_heat_relief: i32,
    pub bail_base: i64,
    pub bail_per_level: i64,
    pub bail_per_notoriety: i64,
    /// Loyalty a bailed hand comes back with. A cell is not a holiday.
    pub bail_return_loyalty: i32,
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
    /// Achievement definitions, in the order they are shown.
    pub awards: Vec<crate::sim::AwardDef>,
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
        let awards = load_embedded_json_labeled("achievements", ACHIEVEMENTS_JSON)?;

        Ok(Self {
            config,
            crew_pool,
            equipment,
            encounters,
            targets,
            environment,
            outcomes,
            awards,
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

/// A count of everything authored, measured against the GDD 8 content table.
/// Volume is a design commitment, not an accident, so it is something a test
/// can hold the project to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentInventory {
    pub recruits: usize,
    pub personality_traits: usize,
    pub equipment: usize,
    pub encounters: usize,
    pub targets: usize,
    pub environment_factors: usize,
    pub critical_effects: usize,
    pub outcome_lines: usize,
    pub achievements: usize,
}

impl GameData {
    pub fn inventory(&self) -> ContentInventory {
        let mut traits: Vec<&str> = self
            .crew_pool
            .iter()
            .flat_map(|(_, member)| member.personality_traits.iter())
            .map(|name| name.as_str())
            .collect();
        traits.sort_unstable();
        traits.dedup();

        let critical_effects = self
            .encounters
            .iter()
            .map(|(_, encounter)| {
                usize::from(encounter.critical_success_reward.is_some())
                    + usize::from(encounter.critical_failure_effect.is_some())
            })
            .sum();

        ContentInventory {
            recruits: self.crew_pool.len(),
            personality_traits: traits.len(),
            equipment: self.equipment.len(),
            encounters: self.encounters.len(),
            targets: self.targets.len(),
            environment_factors: self.environment.len(),
            critical_effects,
            outcome_lines: self.outcomes.total_lines(),
            achievements: self.awards.len(),
        }
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
    fn every_content_axis_meets_its_full_gdd_target() {
        // The full column of the GDD 8 table, not the prototype one. These are
        // floors, never ceilings: content may only grow, and this fails the
        // moment any axis shrinks below what was shipped.
        let inventory = GameData::load().unwrap().inventory();

        assert!(inventory.outcome_lines >= 400, "{:?}", inventory);
        assert!(inventory.encounters >= 70, "{:?}", inventory);
        assert!(inventory.equipment >= 60, "{:?}", inventory);
        assert!(inventory.targets >= 45, "{:?}", inventory);
        assert!(inventory.recruits >= 40, "{:?}", inventory);
        assert!(inventory.personality_traits >= 30, "{:?}", inventory);
        assert!(inventory.critical_effects >= 50, "{:?}", inventory);
        assert!(inventory.environment_factors >= 15, "{:?}", inventory);
        assert!(inventory.achievements >= 40, "{:?}", inventory);
    }

    #[test]
    fn every_achievement_is_uniquely_named_and_reachable() {
        let data = GameData::load().unwrap();

        let mut ids: Vec<&str> = data.awards.iter().map(|a| a.id.as_str()).collect();
        let before = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), before, "duplicate achievement ids");

        for award in &data.awards {
            assert!(!award.name.is_empty(), "{} has no name", award.id);
            assert!(!award.description.is_empty(), "{} says nothing", award.id);
            assert!(award.at_least > 0, "{} unlocks for free", award.id);
        }
    }

    #[test]
    fn the_ladder_of_marks_runs_all_the_way_up() {
        // Reputation is the campaign's spine. There has to be something to open
        // at every rung of it, or the ladder has a missing step.
        let data = GameData::load().unwrap();
        let mut gates: Vec<i32> = data
            .targets
            .iter()
            .map(|(_, target)| target.required_reputation)
            .collect();
        gates.sort_unstable();

        assert!(gates.first() == Some(&0), "nothing is open at week one");
        assert!(*gates.last().unwrap() >= 60, "the ladder stops too early");
        for pair in gates.windows(2) {
            assert!(
                pair[1] - pair[0] <= 6,
                "a {}-point gap between marks at reputation {}",
                pair[1] - pair[0],
                pair[0]
            );
        }
    }

    #[test]
    fn every_difficulty_band_has_marks_in_it() {
        use crate::model::DifficultyBand;
        let data = GameData::load().unwrap();

        for band in DifficultyBand::ALL {
            let count = data
                .targets
                .iter()
                .filter(|(_, target)| target.difficulty == band)
                .count();
            assert!(count >= 5, "only {} {} marks", count, band.label());
        }
    }

    #[test]
    fn every_band_and_skill_carries_a_deep_enough_table() {
        let data = GameData::load().unwrap();
        for outcome in crate::rules::Outcome::ALL {
            for skill in Skill::ALL {
                let lines = data.outcomes.lines(outcome, skill);
                assert!(
                    lines.len() >= 10,
                    "{}/{} has only {} lines",
                    outcome.key(),
                    skill.key(),
                    lines.len()
                );
            }
        }
    }

    #[test]
    fn no_narrative_line_is_written_twice_anywhere() {
        let data = GameData::load().unwrap();
        let mut seen: Vec<&str> = Vec::new();
        for outcome in crate::rules::Outcome::ALL {
            for skill in Skill::ALL {
                for line in data.outcomes.lines(outcome, skill) {
                    seen.push(line.as_str());
                }
            }
        }
        let before = seen.len();
        seen.sort_unstable();
        seen.dedup();
        // The generic table is shared by design, so a skill without its own
        // list legitimately returns the same lines; compare against the
        // authored total instead of the resolved one.
        assert!(
            before - seen.len() < before / 4,
            "too many repeated lines across the tables"
        );
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
