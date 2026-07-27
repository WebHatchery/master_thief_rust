//! Crew chemistry: who works well with whom, and who will not work at all.
//!
//! The original tracked a `characterRelationships` field that nothing ever
//! wrote — three `// TODO: Implement full relationship system` sites and a dead
//! column in the save. GDD 5.5 says build it properly or delete the field; this
//! is the building. Chemistry moves on shared outcomes, personality sets the
//! rate, and the result is a small named modifier when two hands are on the same
//! job — enough to make the roster a composition problem rather than a sum.

use super::outcome::{ModifierEntry, Outcome};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The scale chemistry lives on. Beyond the refusal floor a pair will not take
/// the same job.
pub const MAX: i32 = 100;
pub const MIN: i32 = -100;
/// At or below this, the pair refuses to work together.
pub const REFUSAL: i32 = -60;
/// At or above this, a pair has become a unit — they read each other on a job,
/// and they know it when the cut is discussed (GDD 5.5).
///
/// Set against what play actually produces, not against the nominal ±100
/// scale: a campaign's warmest pair lands in the teens or twenties, because
/// every week apart cools them. A threshold of forty was one no crew ever
/// reached, which made the whole tier decoration.
pub const PARTNERSHIP: i32 = 20;
/// How many points of chemistry buy one point on a check. Same reasoning: at
/// twenty-five, the modifier this system exists to produce almost never fired.
const POINTS_PER_MODIFIER: i32 = 12;
/// The most chemistry can swing a single check, either way.
const MODIFIER_CAP: i32 = 3;

/// A symmetric table of pair chemistry, stored one entry per unordered pair.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chemistry {
    pairs: BTreeMap<String, i32>,
}

fn key(a: &str, b: &str) -> String {
    if a <= b {
        format!("{}|{}", a, b)
    } else {
        format!("{}|{}", b, a)
    }
}

impl Chemistry {
    pub fn get(&self, a: &str, b: &str) -> i32 {
        if a == b {
            return 0;
        }
        self.pairs.get(&key(a, b)).copied().unwrap_or(0)
    }

    pub fn set(&mut self, a: &str, b: &str, value: i32) {
        if a == b {
            return;
        }
        self.pairs.insert(key(a, b), value.clamp(MIN, MAX));
    }

    pub fn adjust(&mut self, a: &str, b: &str, delta: i32) {
        let next = self.get(a, b) + delta;
        self.set(a, b, next);
    }

    /// Will these two take the same job?
    pub fn refuses(&self, a: &str, b: &str) -> bool {
        self.get(a, b) <= REFUSAL
    }

    /// Anyone on the crew this member will not stand beside.
    pub fn refusals<'a>(&self, member: &str, crew: impl Iterator<Item = &'a str>) -> Vec<String> {
        crew.filter(|other| *other != member && self.refuses(member, other))
            .map(|other| other.to_owned())
            .collect()
    }

    /// The named modifier one member carries for the company they are keeping
    /// on this job. Averaged, so a big crew does not stack into a landslide.
    pub fn modifier(&self, member: &str, others: &[String]) -> Option<ModifierEntry> {
        let scores: Vec<i32> = others
            .iter()
            .filter(|other| other.as_str() != member)
            .map(|other| self.get(member, other))
            .collect();

        if scores.is_empty() {
            return None;
        }

        let average = scores.iter().sum::<i32>() / scores.len() as i32;
        let value = (average / POINTS_PER_MODIFIER).clamp(-MODIFIER_CAP, MODIFIER_CAP);
        (value != 0).then(|| {
            ModifierEntry::new(
                if value > 0 {
                    "Crew rapport"
                } else {
                    "Crew friction"
                },
                value,
            )
        })
    }

    /// Have these two become a unit?
    pub fn is_partnership(&self, a: &str, b: &str) -> bool {
        self.get(a, b) >= PARTNERSHIP
    }

    /// Established pairs among the hands down for one job, each counted once.
    /// This is what the cut is negotiated against: a pair who work as one know
    /// what a pair is worth.
    pub fn partnerships_among(&self, crew: &[String]) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        for (index, a) in crew.iter().enumerate() {
            for b in crew.iter().skip(index + 1) {
                if self.is_partnership(a, b) {
                    pairs.push((a.clone(), b.clone()));
                }
            }
        }
        pairs
    }

    /// A week apart. Warmth and grudges both fade toward indifference when a
    /// pair is not put in the same building — chemistry the fixer wants has to
    /// be kept up, and a grudge they can wait out will cool on its own.
    pub fn cool_off(&mut self, worked_together: &[String], amount: i32) {
        if amount <= 0 {
            return;
        }
        let mut worked: Vec<String> = Vec::new();
        for (index, a) in worked_together.iter().enumerate() {
            for b in worked_together.iter().skip(index + 1) {
                worked.push(key(a, b));
            }
        }

        for (pair, value) in self.pairs.iter_mut() {
            if worked.contains(pair) {
                continue;
            }
            *value -= value.signum() * amount.min(value.abs());
        }
        self.pairs.retain(|_, value| *value != 0);
    }

    /// Every pair the table has an opinion about, for the crew screen.
    pub fn known_pairs(&self) -> impl Iterator<Item = (&str, &str, i32)> {
        self.pairs.iter().filter_map(|(pair, value)| {
            let (a, b) = pair.split_once('|')?;
            Some((a, b, *value))
        })
    }

    /// Drop anyone who has left the payroll, so a retired hand's grudges do not
    /// outlive them in the save.
    pub fn retain_crew(&mut self, crew: &[String]) {
        self.pairs.retain(|pair, _| {
            pair.split_once('|')
                .map(|(a, b)| crew.iter().any(|id| id == a) && crew.iter().any(|id| id == b))
                .unwrap_or(false)
        });
    }
}

/// How hard a personality takes what it just watched. Steady hands barely move;
/// reckless ones swing.
pub fn reaction_rate(traits: &[String]) -> f32 {
    let mut rate: f32 = 1.0;
    for name in traits {
        rate *= match name.to_ascii_lowercase().as_str() {
            // Hands who work to a method do not re-read a bad night.
            "steady" | "patient" | "methodical" | "meticulous" | "punctual" | "quiet" => 0.7,
            // Hands who take everything personally take this personally too.
            "reckless" | "impulsive" | "vain" | "hot-headed" | "greedy" | "proud" => 1.4,
            // Expecting the worst blunts it when it arrives.
            "cynical" | "guarded" | "private" | "suspicious" | "fatalistic" | "nervous" => 0.85,
            // Warm people warm faster, and cool faster too.
            "optimistic" | "charming" | "loyal" | "generous" | "gregarious" | "sentimental" => 1.15,
            _ => 1.0,
        };
    }
    rate.clamp(0.4, 2.0)
}

/// What watching one door does to a pair, before the watcher's personality is
/// applied. Succeeding together raises it; watching a partner blow it lowers it.
pub fn shared_outcome_delta(outcome: Outcome) -> i32 {
    match outcome {
        Outcome::CriticalSuccess => 4,
        Outcome::Success => 2,
        Outcome::Neutral => 0,
        Outcome::Failure => -2,
        Outcome::CriticalFailure => -5,
    }
}

/// Apply one door's outcome across the pairs that watched it happen.
pub fn record_outcome(
    chemistry: &mut Chemistry,
    actor: &str,
    watchers: &[(String, Vec<String>)],
    outcome: Outcome,
) {
    let base = shared_outcome_delta(outcome);
    if base == 0 {
        return;
    }

    for (watcher, traits) in watchers {
        if watcher == actor {
            continue;
        }
        let delta = (base as f32 * reaction_rate(traits)).round() as i32;
        chemistry.adjust(actor, watcher, delta);
    }
}

#[cfg(test)]
mod partnership_tests {
    use super::*;

    fn ids(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn a_warm_enough_pair_counts_as_a_partnership() {
        let mut chemistry = Chemistry::default();
        chemistry.set("vera", "otis", PARTNERSHIP - 1);
        assert!(!chemistry.is_partnership("vera", "otis"));

        chemistry.set("vera", "otis", PARTNERSHIP);
        assert!(chemistry.is_partnership("vera", "otis"));
    }

    #[test]
    fn partnerships_on_a_job_are_each_counted_once() {
        let mut chemistry = Chemistry::default();
        chemistry.set("vera", "otis", 70);
        chemistry.set("otis", "birdie", 80);
        chemistry.set("vera", "birdie", 10);

        let pairs = chemistry.partnerships_among(&ids(&["vera", "otis", "birdie"]));
        assert_eq!(pairs.len(), 2, "{:?}", pairs);
        assert!(!pairs.contains(&("vera".to_owned(), "birdie".to_owned())));
    }

    #[test]
    fn a_pair_kept_apart_drifts_back_toward_indifference() {
        // The other half of what makes a good pair a decision: warmth is not a
        // permanent acquisition, it is something the fixer keeps paying for.
        let mut chemistry = Chemistry::default();
        chemistry.set("vera", "otis", 50);
        chemistry.set("vera", "birdie", -50);

        chemistry.cool_off(&[], 5);
        assert_eq!(chemistry.get("vera", "otis"), 45);
        assert_eq!(chemistry.get("vera", "birdie"), -45, "grudges cool too");
    }

    #[test]
    fn a_pair_who_worked_the_same_job_do_not_cool() {
        let mut chemistry = Chemistry::default();
        chemistry.set("vera", "otis", 50);
        chemistry.set("vera", "birdie", 50);

        chemistry.cool_off(&ids(&["vera", "otis"]), 5);
        assert_eq!(chemistry.get("vera", "otis"), 50);
        assert_eq!(chemistry.get("vera", "birdie"), 45);
    }

    #[test]
    fn cooling_settles_at_indifference_rather_than_overshooting() {
        let mut chemistry = Chemistry::default();
        chemistry.set("vera", "otis", 3);
        chemistry.set("vera", "birdie", -2);

        chemistry.cool_off(&[], 10);
        assert_eq!(chemistry.get("vera", "otis"), 0);
        assert_eq!(chemistry.get("vera", "birdie"), 0);
        assert_eq!(
            chemistry.known_pairs().count(),
            0,
            "dead opinions are dropped"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn watchers(names: &[(&str, &[&str])]) -> Vec<(String, Vec<String>)> {
        names
            .iter()
            .map(|(id, traits)| {
                (
                    (*id).to_owned(),
                    traits.iter().map(|t| (*t).to_owned()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn a_pair_starts_with_no_opinion_either_way() {
        let chemistry = Chemistry::default();
        assert_eq!(chemistry.get("vera", "otis"), 0);
        assert!(!chemistry.refuses("vera", "otis"));
    }

    #[test]
    fn chemistry_is_symmetric_however_it_is_asked() {
        let mut chemistry = Chemistry::default();
        chemistry.adjust("otis", "vera", 12);
        assert_eq!(chemistry.get("vera", "otis"), 12);
        assert_eq!(chemistry.get("otis", "vera"), 12);
    }

    #[test]
    fn nobody_has_chemistry_with_themselves() {
        let mut chemistry = Chemistry::default();
        chemistry.adjust("vera", "vera", 50);
        assert_eq!(chemistry.get("vera", "vera"), 0);
    }

    #[test]
    fn chemistry_cannot_run_off_its_scale() {
        let mut chemistry = Chemistry::default();
        chemistry.adjust("vera", "otis", 500);
        assert_eq!(chemistry.get("vera", "otis"), MAX);
        chemistry.adjust("vera", "otis", -500);
        assert_eq!(chemistry.get("vera", "otis"), MIN);
    }

    #[test]
    fn succeeding_together_warms_a_pair_and_disaster_cools_it() {
        let mut chemistry = Chemistry::default();
        let crew = watchers(&[("vera", &[]), ("otis", &[])]);

        record_outcome(&mut chemistry, "vera", &crew, Outcome::Success);
        assert!(chemistry.get("vera", "otis") > 0);

        record_outcome(&mut chemistry, "vera", &crew, Outcome::CriticalFailure);
        assert!(chemistry.get("vera", "otis") < 0);
    }

    #[test]
    fn personality_sets_how_hard_a_watcher_takes_it() {
        let mut steady = Chemistry::default();
        let mut reckless = Chemistry::default();

        record_outcome(
            &mut steady,
            "vera",
            &watchers(&[("otis", &["Steady"])]),
            Outcome::CriticalFailure,
        );
        record_outcome(
            &mut reckless,
            "vera",
            &watchers(&[("otis", &["Reckless"])]),
            Outcome::CriticalFailure,
        );

        assert!(
            reckless.get("vera", "otis") < steady.get("vera", "otis"),
            "a reckless watcher should hold it against them harder"
        );
    }

    #[test]
    fn a_neutral_door_changes_nobodys_mind() {
        let mut chemistry = Chemistry::default();
        record_outcome(
            &mut chemistry,
            "vera",
            &watchers(&[("otis", &[])]),
            Outcome::Neutral,
        );
        assert_eq!(chemistry.get("vera", "otis"), 0);
    }

    #[test]
    fn the_actor_never_scores_against_themselves() {
        let mut chemistry = Chemistry::default();
        record_outcome(
            &mut chemistry,
            "vera",
            &watchers(&[("vera", &[]), ("otis", &[])]),
            Outcome::Success,
        );
        assert_eq!(chemistry.get("vera", "vera"), 0);
        assert!(chemistry.get("vera", "otis") > 0);
    }

    #[test]
    fn rapport_and_friction_arrive_as_named_modifiers() {
        let mut chemistry = Chemistry::default();
        chemistry.set("vera", "otis", 80);
        let others = vec!["otis".to_owned()];

        let entry = chemistry.modifier("vera", &others).unwrap();
        assert_eq!(entry.label, "Crew rapport");
        assert_eq!(entry.value, 3);

        chemistry.set("vera", "otis", -80);
        let entry = chemistry.modifier("vera", &others).unwrap();
        assert_eq!(entry.label, "Crew friction");
        assert_eq!(entry.value, -3);
    }

    #[test]
    fn a_modifier_is_capped_however_fond_the_crew_get() {
        let mut chemistry = Chemistry::default();
        chemistry.set("vera", "otis", MAX);
        let entry = chemistry.modifier("vera", &["otis".to_owned()]).unwrap();
        assert_eq!(entry.value, MODIFIER_CAP);
    }

    #[test]
    fn working_alone_carries_no_chemistry_at_all() {
        let mut chemistry = Chemistry::default();
        chemistry.set("vera", "otis", 90);
        assert!(chemistry.modifier("vera", &[]).is_none());
        assert!(chemistry.modifier("vera", &["vera".to_owned()]).is_none());
    }

    #[test]
    fn a_bad_enough_pair_refuses_the_same_job() {
        let mut chemistry = Chemistry::default();
        chemistry.set("vera", "otis", REFUSAL);
        assert!(chemistry.refuses("vera", "otis"));

        let crew = ["vera".to_owned(), "otis".to_owned(), "birdie".to_owned()];
        let refused = chemistry.refusals("vera", crew.iter().map(|s| s.as_str()));
        assert_eq!(refused, vec!["otis"]);
    }

    #[test]
    fn a_hand_who_leaves_takes_their_grudges_with_them() {
        let mut chemistry = Chemistry::default();
        chemistry.set("vera", "otis", 40);
        chemistry.set("vera", "birdie", -30);

        chemistry.retain_crew(&["vera".to_owned(), "birdie".to_owned()]);
        assert_eq!(chemistry.get("vera", "otis"), 0);
        assert_eq!(chemistry.get("vera", "birdie"), -30);
    }
}
