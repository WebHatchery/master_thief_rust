//! Work the crew bring in themselves.
//!
//! Loyalty does three things, and all three of them are threats: it moves the
//! die, it decides who gives notice, and it decides who holds out for a bigger
//! cut. So the only reason to keep a hand happy is to stop something bad, and
//! "stop something bad" is a weaker pull than it looks — a fixer with a thin
//! week will always find something more urgent than goodwill.
//!
//! A tip-off is the other direction. A hand who is genuinely content hears
//! things: a cousin who works nights somewhere, a room somebody mentioned. The
//! mark arrives off the board, with part of its file already written, because
//! the person who brought it knows the place. It is the one thing in the week
//! that a good roster *generates* rather than merely survives.
//!
//! It also makes the board partly a function of who the outfit employs, where
//! before it was entirely a function of the seed.
//!
//! One draw per week from the session RNG, at a fixed point in
//! [`crate::sim::advance_week`] (GDD 5.7).

use crate::data::{GameData, LeadConfig};
use crate::state::{BoardEntry, GameSession};

/// A mark somebody on the payroll brought in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lead {
    /// Who heard about it.
    pub finder: String,
    pub target_id: String,
    pub target_name: String,
    /// Doors already on the file, because they know the place.
    pub doors_on_file: u32,
}

impl Lead {
    pub fn headline(&self) -> String {
        if self.doors_on_file > 0 {
            format!(
                "{} brought in {} — {} doors already on the file",
                self.finder, self.target_name, self.doors_on_file
            )
        } else {
            format!("{} brought in {}", self.finder, self.target_name)
        }
    }
}

/// The chance somebody brings something in this week. Every contented hand is
/// another set of ears, with a ceiling so a large happy crew does not simply
/// print opportunities.
pub fn chance_of_a_lead(session: &GameSession, config: &LeadConfig) -> f32 {
    let ears = session
        .crew
        .iter()
        .filter(|member| member.condition.loyalty >= config.loyalty_threshold)
        .count();
    if ears == 0 {
        return 0.0;
    }
    (ears as f32 * config.chance_per_hand).min(config.max_chance)
}

/// Roll for a tip-off, and put it on the board if one comes in.
pub fn roll_lead(session: &mut GameSession, data: &GameData) -> Option<Lead> {
    let config = &data.config.leads;
    let chance = chance_of_a_lead(session, config);
    if chance <= 0.0 || session.rng.next_f32() >= chance {
        return None;
    }

    // Only marks the outfit's name already opens, and only ones nobody is
    // already looking at. Sorted: registry order is not stable (GDD 5.7).
    let mut candidates: Vec<String> = session
        .eligible_targets(data)
        .into_iter()
        .map(|target| target.id.clone())
        .filter(|id| !session.board.iter().any(|entry| &entry.target_id == id))
        .collect();
    candidates.sort();
    if candidates.is_empty() {
        return None;
    }

    // Whoever is happiest hears it first; the id breaks ties so two equally
    // contented hands always resolve the same way.
    let finder = session
        .crew
        .iter()
        .filter(|member| member.condition.loyalty >= config.loyalty_threshold)
        .max_by(|a, b| {
            a.condition
                .loyalty
                .cmp(&b.condition.loyalty)
                .then(b.id.cmp(&a.id))
        })?
        .name
        .clone();

    let target_id = candidates[session.rng.below(candidates.len())].clone();
    let target_name = data
        .targets
        .get(&target_id)
        .map(|target| target.name.clone())
        .unwrap_or_else(|| target_id.clone());
    let doors = data
        .targets
        .get(&target_id)
        .map(|target| target.encounters.len() as u32)
        .unwrap_or(0);
    let on_file = config.doors_on_file.min(doors);

    let mut entry = BoardEntry::new(target_id.clone());
    entry.casing = on_file;
    entry.weeks_remaining += config.extra_weeks;
    session.board.push(entry);
    session.tally.leads_brought_in += 1;

    Some(Lead {
        finder,
        target_id,
        target_name,
        doors_on_file: on_file,
    })
}

#[cfg(test)]
mod tests {
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
}
