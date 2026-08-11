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
