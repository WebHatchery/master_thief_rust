use super::*;
use crate::model::Skills;

fn modifier(id: &str, name: &str, skills: Skills, all: i32) -> EnvironmentModifier {
    EnvironmentModifier {
        id: id.to_owned(),
        name: name.to_owned(),
        description: String::new(),
        skill_modifiers: skills,
        all_skills: all,
    }
}

fn registry() -> Vec<EnvironmentModifier> {
    vec![
        modifier(
            "night",
            "Night",
            Skills {
                stealth: 2,
                social: -1,
                ..Skills::default()
            },
            0,
        ),
        modifier(
            "day",
            "Daylight",
            Skills {
                stealth: -1,
                social: 1,
                ..Skills::default()
            },
            0,
        ),
        modifier(
            "fog",
            "Fog",
            Skills {
                stealth: 3,
                hacking: -1,
                ..Skills::default()
            },
            0,
        ),
        modifier(
            "rain",
            "Rain",
            Skills {
                stealth: 1,
                athletics: -2,
                ..Skills::default()
            },
            0,
        ),
        modifier("high_security", "High Security", Skills::default(), -3),
    ]
}

fn lookup<'a>(
    defs: &'a [EnvironmentModifier],
) -> impl Fn(&str) -> Option<&'a EnvironmentModifier> + 'a {
    move |id| defs.iter().find(|def| def.id == id)
}

#[test]
fn night_favours_stealth_and_costs_the_talker() {
    let defs = registry();
    let env = Environment::new("night", "clear");
    assert_eq!(environment_total(&env, Skill::Stealth, lookup(&defs)), 2);
    assert_eq!(environment_total(&env, Skill::Social, lookup(&defs)), -1);
}

#[test]
fn daylight_favours_the_face() {
    let defs = registry();
    let env = Environment::new("day", "clear");
    assert_eq!(environment_total(&env, Skill::Social, lookup(&defs)), 1);
    assert_eq!(environment_total(&env, Skill::Stealth, lookup(&defs)), -1);
}

#[test]
fn weather_stacks_with_the_hour() {
    let defs = registry();
    let env = Environment::new("night", "fog");
    assert_eq!(environment_total(&env, Skill::Stealth, lookup(&defs)), 5);
    assert_eq!(environment_total(&env, Skill::Hacking, lookup(&defs)), -1);
}

#[test]
fn rain_punishes_the_climber() {
    let defs = registry();
    let env = Environment::new("day", "rain");
    assert_eq!(environment_total(&env, Skill::Athletics, lookup(&defs)), -2);
}

#[test]
fn an_all_skills_factor_hits_everything() {
    let defs = registry();
    let env = Environment::new("clear", "clear").with_factors(vec!["high_security".to_owned()]);
    for skill in Skill::ALL {
        assert!(environment_total(&env, skill, lookup(&defs)) <= -3);
    }
}

#[test]
fn entries_are_named_and_ordered_time_weather_then_factors() {
    let defs = registry();
    let env = Environment::new("night", "fog").with_factors(vec!["high_security".to_owned()]);
    let entries = environment_entries(&env, Skill::Stealth, lookup(&defs));

    let labels: Vec<&str> = entries.iter().map(|e| e.label.as_str()).collect();
    assert_eq!(labels, vec!["Night", "Fog", "High Security"]);
}

#[test]
fn zero_modifiers_are_left_out_of_the_breakdown() {
    let defs = registry();
    let env = Environment::new("night", "clear");
    let entries = environment_entries(&env, Skill::Combat, lookup(&defs));
    assert!(entries.is_empty());
}

#[test]
fn an_unauthored_factor_is_reported_rather_than_silently_ignored() {
    let defs = registry();
    let env = Environment::new("night", "clear").with_factors(vec!["moonquake".to_owned()]);
    assert_eq!(unknown_ids(&env, lookup(&defs)), vec!["clear", "moonquake"]);
}
