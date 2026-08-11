use super::*;

fn tuning() -> ScrutinyTuning {
    ScrutinyTuning::default()
}

#[test]
fn one_job_is_never_enough_to_harden_a_city() {
    // The free threshold is the whole reason this is a habit and not a tax:
    // a crew who work a trade once should read exactly the same as a crew
    // who never touched it.
    let tuning = tuning();
    let mut scrutiny = Scrutiny::default();
    scrutiny.note(Skill::Hacking, tuning.per_door_cleared, &tuning);

    assert_eq!(scrutiny.penalty(Skill::Hacking, &tuning), 0);
    assert!(scrutiny.entry(Skill::Hacking, &tuning).is_none());
    assert!(scrutiny.watched(&tuning).is_empty());
}

#[test]
fn working_the_same_trade_week_after_week_is_what_costs() {
    let tuning = tuning();
    let mut scrutiny = Scrutiny::default();
    for _ in 0..4 {
        scrutiny.note(Skill::Stealth, tuning.per_door_cleared, &tuning);
    }

    assert!(scrutiny.penalty(Skill::Stealth, &tuning) > 0);
    let entry = scrutiny.entry(Skill::Stealth, &tuning).unwrap();
    assert_eq!(entry.label, "Watched: Stealth");
    assert!(entry.value < 0);
    assert_eq!(scrutiny.penalty(Skill::Lockpicking, &tuning), 0);
}

#[test]
fn the_file_stops_growing_at_the_top_of_the_curve() {
    let tuning = tuning();
    let mut scrutiny = Scrutiny::default();
    for _ in 0..200 {
        scrutiny.note(Skill::Combat, tuning.per_door_cleared, &tuning);
    }

    assert_eq!(scrutiny.get(Skill::Combat), tuning.ceiling());
    assert_eq!(scrutiny.penalty(Skill::Combat, &tuning), tuning.max_penalty);
}

#[test]
fn a_maxed_out_file_still_goes_cold_on_a_countable_schedule() {
    // Capping the attention rather than the penalty is what makes this
    // true: without it the player would sit at -3 for an unknowable number
    // of weeks while an invisible number drained.
    let tuning = tuning();
    let mut scrutiny = Scrutiny::default();
    for _ in 0..200 {
        scrutiny.note(Skill::Social, tuning.per_door_cleared, &tuning);
    }

    // Every band the file falls through was quoted before it fell, so a
    // player deciding whether to lie low is deciding on real numbers.
    let mut weeks = 0;
    while scrutiny.penalty(Skill::Social, &tuning) > 0 {
        let quoted = tuning.weeks_to_relief(scrutiny.get(Skill::Social));
        assert!(quoted > 0);
        let band = scrutiny.penalty(Skill::Social, &tuning);

        for _ in 0..quoted {
            scrutiny.cool(&tuning);
            weeks += 1;
        }
        assert_eq!(scrutiny.penalty(Skill::Social, &tuning), band - 1);
        assert!(weeks < 100, "a file that never went cold");
    }

    assert!(weeks > 0);
    assert_eq!(tuning.weeks_to_relief(scrutiny.get(Skill::Social)), 0);
}

#[test]
fn cooling_reports_only_the_weeks_the_player_can_feel() {
    // A week that shed attention without moving the penalty is not news.
    let tuning = ScrutinyTuning {
        decay_per_week: 1,
        ..tuning()
    };
    let mut scrutiny = Scrutiny::default();
    // Just under the second band, so there is a whole step of thinning to
    // do before the die notices.
    let start = tuning.free_threshold + 2 * tuning.step - 1;
    scrutiny.note(Skill::Athletics, start, &tuning);
    assert_eq!(scrutiny.penalty(Skill::Athletics, &tuning), 1);

    for week in 0..tuning.step - 1 {
        assert!(
            scrutiny.cool(&tuning).is_empty(),
            "week {} reported relief the player could not feel",
            week
        );
        assert_eq!(scrutiny.penalty(Skill::Athletics, &tuning), 1);
    }

    assert_eq!(scrutiny.cool(&tuning), vec![Skill::Athletics]);
    assert_eq!(scrutiny.penalty(Skill::Athletics, &tuning), 0);
    assert!(scrutiny.cool(&tuning).is_empty());
}

#[test]
fn the_watch_list_is_ordered_worst_first_and_never_shuffles() {
    let tuning = tuning();
    let mut scrutiny = Scrutiny::default();
    scrutiny.note(Skill::Hacking, tuning.ceiling(), &tuning);
    scrutiny.note(Skill::Stealth, tuning.free_threshold + tuning.step, &tuning);
    scrutiny.note(Skill::Combat, tuning.free_threshold, &tuning);

    let watched = scrutiny.watched(&tuning);
    assert_eq!(
        watched,
        vec![(Skill::Hacking, tuning.max_penalty), (Skill::Stealth, 1)]
    );
}

#[test]
fn a_tuning_with_no_step_in_it_refuses_to_divide_by_nothing() {
    let flat = ScrutinyTuning {
        step: 0,
        decay_per_week: 0,
        ..tuning()
    };
    assert_eq!(flat.penalty_for(1_000), 0);
    assert_eq!(flat.weeks_to_relief(1_000), 0);
}

#[test]
fn a_file_survives_the_save() {
    let tuning = tuning();
    let mut scrutiny = Scrutiny::default();
    scrutiny.note(Skill::Lockpicking, tuning.ceiling(), &tuning);

    let encoded = serde_json::to_string(&scrutiny).unwrap();
    let restored: Scrutiny = serde_json::from_str(&encoded).unwrap();
    assert_eq!(restored, scrutiny);
    assert_eq!(restored.get(Skill::Lockpicking), tuning.ceiling());
}
