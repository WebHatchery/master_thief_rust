use super::*;

fn setup(seed: u64) -> (GameData, GameSession) {
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config, &data, seed);
    (data, session)
}

/// Put a spare on the shelf that nobody is carrying, and that the outfit
/// does not already own a copy of. `DataRegistry::ids()` has no defined
/// order — the rest of the codebase sorts before drawing from it — so the
/// pick has to be made deterministic here or the test is a coin toss.
fn shelve(session: &mut GameSession, data: &GameData) -> String {
    let mut ids: Vec<&String> = data.equipment.ids().collect();
    ids.sort();
    let id = ids
        .into_iter()
        .find(|id| !session.inventory.contains(id))
        .expect("the catalogue is wider than the starting kit")
        .clone();
    session.inventory.push(id.clone());
    id
}

#[test]
fn a_quiet_outfit_gets_the_best_price_there_is() {
    let (data, session) = setup(1);
    let config = &data.config.fence;
    let item = data
        .equipment
        .get(&shelve(&mut session.clone(), &data))
        .unwrap();

    assert_eq!(share_at(session.heat, config), config.share_base);
    assert!(config.share_base < 1.0, "a fence is not a charity");
    assert!(quote(&session, item, config).price < item.cost);
}

#[test]
fn a_watched_outfit_gets_squeezed_and_then_hits_a_floor() {
    let (data, _) = setup(2);
    let config = &data.config.fence;

    let quiet = share_at(0, config);
    let warm = share_at(40, config);
    assert!(warm < quiet, "heat cost the outfit nothing at the fence");
    assert_eq!(share_at(100_000, config), config.min_share);
}

#[test]
fn selling_puts_money_in_and_takes_the_piece_out() {
    let (data, mut session) = setup(3);
    let id = shelve(&mut session, &data);
    let budget = session.budget;
    let held = session.inventory.len();

    assert!(sell(&mut session, &data, &data.config, &id).is_ok());

    assert!(session.budget > budget, "the fence paid nothing");
    assert_eq!(session.inventory.len(), held - 1);
}

#[test]
fn selling_is_never_a_way_to_get_richer_than_buying() {
    let (data, mut session) = setup(4);
    let id = shelve(&mut session, &data);
    let cost = data.equipment.get(&id).unwrap().cost;
    let budget = session.budget;

    sell(&mut session, &data, &data.config, &id).unwrap();
    assert!(
        session.budget - budget < cost,
        "buy-and-sell was a money printer"
    );
}

#[test]
fn every_sale_costs_a_little_anonymity() {
    let (data, mut session) = setup(5);
    let id = shelve(&mut session, &data);
    let heat = session.heat;

    sell(&mut session, &data, &data.config, &id).unwrap();
    assert_eq!(session.heat, heat + data.config.fence.heat_per_sale);
}

#[test]
fn a_tool_somebody_is_carrying_is_not_the_outfits_to_sell() {
    let (data, mut session) = setup(6);
    let carried = session
        .crew
        .iter()
        .find_map(|member| member.equipment.item_ids().next().map(|id| id.to_owned()))
        .expect("somebody carries the starting kit");
    let budget = session.budget;

    assert!(sell(&mut session, &data, &data.config, &carried).is_err());
    assert_eq!(session.budget, budget);
}

#[test]
fn selling_the_last_of_something_forgets_how_worn_it_was() {
    let (data, mut session) = setup(7);
    let id = shelve(&mut session, &data);
    session.kit_wear.insert(id.clone(), 9);

    sell(&mut session, &data, &data.config, &id).unwrap();
    assert!(
        !session.kit_wear.contains_key(&id),
        "a tool the outfit no longer owns kept its wear"
    );
}

#[test]
fn a_spare_kept_on_the_shelf_keeps_its_history() {
    let (data, mut session) = setup(8);
    let id = shelve(&mut session, &data);
    // A second copy of the *same* piece, which is what the rule is about.
    session.inventory.push(id.clone());
    session.kit_wear.insert(id.clone(), 4);

    sell(&mut session, &data, &data.config, &id).unwrap();
    assert_eq!(session.kit_wear.get(&id), Some(&4), "the other copy forgot");
}
