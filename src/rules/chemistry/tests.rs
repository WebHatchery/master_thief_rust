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

/// The table the game actually ships, so these tests cannot pass against
/// numbers the content does not use.
fn rates() -> TraitRates {
    crate::data::GameData::load().unwrap().trait_rates
}

#[test]
fn every_authored_trait_moves_somebody() {
    // Six traits used to match nothing and silently mean "no reaction".
    // A hand with a personality that does not reach the rules has a
    // decorative personality, which GDD 0 says this port does not ship.
    let data = crate::data::GameData::load().unwrap();
    for (_, member) in data.crew_pool.iter() {
        for trait_name in &member.personality_traits {
            assert!(
                data.trait_rates
                    .contains_key(&trait_name.to_ascii_lowercase()),
                "{} carries \"{}\", which the reaction table has never heard of",
                member.name,
                trait_name
            );
        }
    }
}

#[test]
fn a_trait_the_table_knows_actually_changes_the_rate() {
    let rates = rates();
    assert_ne!(reaction_rate(&["Steady".to_owned()], &rates), 1.0);
    assert_ne!(reaction_rate(&["Stubborn".to_owned()], &rates), 1.0);
    assert_ne!(reaction_rate(&["Superstitious".to_owned()], &rates), 1.0);
    // Unknown names are ignored rather than panicking: content can be
    // added before its rate is authored, and the test above catches that.
    assert_eq!(reaction_rate(&["Nonesuch".to_owned()], &rates), 1.0);
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

    record_outcome(&mut chemistry, "vera", &crew, Outcome::Success, &rates());
    assert!(chemistry.get("vera", "otis") > 0);

    record_outcome(
        &mut chemistry,
        "vera",
        &crew,
        Outcome::CriticalFailure,
        &rates(),
    );
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
        &rates(),
    );
    record_outcome(
        &mut reckless,
        "vera",
        &watchers(&[("otis", &["Reckless"])]),
        Outcome::CriticalFailure,
        &rates(),
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
        &rates(),
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
        &rates(),
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
