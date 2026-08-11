use super::*;

fn setup(seed: u64) -> (GameData, GameSession) {
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config, &data, seed);
    (data, session)
}

fn content(session: &mut GameSession, loyalty: i32) {
    for member in &mut session.crew {
        member.condition.loyalty = loyalty;
    }
}

/// A name that opens more marks than the board can hold. Below that there
/// is nothing to be tipped off *about* — see the test that pins it.
fn known(session: &mut GameSession) {
    session.reputation = 100;
}

#[test]
fn a_sullen_crew_hears_nothing() {
    let (data, mut session) = setup(1);
    content(&mut session, data.config.leads.loyalty_threshold - 1);

    assert_eq!(chance_of_a_lead(&session, &data.config.leads), 0.0);
    for _ in 0..60 {
        assert!(roll_lead(&mut session, &data).is_none());
    }
}

#[test]
fn more_contented_hands_means_more_ears_up_to_a_point() {
    let (data, mut session) = setup(2);
    let config = &data.config.leads;

    content(&mut session, config.loyalty_threshold);
    let few = chance_of_a_lead(&session, config);
    assert!(few > 0.0);

    for _ in 0..40 {
        let mut extra = session.crew[0].clone();
        extra.id = format!("{}_{}", extra.id, session.crew.len());
        session.crew.push(extra);
    }
    assert_eq!(chance_of_a_lead(&session, config), config.max_chance);
}

#[test]
fn a_lead_arrives_off_the_board_with_part_of_its_file_written() {
    let (data, mut session) = setup(3);
    content(&mut session, 100);
    known(&mut session);
    let before: Vec<String> = session
        .board
        .iter()
        .map(|entry| entry.target_id.clone())
        .collect();

    let mut lead = None;
    for _ in 0..200 {
        lead = roll_lead(&mut session, &data);
        if lead.is_some() {
            break;
        }
    }

    let lead = lead.expect("two hundred contented weeks and nobody heard a thing");
    assert!(
        !before.contains(&lead.target_id),
        "the lead was already on the board"
    );
    assert!(lead.doors_on_file > 0);

    let entry = session.board_entry(&lead.target_id).expect("it was added");
    assert_eq!(entry.casing, lead.doors_on_file);
    assert!(entry.weeks_remaining > 4, "a lead gets a longer window");
    assert!(lead.headline().contains(&lead.finder));
}

#[test]
fn a_lead_is_never_a_mark_the_outfits_name_cannot_open() {
    let (data, mut session) = setup(4);
    content(&mut session, 100);
    known(&mut session);

    for _ in 0..200 {
        if let Some(lead) = roll_lead(&mut session, &data) {
            let target = data.targets.get(&lead.target_id).unwrap();
            assert!(target.required_reputation <= session.reputation);
        }
    }
}

#[test]
fn the_happiest_hand_is_the_one_who_heard() {
    let (data, mut session) = setup(5);
    content(&mut session, data.config.leads.loyalty_threshold);
    known(&mut session);
    session.crew[1].condition.loyalty = 100;
    let expected = session.crew[1].name.clone();

    for _ in 0..200 {
        if let Some(lead) = roll_lead(&mut session, &data) {
            assert_eq!(lead.finder, expected);
            return;
        }
    }
    panic!("no lead ever came in");
}

#[test]
fn there_is_nothing_to_hear_when_the_board_already_shows_everything() {
    // At week one the outfit's name opens three marks and the board holds
    // five, so the board *is* the city. A tip-off needs somewhere to point
    // that the player cannot already see — this is behaviour, not a gap.
    let (data, mut session) = setup(6);
    content(&mut session, 100);
    assert!(chance_of_a_lead(&session, &data.config.leads) > 0.0);

    for _ in 0..80 {
        assert!(
            roll_lead(&mut session, &data).is_none(),
            "a lead pointed at a mark already on the board"
        );
    }
}

#[test]
fn the_same_seed_hears_the_same_things() {
    let data = GameData::load().unwrap();
    let mut a = GameSession::new(&data.config, &data, 808);
    let mut b = GameSession::new(&data.config, &data, 808);
    content(&mut a, 100);
    content(&mut b, 100);
    known(&mut a);
    known(&mut b);

    for _ in 0..30 {
        assert_eq!(roll_lead(&mut a, &data), roll_lead(&mut b, &data));
    }
}
