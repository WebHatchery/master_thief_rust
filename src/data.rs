//! The paperwork: every balance value, dossier, and narrative line, embedded.
//!
//! Nothing in here is hardcoded in Rust. Add a target, a tool, or an outcome
//! line by editing `assets/data/*.json`.

pub mod outcomes;

use crate::model::{CrewMember, Encounter, EnvironmentModifier, EquipmentDef, HeistTarget};
use macroquad_toolkit::data_loader::{load_embedded_json_labeled, DataRegistry};
use outcomes::OutcomeTables;
use serde::{Deserialize, Serialize};

const GAME_CONFIG_JSON: &str =
    macroquad_toolkit::include_json_str!("../assets/data/game_config.json");
const CHARACTERS_JSON: &str =
    macroquad_toolkit::include_json_str!("../assets/data/characters.json");
const EQUIPMENT_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/equipment.json");
const ENCOUNTERS_JSON: &str =
    macroquad_toolkit::include_json_str!("../assets/data/encounters.json");
const TARGETS_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/targets.json");
const ENVIRONMENT_JSON: &str =
    macroquad_toolkit::include_json_str!("../assets/data/environment.json");
const OUTCOMES_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/outcomes.json");
const ACHIEVEMENTS_JSON: &str =
    macroquad_toolkit::include_json_str!("../assets/data/achievements.json");
const TRAITS_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/traits.json");

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
    /// Hours of the crew's attention in a week, across everything. Scouting a
    /// door spends one and so does drilling a hand who has levelled, so the two
    /// compete: this is the half of casing that money cannot buy, and the half
    /// of training that experience does not (GDD 12, open question 2).
    pub attention_per_week: u32,

    /// Fatigue removed by a week of rest, before the constitution bonus.
    pub rest_recovery: i32,
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
    /// What a job pays, and what it leaves behind.
    pub payout: PayoutConfig,
    /// What a job the crew walked out of costs.
    pub walk_away: WalkAwayConfig,
    /// What the outfit costs to keep standing, week in, week out.
    pub payroll: PayrollConfig,
    /// What the city does about an outfit it has started to notice.
    pub law: LawConfig,
    /// How fatigue and loyalty grade into modifiers on the die.
    pub condition: crate::rules::ConditionTuning,
    /// What the city learns from the trades the outfit keeps working.
    pub scrutiny: crate::rules::ScrutinyTuning,
    /// How long it takes a hand to get good at their own trade.
    pub mastery: crate::rules::MasteryTuning,
}

/// What a job is worth. Every one of these spent the campaign written into
/// `sim/job/settle.rs` and `sim/loot.rs` as a literal, which is the one place
/// balance is not allowed to live — and the whole of the game's income runs
/// through them.
///
/// Not `Eq`: these are shares and chances, and pretending two floats compare
/// exactly would be a lie about what comparing them means.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PayoutConfig {
    /// Share of a mark's doors that have to be cleared for the job to have
    /// worked at all.
    pub success_threshold: f32,
    /// Above this share, the crew were good enough to be paid for it.
    pub clean_threshold: f32,
    /// What a job that clean is multiplied by.
    pub clean_bonus: f32,
    /// Share of what the cleared doors were worth that a *failed* job fetches.
    ///
    /// It used to be a flat fifteen per cent of the mark regardless, which
    /// meant that on a job the crew were losing, getting one more door open was
    /// worth exactly nothing — and a total wipeout paid the same as a near
    /// miss. Proportional, every door cleared is worth something and nothing
    /// cleared is worth nothing.
    pub failed_share: f32,
    /// The same, for a job the crew were told to walk out of. Lower than
    /// `failed_share`: staying means carrying more out.
    pub walked_share: f32,
    /// Chance a finished job leaves something behind at all.
    pub loot_chance: f32,
    /// Added chance per door taken with a natural flourish.
    pub loot_chance_per_critical: f32,
}

/// What a job the crew were told to abandon costs. The standing order is set
/// before the dice, so this is the price of a nerve the fixer committed to in
/// advance (GDD 5.2). What it *pays* is `payout.walked_share`.
///
/// Not `Eq`: this is a share, and pretending two floats compare exactly would
/// be a lie about what comparing them means.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WalkAwayConfig {
    /// Share of the mark's notoriety a crew who left still pick up. The rest,
    /// and the botched-job penalty entirely, is what the forfeited take buys.
    pub notoriety_share: f32,
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
    /// How hard each personality takes what it watches (GDD 5.5).
    pub trait_rates: crate::rules::chemistry::TraitRates,
}

impl GameData {
    pub fn load() -> Result<Self, String> {
        let config = load_embedded_json_labeled("game_config", GAME_CONFIG_JSON)?;
        let crew_pool = DataRegistry::from_embedded_json(CHARACTERS_JSON, "id")
            .map_err(|error| format!("characters: {error}"))?;
        let equipment = DataRegistry::from_embedded_json(EQUIPMENT_JSON, "id")
            .map_err(|error| format!("equipment: {error}"))?;
        let encounters = DataRegistry::from_embedded_json(ENCOUNTERS_JSON, "id")
            .map_err(|error| format!("encounters: {error}"))?;
        let targets = DataRegistry::from_embedded_json(TARGETS_JSON, "id")
            .map_err(|error| format!("targets: {error}"))?;
        let environment = DataRegistry::from_embedded_json(ENVIRONMENT_JSON, "id")
            .map_err(|error| format!("environment: {error}"))?;
        let outcomes = load_embedded_json_labeled("outcomes", OUTCOMES_JSON)?;
        let awards = load_embedded_json_labeled("achievements", ACHIEVEMENTS_JSON)?;
        let trait_rates = load_embedded_json_labeled("traits", TRAITS_JSON)?;

        Ok(Self {
            config,
            crew_pool,
            equipment,
            encounters,
            targets,
            environment,
            outcomes,
            awards,
            trait_rates,
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

#[cfg(test)]
mod tests;
