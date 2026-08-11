use super::*;
use crate::data::GameData;

fn setup(seed: u64) -> (GameData, GameSession) {
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config, &data, seed);
    (data, session)
}

#[test]
fn a_week_of_doing_nothing_still_costs_the_outfit_money() {
    let (data, mut session) = setup(1);
    let before = session.budget;

    let outcome = settle_payroll(&mut session, &data.config);

    assert!(outcome.total_due() > 0, "the outfit is free to keep");
    assert_eq!(session.budget, before - outcome.paid);
    assert!(!outcome.was_short(), "week one should be affordable");
}

#[test]
fn a_bigger_name_and_a_better_crew_both_cost_more() {
    let (data, mut session) = setup(2);
    let lean = weekly_outgoings(&session, &data.config.payroll);

    session.reputation = 50;
    let famous = weekly_outgoings(&session, &data.config.payroll);
    assert!(famous > lean, "reputation is free to carry");

    session.crew[0].progression.level = 9;
    let expensive = weekly_outgoings(&session, &data.config.payroll);
    assert!(expensive > famous, "a veteran works for a beginner's wage");
}

#[test]
fn a_broke_outfit_misses_wages_and_the_crew_remembers() {
    let (data, mut session) = setup(3);
    session.budget = 0;

    let outcome = settle_payroll(&mut session, &data.config);

    assert!(outcome.was_short());
    assert_eq!(outcome.unpaid.len(), session.crew.len());
    assert!(session.crew.iter().all(|m| m.condition.weeks_unpaid == 1));
    assert!(session.crew.iter().all(|m| m.condition.loyalty < 60));
    assert_eq!(
        session.budget, 0,
        "nobody is paid with money that is not there"
    );
}

#[test]
fn going_unpaid_costs_more_the_longer_it_runs() {
    let (data, mut session) = setup(4);
    session.budget = 0;

    settle_payroll(&mut session, &data.config);
    let after_one = session.crew[0].condition.loyalty;
    settle_payroll(&mut session, &data.config);
    let after_two = session.crew[0].condition.loyalty;

    assert!(
        after_one - after_two > data.config.payroll.unpaid_loyalty_cost,
        "the second missed week hurt no more than the first"
    );
}

#[test]
fn a_sullen_hand_gives_notice_before_they_go() {
    let (data, mut session) = setup(5);
    session.crew[0].condition.loyalty = 10;
    let name = session.crew[0].name.clone();
    let roster = session.crew.len();

    let first = settle_payroll(&mut session, &data.config);
    assert_eq!(first.notices, vec![name.clone()]);
    assert!(first.walkouts.is_empty(), "no warning was given");
    assert_eq!(session.crew.len(), roster);

    session.crew[0].condition.loyalty = 10;
    let second = settle_payroll(&mut session, &data.config);
    assert_eq!(second.walkouts, vec![name]);
    assert_eq!(session.crew.len(), roster - 1);
}

#[test]
fn a_bonus_talks_them_round_and_the_money_is_gone() {
    let (data, mut session) = setup(6);
    session.crew[0].condition.loyalty = 10;
    settle_payroll(&mut session, &data.config);
    assert!(session.crew[0].condition.notice_given);

    let id = session.crew[0].id.clone();
    let cost = bonus_cost(&session.crew[0], &data.config.payroll);
    // A week's payroll leaves the opening budget too thin for a bonus,
    // which is the economy working; this test is about the mechanic.
    session.budget = cost * 4;
    let budget = session.budget;

    assert!(pay_bonus(&mut session, &data.config, &id).is_ok());
    assert_eq!(session.budget, budget - cost);
    assert!(!session.crew[0].condition.notice_given);

    // Talked round, they survive the next payroll instead of walking.
    let after = settle_payroll(&mut session, &data.config);
    assert!(after.walkouts.is_empty());
}

#[test]
fn a_hand_can_be_paid_off_and_the_bill_shrinks() {
    // The trap this closes: a retainer with no way to stop it.
    let (data, mut session) = setup(50);
    session.budget = 5_000_000;
    let id = session.crew[0].id.clone();
    let roster = session.crew.len();
    let before = weekly_outgoings(&session, &data.config.payroll);
    let cost = severance_cost(&session.crew[0], &data.config.payroll);

    assert!(dismiss(&mut session, &data.config, &id).is_ok());

    assert_eq!(session.crew.len(), roster - 1);
    assert!(session.member(&id).is_none());
    assert_eq!(session.budget, 5_000_000 - cost);
    assert!(
        weekly_outgoings(&session, &data.config.payroll) < before,
        "the outfit is still paying somebody who left"
    );
}

#[test]
fn the_rest_of_the_crew_watch_it_happen() {
    let (data, mut session) = setup(51);
    session.budget = 5_000_000;
    let id = session.crew[0].id.clone();
    let watcher = session.crew[1].id.clone();
    let before = session.member(&watcher).unwrap().condition.loyalty;

    dismiss(&mut session, &data.config, &id).unwrap();

    assert_eq!(
        session.member(&watcher).unwrap().condition.loyalty,
        before - data.config.payroll.dismissal_loyalty_cost,
        "clearing house cost nothing in goodwill"
    );
}

#[test]
fn the_last_hand_standing_cannot_be_paid_off() {
    let (data, mut session) = setup(52);
    session.budget = 5_000_000;
    session.crew.truncate(1);
    let id = session.crew[0].id.clone();

    assert!(dismiss(&mut session, &data.config, &id).is_err());
    assert_eq!(session.crew.len(), 1, "the outfit dissolved itself");
}

#[test]
fn a_severance_nobody_can_afford_keeps_them_on_the_books() {
    let (data, mut session) = setup(53);
    session.budget = 0;
    let id = session.crew[0].id.clone();
    let roster = session.crew.len();

    assert!(dismiss(&mut session, &data.config, &id).is_err());
    assert_eq!(session.crew.len(), roster);
}

#[test]
fn a_dismissed_hands_kit_comes_back_to_the_lockup() {
    let (data, mut session) = setup(54);
    session.budget = 5_000_000;
    let id = session
        .crew
        .iter()
        .find(|member| member.equipment.item_ids().count() > 0)
        .expect("somebody carries the starting kit")
        .id
        .clone();
    let shelf = session.unassigned_inventory(&data).len();

    dismiss(&mut session, &data.config, &id).unwrap();

    assert!(
        session.unassigned_inventory(&data).len() > shelf,
        "their tools left with them"
    );
}

#[test]
fn a_bonus_nobody_can_afford_changes_nothing() {
    let (data, mut session) = setup(7);
    let id = session.crew[0].id.clone();
    session.budget = 0;

    assert!(pay_bonus(&mut session, &data.config, &id).is_err());
    assert_eq!(session.budget, 0);
}

#[test]
fn taking_somebody_on_shortens_the_runway_twice_over() {
    // Once for the fee out of the purse, and again for every week of their
    // retainer after it. The hiring screen used to show only the first.
    let (data, mut session) = setup(40);
    let payroll = &data.config.payroll;
    let recruit = data
        .crew_pool
        .get(session.recruits.first().expect("somebody is asking"))
        .unwrap();
    session.budget = 400_000;

    let before = weeks_of_runway(&session, payroll);
    let fee = session.hire_fee(recruit, &data.config);
    let after = runway_after_hiring(&session, payroll, recruit, fee);
    assert!(after < before, "a new hand paid for themselves");

    // And the fee alone does not explain it: the retainer is the larger
    // half of the cost over any campaign worth playing.
    let fee_only = (session.budget - fee) / weekly_outgoings(&session, payroll);
    assert!(
        after < fee_only,
        "the weekly cost of keeping them was not counted"
    );
}

#[test]
fn a_hand_the_outfit_cannot_keep_shows_a_runway_of_nothing() {
    let (data, mut session) = setup(41);
    let recruit = data
        .crew_pool
        .get(session.recruits.first().unwrap())
        .unwrap();
    session.budget = recruit.hire_cost;
    let fee = session.hire_fee(recruit, &data.config);

    assert_eq!(
        runway_after_hiring(&session, &data.config.payroll, recruit, fee),
        0,
        "signing them emptied the purse and the screen should say so"
    );
}

#[test]
fn a_bigger_crew_wants_a_bigger_share() {
    let (data, session) = setup(20);
    let one = crew_cut(&session, &data.config, &session.crew_ids()[..1]);
    let all = crew_cut(&session, &data.config, &session.crew_ids());

    assert!(all.share > one.share, "extra hands were free");
    assert!(all
        .reasons
        .iter()
        .any(|reason| reason.label.contains("splitting it")));
}

#[test]
fn a_sullen_hand_holds_out_and_a_steady_one_does_not_haggle() {
    let (data, mut session) = setup(21);
    let ids = session.crew_ids();
    let base = crew_cut(&session, &data.config, &ids).share;

    session.crew[0].condition.loyalty = data.config.cut.holdout_loyalty;
    let holding = crew_cut(&session, &data.config, &ids);
    assert!(holding.share > base, "discontent came free");
    assert!(holding
        .reasons
        .iter()
        .any(|reason| reason.label.contains("holding out")));

    for member in &mut session.crew {
        member.condition.loyalty = 100;
    }
    let steady = crew_cut(&session, &data.config, &ids);
    assert!(steady.share < base, "goodwill bought nothing");
}

#[test]
fn a_pair_who_work_as_one_charge_as_one() {
    // GDD 5.5's missing half: a good partnership improves the odds at every
    // door, and now it has a price, so keeping it together is a decision.
    let (data, mut session) = setup(25);
    let ids = session.crew_ids();
    let base = crew_cut(&session, &data.config, &ids).share;

    session.chemistry.set(&ids[0], &ids[1], 80);
    let paired = crew_cut(&session, &data.config, &ids);

    assert!(paired.share > base, "a partnership cost nothing extra");
    assert!(paired
        .reasons
        .iter()
        .any(|reason| reason.label.contains("work as a pair")));
}

#[test]
fn splitting_the_pair_up_is_the_cheaper_roster() {
    let (data, mut session) = setup(26);
    let ids = session.crew_ids();
    session.chemistry.set(&ids[0], &ids[1], 90);

    let together = crew_cut(&session, &data.config, &ids[..2]);
    let apart = crew_cut(&session, &data.config, &ids[..1]);

    assert!(
        apart.share < together.share,
        "the pair was free to keep together"
    );
}

#[test]
fn a_crew_who_did_the_thinking_charge_for_the_thinking() {
    // Pillar 5: delegation is a discount, not a shortcut. It was neither —
    // the same assignment for the same money, one click sooner.
    let (data, session) = setup(30);
    let ids = session.crew_ids();

    let planned = crew_cut_for(&session, &data.config, &ids, false);
    let delegated = crew_cut_for(&session, &data.config, &ids, true);

    assert!(delegated.share > planned.share, "delegating was free");
    assert!(delegated
        .reasons
        .iter()
        .any(|reason| reason.label.contains("planned it themselves")));
}

#[test]
fn nobody_charges_for_planning_a_job_with_no_crew_on_it() {
    let (data, session) = setup(31);
    let cut = crew_cut_for(&session, &data.config, &[], true);
    assert_eq!(cut.share, data.config.cut.base_share);
}

#[test]
fn the_share_never_leaves_its_bounds_however_bad_the_roster_is() {
    let (data, mut session) = setup(22);
    let ids = session.crew_ids();
    for member in &mut session.crew {
        member.condition.loyalty = 0;
    }

    let cut = crew_cut(&session, &data.config, &ids);
    assert!(cut.share <= data.config.cut.max_share);
    assert!(cut.share >= data.config.cut.min_share);
}

#[test]
fn the_cut_is_arithmetic_the_player_can_check() {
    let (data, session) = setup(23);
    let cut = crew_cut(&session, &data.config, &session.crew_ids());

    assert_eq!(cut.take_of(1_000_000) + cut.net_of(1_000_000), 1_000_000);
    assert!((cut.percent() - cut.share * 100.0).abs() < f32::EPSILON);
}

#[test]
fn nobody_on_the_job_means_nobody_to_pay() {
    let (data, session) = setup(24);
    let cut = crew_cut(&session, &data.config, &[]);

    assert_eq!(cut.share, data.config.cut.base_share);
    assert!(cut.reasons.is_empty());
}

#[test]
fn the_runway_is_how_many_quiet_weeks_the_outfit_can_buy() {
    let (data, mut session) = setup(8);
    let due = weekly_outgoings(&session, &data.config.payroll);
    session.budget = due * 3;

    assert_eq!(weeks_of_runway(&session, &data.config.payroll), 3);
    session.budget = 0;
    assert_eq!(weeks_of_runway(&session, &data.config.payroll), 0);
}
