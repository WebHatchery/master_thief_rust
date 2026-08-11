use super::*;
use crate::model::crew::Injury;
use crate::sim::law::LawEventKind;
use crate::sim::payroll::weekly_outgoings;

fn setup(seed: u64) -> (GameData, GameSession) {
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config, &data, seed);
    (data, session)
}

#[test]
fn a_week_of_rest_sheds_fatigue() {
    let (data, mut session) = setup(1);
    session.crew[0].condition.fatigue = 70;

    let summary = advance_week(&mut session, &data);
    assert!(session.crew[0].condition.fatigue < 70);
    assert!(summary.fatigue_shed > 0);
    assert_eq!(summary.week, 2);
}

#[test]
fn injuries_clear_on_their_own_schedule_not_instantly() {
    let (data, mut session) = setup(2);
    session.crew[0]
        .condition
        .injuries
        .push(Injury::major("Torn shoulder"));

    advance_week(&mut session, &data);
    assert_eq!(session.crew[0].condition.injuries.len(), 1);

    advance_week(&mut session, &data);
    advance_week(&mut session, &data);
    assert!(session.crew[0].condition.injuries.is_empty());
}

#[test]
fn lying_low_cools_the_city() {
    let (data, mut session) = setup(3);
    session.heat = 40;

    let summary = advance_week(&mut session, &data);
    assert_eq!(session.heat, 40 - data.config.heat_decay_per_week);
    assert_eq!(summary.heat_shed, data.config.heat_decay_per_week);
}

#[test]
fn heat_never_goes_below_nothing() {
    let (data, mut session) = setup(4);
    session.heat = 2;

    advance_week(&mut session, &data);
    assert_eq!(session.heat, 0);
}

#[test]
fn the_board_keeps_itself_stocked_as_marks_expire() {
    let (data, mut session) = setup(5);
    for _ in 0..6 {
        advance_week(&mut session, &data);
    }
    assert!(!session.board.is_empty());
}

#[test]
fn a_hurt_crew_loses_faith_while_a_rested_one_settles() {
    let (data, mut session) = setup(6);
    session.crew[0].condition.loyalty = 50;
    session.crew[1]
        .condition
        .injuries
        .push(Injury::major("Broken hand"));
    session.crew[1].condition.loyalty = 50;

    advance_week(&mut session, &data);
    assert!(session.crew[0].condition.loyalty > 50);
    assert!(session.crew[1].condition.loyalty < 50);
}

#[test]
fn a_quiet_week_costs_the_outfit_a_weeks_pay() {
    // The whole point of the change: "rest until everyone is fresh" is now
    // a purchase, and the fixer can read the price.
    let (data, mut session) = setup(7);
    let due = weekly_outgoings(&session, &data.config.payroll);
    let budget = session.budget;

    let summary = advance_week(&mut session, &data);

    assert!(due > 0);
    assert_eq!(summary.payroll.total_due(), due);
    assert_eq!(session.budget, budget - due);
    assert!(summary.ledger_line().contains("paid out"));
}

#[test]
fn an_outfit_that_never_works_runs_itself_into_the_ground() {
    // Twenty weeks of lying low used to leave a rested crew and a full
    // wallet. It should now leave neither.
    let (data, mut session) = setup(8);

    for _ in 0..20 {
        advance_week(&mut session, &data);
    }

    assert!(session.budget <= 0, "idling never cost anything");
    assert!(
        session.crew.len() < 3 || session.crew.iter().any(|m| m.condition.notice_given),
        "twenty unpaid weeks and the crew stayed cheerful"
    );
}

#[test]
fn a_missed_payroll_is_reported_rather_than_swallowed() {
    let (data, mut session) = setup(9);
    session.budget = 0;

    let summary = advance_week(&mut session, &data);

    assert!(summary.payroll.was_short());
    assert!(summary.ledger_line().contains("short by"));
}

#[test]
fn a_hot_week_can_end_with_somebody_in_a_cell() {
    let (data, mut session) = setup(10);
    session.budget = 5_000_000;

    let mut arrested = false;
    for _ in 0..60 {
        session.heat = data.config.law.custody_threshold + 30;
        let summary = advance_week(&mut session, &data);
        if summary
            .law
            .as_ref()
            .is_some_and(|event| event.kind == LawEventKind::Arrest)
        {
            arrested = true;
            break;
        }
    }

    assert!(arrested, "sixty of the hottest weeks and nobody was taken");
    assert_eq!(session.custody.len(), 1);
    assert!(session
        .custody
        .first()
        .is_some_and(|record| record.bail > 0));
}

#[test]
fn a_pair_the_fixer_stops_using_goes_off_the_boil() {
    let (data, mut session) = setup(12);
    let ids = session.crew_ids();
    session.chemistry.set(&ids[0], &ids[1], 60);

    // A quiet week is a week nobody stood in a building together.
    advance_week(&mut session, &data);

    assert_eq!(
        session.chemistry.get(&ids[0], &ids[1]),
        60 - data.config.chemistry_cooling,
        "warmth survived a week of nobody working"
    );
}

#[test]
fn a_pair_who_worked_together_keep_what_they_built() {
    let (data, mut session) = setup(13);
    let ids = session.crew_ids();
    session.chemistry.set(&ids[0], &ids[1], 60);
    session.chemistry.set(&ids[0], &ids[2], 60);

    // Mark two of them as having worked, and file a job for this week so
    // the week knows the outfit was out.
    session.crew[0].condition.worked_this_week = true;
    session.crew[1].condition.worked_this_week = true;
    session.history.push(crate::state::JobRecord {
        week: session.week,
        target_name: "Somewhere".to_owned(),
        difficulty: crate::model::DifficultyBand::Easy,
        success: true,
        doors_passed: 1,
        doors_total: 1,
        payout: 0,
        delegated: false,
        reputation: 0,
        notoriety: 0,
    });

    advance_week(&mut session, &data);

    assert_eq!(session.chemistry.get(&ids[0], &ids[1]), 60);
    assert_eq!(
        session.chemistry.get(&ids[0], &ids[2]),
        60 - data.config.chemistry_cooling
    );
    assert!(
        session.crew.iter().all(|m| !m.condition.worked_this_week),
        "the week did not reset who worked"
    );
}

#[test]
fn a_mark_left_to_ripen_can_be_taken_by_somebody_else() {
    // The whole reason rivals exist: ripening made waiting profitable and
    // perfectly calculable. This is the part that cannot be calculated.
    let (data, mut session) = setup(14);
    session.budget = 50_000_000;
    let mut lost = 0;

    for _ in 0..60 {
        for entry in &mut session.board {
            entry.ripeness = data.config.board.ripeness_max;
            entry.weeks_remaining = 9;
        }
        if advance_week(&mut session, &data).rival.is_some() {
            lost += 1;
        }
    }

    assert!(lost > 0, "sixty ripe weeks and nobody else took anything");
    assert_eq!(session.tally.marks_lost_to_rivals, lost as i64);
}

#[test]
fn a_board_nobody_is_sitting_on_is_never_poached() {
    // Rivals answer hesitation, not existence. A crew who take their work
    // promptly should never meet one.
    let (data, mut session) = setup(15);
    session.budget = 50_000_000;

    for _ in 0..40 {
        for entry in &mut session.board {
            entry.ripeness = 0;
        }
        assert!(advance_week(&mut session, &data).rival.is_none());
    }
    assert_eq!(session.tally.marks_lost_to_rivals, 0);
}

#[test]
fn a_quiet_week_thins_the_citys_file_on_how_the_outfit_works() {
    // The only thing that answers scrutiny. Heat has three answers and two
    // of them are purchases; a reputation for a method has one, and it is
    // the week itself (GDD 5.4).
    use crate::model::Skill;
    let (data, mut session) = setup(16);
    let tuning = &data.config.scrutiny;
    session
        .scrutiny
        .note(Skill::Hacking, tuning.ceiling(), tuning);
    let before = session.scrutiny.get(Skill::Hacking);

    advance_week(&mut session, &data);

    assert_eq!(
        session.scrutiny.get(Skill::Hacking),
        before - tuning.decay_per_week
    );
}

#[test]
fn the_week_says_when_a_trade_has_come_off_the_list() {
    use crate::model::Skill;
    let (data, mut session) = setup(17);
    let tuning = &data.config.scrutiny;
    // Exactly onto the first band, so a single week's decay clears it.
    session
        .scrutiny
        .note(Skill::Social, tuning.free_threshold + tuning.step, tuning);
    assert_eq!(session.scrutiny.penalty(Skill::Social, tuning), 1);

    let summary = advance_week(&mut session, &data);

    assert_eq!(summary.trades_cooled, vec![Skill::Social]);
    assert!(summary
        .notes()
        .iter()
        .any(|note| note.contains("stopped watching social")));
}

#[test]
fn a_file_that_only_thinned_is_not_reported_as_relief() {
    // Attention drains every week; the player only hears about it on the
    // week it changes a die. Anything else is noise in the notifications.
    use crate::model::Skill;
    let (data, mut session) = setup(18);
    let tuning = &data.config.scrutiny;
    session
        .scrutiny
        .note(Skill::Combat, tuning.ceiling(), tuning);

    let first = advance_week(&mut session, &data);
    let band = session.scrutiny.penalty(Skill::Combat, tuning);
    let second = advance_week(&mut session, &data);

    assert_eq!(first.trades_cooled, vec![Skill::Combat]);
    assert!(second.trades_cooled.is_empty());
    assert_eq!(session.scrutiny.penalty(Skill::Combat, tuning), band);
}

#[test]
fn a_tail_runs_out_on_its_own() {
    let (data, mut session) = setup(11);
    session.surveillance_weeks = 2;

    let summary = advance_week(&mut session, &data);
    assert_eq!(summary.surveillance_weeks, 1);
    advance_week(&mut session, &data);
    assert_eq!(session.surveillance_weeks, 0);
}

#[test]
fn the_same_seed_advances_the_same_week() {
    let data = GameData::load().unwrap();
    let mut a = GameSession::new(&data.config, &data, 4242);
    let mut b = GameSession::new(&data.config, &data, 4242);
    a.heat = 90;
    b.heat = 90;

    for _ in 0..8 {
        let left = advance_week(&mut a, &data);
        let right = advance_week(&mut b, &data);
        assert_eq!(left, right);
    }
    assert_eq!(a.budget, b.budget);
    assert_eq!(a.custody.len(), b.custody.len());
}
