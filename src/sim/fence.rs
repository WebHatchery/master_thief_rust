//! Turning kit back into money.
//!
//! Every other system built into this week takes money out: casing, hiring,
//! buying, bribes, bail, bonuses, the doctor, the bench. There is exactly one
//! way in, and it is running a job. That asymmetry meant a fixer who could not
//! make payroll had precisely one answer, and it was to take work they had
//! already decided against.
//!
//! Meanwhile nothing ever left the lockup. Loot came out of every job and
//! accumulated forever, so a long campaign ended up sitting on a pile of
//! unusable tools worth nothing to anybody.
//!
//! The fence answers both. Spare kit sells, at a fraction of what it cost, and
//! the fraction falls as the city's attention rises — nobody wants to be seen
//! doing business with an outfit everyone is watching. Each sale adds a little
//! heat of its own, so selling your way out of a bad week makes the next one
//! worse. It is the one lever that points the other way, and it is priced like
//! it knows that.
//!
//! No RNG: what a thing fetches is arithmetic the player reads before agreeing.

use crate::data::{FenceConfig, GameConfig, GameData};
use crate::state::GameSession;
use macroquad_toolkit::ui::format_money;

/// What a fence will give for one piece, and how badly the outfit's reputation
/// for being watched is hurting the price.
#[derive(Debug, Clone, PartialEq)]
pub struct Offer {
    pub item_id: String,
    pub name: String,
    /// What it cost new.
    pub worth: i64,
    pub price: i64,
    /// Share of the original price, 0..1.
    pub share: f32,
    /// True when heat has pushed the offer down to the floor.
    pub squeezed: bool,
}

/// The share of an item's price a fence will pay at the outfit's current heat.
pub fn share_at(heat: i32, config: &FenceConfig) -> f32 {
    let squeezed = config.share_base - heat.max(0) as f32 * config.share_heat_penalty;
    squeezed.max(config.min_share)
}

/// What this piece would fetch today.
pub fn quote(
    session: &GameSession,
    item: &crate::model::EquipmentDef,
    config: &FenceConfig,
) -> Offer {
    let share = share_at(session.heat, config);
    Offer {
        item_id: item.id.clone(),
        name: item.name.clone(),
        worth: item.cost,
        price: (item.cost as f32 * share) as i64,
        share,
        squeezed: share <= config.min_share,
    }
}

/// Sell one piece out of the lockup. Only kit nobody is carrying: the tool in
/// somebody's hand is not the outfit's to sell out from under them.
pub fn sell(
    session: &mut GameSession,
    data: &GameData,
    config: &GameConfig,
    item_id: &str,
) -> Result<String, String> {
    let Some(item) = data.equipment.get(item_id) else {
        return Err("Nobody deals in that".to_owned());
    };
    if !session
        .unassigned_inventory(data)
        .iter()
        .any(|def| def.id == item.id)
    {
        return Err(format!(
            "{} is on somebody — take it off them first",
            item.name
        ));
    }

    let offer = quote(session, item, &config.fence);
    let Some(index) = session.inventory.iter().position(|id| id == item_id) else {
        return Err(format!("{} is not in the lockup", item.name));
    };
    session.inventory.remove(index);

    // A tool the outfit no longer owns any of has no wear worth remembering.
    if !session.inventory.iter().any(|id| id == item_id) {
        session.kit_wear.remove(item_id);
    }

    session.budget += offer.price;
    session.heat += config.fence.heat_per_sale;
    session.tally.fenced_total += offer.price;
    session.tally.items_fenced += 1;

    Ok(format!(
        "{} sold on for {} ({:.0}% of list)",
        offer.name,
        format_money(offer.price),
        offer.share * 100.0
    ))
}

#[cfg(test)]
mod tests;
