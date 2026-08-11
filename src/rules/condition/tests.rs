use super::super::outcome::Outcome;
use super::*;

/// A real dossier out of the content, so the entries under test are built
/// the same way the game builds them.
fn fixture() -> CrewMember {
    let data = crate::data::GameData::load().unwrap();
    let id = data.crew_pool.ids().next().expect("a crew pool").clone();
    data.crew_pool.get(&id).expect("a dossier").clone()
}

#[test]
fn loyalty_grades_all_the_way_down_rather_than_falling_off_a_cliff() {
    // Two flat bands made loyalty a switch. Graded, every point of goodwill
    // the outfit burns is worth something on the die.
    let tuning = ConditionTuning::default();
    let steps: Vec<i32> = [100, 80, 60, 40, 20, 0]
        .into_iter()
        .map(|loyalty| tuning.loyalty_modifier(loyalty))
        .collect();

    assert_eq!(steps, vec![2, 1, 0, -1, -2, -3]);
    for pair in steps.windows(2) {
        assert!(pair[0] >= pair[1], "the curve doubles back: {:?}", steps);
    }
}

#[test]
fn the_grading_never_runs_past_its_own_bounds() {
    let tuning = ConditionTuning::default();
    assert_eq!(tuning.loyalty_modifier(1_000), tuning.loyalty_max_bonus);
    assert_eq!(tuning.loyalty_modifier(-1_000), -tuning.loyalty_max_penalty);
    assert_eq!(tuning.fatigue_modifier(0), 0);
    assert_eq!(tuning.fatigue_modifier(tuning.fatigue_free_threshold), 0);
}

#[test]
fn tiredness_is_a_price_and_injury_is_a_wall() {
    use crate::model::crew::{Condition, Injury};
    let tuning = ConditionTuning::default();
    let mut condition = Condition {
        fatigue: 100,
        ..Condition::default()
    };

    assert!(
        tuning.can_work(&condition),
        "exhaustion took the choice away from the fixer"
    );
    assert!(tuning.is_spent(condition.fatigue));

    condition.injuries = (0..tuning.max_injuries_for_work + 1)
        .map(|n| Injury::major(format!("Hurt {}", n)))
        .collect();
    assert!(!tuning.can_work(&condition));
}

#[test]
fn being_spent_is_its_own_named_line_and_not_more_fatigue() {
    // Two separate costs on purpose: the graded curve says how tired
    // somebody is, and this says the fixer is choosing to send them anyway.
    use crate::model::crew::Condition;
    let tuning = ConditionTuning::default();
    let below = Condition {
        fatigue: tuning.fatigue_work_threshold,
        ..Condition::default()
    };
    let above = Condition {
        fatigue: tuning.fatigue_work_threshold + 1,
        ..Condition::default()
    };

    let named = |condition: Condition| {
        let member = crate::model::CrewMember {
            condition,
            ..fixture()
        };
        condition_entries(&member, &tuning)
            .into_iter()
            .any(|entry| entry.label == "Running on empty")
    };

    assert!(!named(below));
    assert!(named(above));
}

#[test]
fn a_tired_hand_gets_hurt_easier_at_every_door_but_a_flawless_one() {
    // The half of the price that never reaches the breakdown, which is why
    // the planning screen has to say it in words instead.
    let tuning = ConditionTuning::default();

    for outcome in Outcome::ALL {
        let rested = tuning.injury_chance(outcome, false);
        let spent = tuning.injury_chance(outcome, true);
        assert_eq!(rested, outcome.injury_chance());

        if outcome == Outcome::CriticalSuccess {
            assert_eq!(spent, rested, "a flawless door charged a tired hand");
        } else {
            assert!(
                spent > rested,
                "{:?}: {} against {}",
                outcome,
                spent,
                rested
            );
        }
        assert!(spent <= 1.0);
    }
}

#[test]
fn a_tuning_with_no_step_in_it_refuses_to_divide_by_nothing() {
    let flat = ConditionTuning {
        loyalty_step: 0,
        fatigue_step: 0,
        ..ConditionTuning::default()
    };
    assert_eq!(flat.loyalty_modifier(0), 0);
    assert_eq!(flat.fatigue_modifier(100), 0);
}
