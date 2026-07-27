//! What the city does about an outfit it has started to notice.
//!
//! GDD 5.6 promised that heat above a threshold "adds law-enforcement
//! encounters, and can retire a crew member into custody". Below the threshold
//! heat is still only a difficulty modifier; above it, the week rolls once and
//! the city can take money, take attention, or take somebody.
//!
//! Every draw here comes from the session's RNG, at a fixed point in
//! [`crate::sim::advance_week`], so a seed still replays a campaign exactly
//! (GDD 5.7).

use crate::data::{GameConfig, LawConfig};
use crate::state::{CustodyRecord, GameSession};
use macroquad_toolkit::ui::format_money;

/// The three shapes an incident takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LawEventKind {
    /// A tail on the crew: every door is harder while it lasts.
    Surveillance,
    /// A visit to the safehouse. They take what is lying around.
    Raid,
    /// Somebody is picked up and held. Bail is the only way back.
    Arrest,
}

impl LawEventKind {
    pub fn label(self) -> &'static str {
        match self {
            LawEventKind::Surveillance => "Surveillance",
            LawEventKind::Raid => "Raid",
            LawEventKind::Arrest => "Arrest",
        }
    }
}

/// One incident, already applied, described for the week summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawEvent {
    pub kind: LawEventKind,
    /// What happened, in one line the player reads on the week roll-over.
    pub headline: String,
    /// Cash seized, if any.
    pub cash_lost: i64,
    /// Who was taken, if anybody.
    pub taken: Option<String>,
    pub heat_shed: i32,
}

/// How exposed the outfit is this week, 0..1. Shown on the crew screen so the
/// player can see the risk before they decide to run another job (pillar 2).
pub fn attention_chance(session: &GameSession, config: &LawConfig) -> f32 {
    let over = (session.heat - config.attention_threshold).max(0);
    if over <= 0 {
        return 0.0;
    }
    (over as f32 * config.attention_chance_per_point).min(config.attention_chance_max)
}

/// Roll the week's attention. Draws from the run's RNG only when the city is
/// actually looking, and applies whatever it finds before returning it.
pub fn roll_attention(session: &mut GameSession, config: &GameConfig) -> Option<LawEvent> {
    let law = &config.law;
    let chance = attention_chance(session, law);
    if chance <= 0.0 {
        return None;
    }

    if session.rng.next_f32() >= chance {
        return None;
    }

    let pick = session.rng.below(100);
    let can_arrest = session.heat >= law.custody_threshold && !session.crew.is_empty();

    let event = if can_arrest && pick < law.arrest_share {
        arrest(session, law)
    } else if pick < law.raid_share {
        raid(session, law)
    } else {
        surveillance(session, law)
    };

    session.tally.law_incidents += 1;
    Some(event)
}

/// A tail. Nothing is taken; everything gets harder for a couple of weeks.
fn surveillance(session: &mut GameSession, config: &LawConfig) -> LawEvent {
    session.surveillance_weeks = session.surveillance_weeks.max(config.surveillance_weeks);
    LawEvent {
        kind: LawEventKind::Surveillance,
        headline: format!(
            "A car sits on the safehouse. Every door is {} harder for {} weeks",
            config.surveillance_penalty, config.surveillance_weeks
        ),
        cash_lost: 0,
        taken: None,
        heat_shed: 0,
    }
}

/// A raid. They take a share of whatever the outfit is sitting on, and go away
/// slightly satisfied.
fn raid(session: &mut GameSession, config: &LawConfig) -> LawEvent {
    let seized = ((session.budget.max(0) as f32) * config.raid_seizure_share) as i64;
    session.budget -= seized;
    let shed = shed_heat(session, config.raid_heat_relief);
    session.tally.cash_seized += seized;

    LawEvent {
        kind: LawEventKind::Raid,
        headline: format!(
            "The safehouse was turned over — {} gone",
            format_money(seized)
        ),
        cash_lost: seized,
        taken: None,
        heat_shed: shed,
    }
}

/// An arrest. They take the face they have seen most often — the hand with the
/// most jobs behind them — and hold them until somebody posts bail.
fn arrest(session: &mut GameSession, config: &LawConfig) -> LawEvent {
    let Some(index) = most_exposed(session) else {
        return surveillance(session, config);
    };

    let member = session.crew.remove(index);
    let bail = bail_cost(&member, session.notoriety, config);
    let name = member.name.clone();

    session.custody.push(CustodyRecord {
        member,
        week_taken: session.week,
        bail,
    });
    let shed = shed_heat(session, config.arrest_heat_relief);
    session.tally.arrests += 1;

    LawEvent {
        kind: LawEventKind::Arrest,
        headline: format!("{} was picked up. Bail is {}", name, format_money(bail)),
        cash_lost: 0,
        taken: Some(name),
        heat_shed: shed,
    }
}

/// Whoever the city has seen the most of. Deterministic: jobs first, then the
/// id, so two hands with identical records always resolve the same way.
fn most_exposed(session: &GameSession) -> Option<usize> {
    session
        .crew
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            a.progression
                .jobs_completed
                .cmp(&b.progression.jobs_completed)
                .then(b.id.cmp(&a.id))
        })
        .map(|(index, _)| index)
}

fn shed_heat(session: &mut GameSession, relief: i32) -> i32 {
    let before = session.heat;
    session.heat = (session.heat - relief).max(0);
    before - session.heat
}

/// What it costs to make the city look elsewhere for a while. A better-known
/// outfit pays more for the same silence, so this never becomes a tap the
/// player can leave running.
pub fn bribe_cost(session: &GameSession, config: &LawConfig) -> i64 {
    config.bribe_cost_base + config.bribe_cost_per_notoriety * session.notoriety.max(0) as i64
}

/// Grease the right palms. Heat is the reducible half of the outfit's
/// reputation for being caught; notoriety is the ledger and never moves down
/// (GDD 12, open question 5).
pub fn grease_palms(session: &mut GameSession, config: &GameConfig) -> Result<String, String> {
    let law = &config.law;
    if session.heat <= 0 {
        return Err("Nobody is looking. Save the money".to_owned());
    }
    let cost = bribe_cost(session, law);
    if session.budget < cost {
        return Err(format!("That kind of quiet costs {}", format_money(cost)));
    }

    session.budget -= cost;
    let shed = shed_heat(session, law.bribe_heat_relief);
    session.tally.bribes_paid += cost;
    session.tally.heat_bought_down += shed as i64;

    Ok(format!("{} spent, heat down {}", format_money(cost), shed))
}

pub fn bail_cost(member: &crate::model::CrewMember, notoriety: i32, config: &LawConfig) -> i64 {
    config.bail_base
        + config.bail_per_level * member.progression.level.max(1) as i64
        + config.bail_per_notoriety * notoriety.max(0) as i64
}

/// Buy somebody back out. They come back rested — a cell is nothing if not
/// restful — and considerably less fond of the outfit that left them in it.
pub fn post_bail(
    session: &mut GameSession,
    config: &GameConfig,
    member_id: &str,
) -> Result<String, String> {
    let Some(index) = session
        .custody
        .iter()
        .position(|record| record.member.id == member_id)
    else {
        return Err("Nobody by that name is being held".to_owned());
    };

    let bail = session.custody[index].bail;
    if session.budget < bail {
        return Err(format!(
            "{} needs {} and the outfit has less",
            session.custody[index].member.name,
            format_money(bail)
        ));
    }

    session.budget -= bail;
    let record = session.custody.remove(index);
    let mut member = record.member;
    let name = member.name.clone();

    member.condition.fatigue = 0;
    member.condition.injuries.clear();
    member.condition.loyalty = config.law.bail_return_loyalty;
    member.condition.notice_given = false;
    member.condition.weeks_unpaid = 0;
    session.crew.push(member);
    session.tally.bails_paid += bail;

    Ok(format!("{} walks out for {}", name, format_money(bail)))
}

/// Put one hand in a cell. Only for tests in other modules that need somebody
/// held without reaching into the arrest logic themselves.
#[cfg(test)]
pub fn tests_support_arrest(session: &mut GameSession, config: &GameConfig) {
    arrest(session, &config.law);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;

    fn setup(seed: u64) -> (GameData, GameSession) {
        let data = GameData::load().unwrap();
        let session = GameSession::new(&data.config, &data, seed);
        (data, session)
    }

    #[test]
    fn a_quiet_outfit_is_never_troubled() {
        let (data, mut session) = setup(1);
        session.heat = data.config.law.attention_threshold;

        assert_eq!(attention_chance(&session, &data.config.law), 0.0);
        for _ in 0..50 {
            assert!(roll_attention(&mut session, &data.config).is_none());
        }
    }

    #[test]
    fn the_risk_climbs_with_the_heat_and_then_stops() {
        let (data, mut session) = setup(2);
        let law = &data.config.law;

        session.heat = law.attention_threshold + 10;
        let mild = attention_chance(&session, law);
        session.heat = law.attention_threshold + 30;
        let bad = attention_chance(&session, law);
        session.heat = 10_000;

        assert!(bad > mild && mild > 0.0);
        assert_eq!(attention_chance(&session, law), law.attention_chance_max);
    }

    #[test]
    fn a_hot_outfit_eventually_gets_a_visit() {
        let (data, mut session) = setup(3);
        session.heat = data.config.law.custody_threshold + 20;
        session.budget = 500_000;

        let mut events = Vec::new();
        for _ in 0..80 {
            session.heat = data.config.law.custody_threshold + 20;
            if let Some(event) = roll_attention(&mut session, &data.config) {
                events.push(event.kind);
            }
        }

        assert!(!events.is_empty(), "eighty hot weeks and nobody called");
        assert!(
            events.contains(&LawEventKind::Arrest),
            "custody never happened: {:?}",
            events
        );
    }

    #[test]
    fn an_arrest_takes_the_hand_the_city_has_seen_most_of() {
        let (data, mut session) = setup(4);
        session.crew[1].progression.jobs_completed = 12;
        let expected = session.crew[1].name.clone();
        let roster = session.crew.len();

        let event = arrest(&mut session, &data.config.law);

        assert_eq!(event.taken.as_deref(), Some(expected.as_str()));
        assert_eq!(session.crew.len(), roster - 1);
        assert_eq!(session.custody.len(), 1);
        assert!(session.custody[0].bail > 0);
    }

    #[test]
    fn bail_returns_them_rested_and_unimpressed() {
        let (data, mut session) = setup(5);
        session.crew[0].condition.fatigue = 70;
        session.crew[0]
            .condition
            .injuries
            .push(crate::model::crew::Injury::major("Cracked rib"));
        arrest(&mut session, &data.config.law);

        let id = session.custody[0].member.id.clone();
        let bail = session.custody[0].bail;
        session.budget = bail + 10;

        assert!(post_bail(&mut session, &data.config, &id).is_ok());
        assert_eq!(session.budget, 10);
        assert!(session.custody.is_empty());

        let back = session.member(&id).expect("they are on the payroll again");
        assert_eq!(back.condition.fatigue, 0);
        assert!(back.condition.injuries.is_empty());
        assert_eq!(back.condition.loyalty, data.config.law.bail_return_loyalty);
    }

    #[test]
    fn bail_nobody_can_afford_leaves_them_where_they_are() {
        let (data, mut session) = setup(6);
        arrest(&mut session, &data.config.law);
        let id = session.custody[0].member.id.clone();
        session.budget = 0;

        assert!(post_bail(&mut session, &data.config, &id).is_err());
        assert_eq!(session.custody.len(), 1);
    }

    #[test]
    fn a_raid_takes_a_share_of_the_cash_and_cools_the_city_a_little() {
        let (data, mut session) = setup(7);
        session.budget = 200_000;
        session.heat = 90;

        let event = raid(&mut session, &data.config.law);

        assert!(event.cash_lost > 0);
        assert_eq!(session.budget, 200_000 - event.cash_lost);
        assert!(session.heat < 90);
    }

    #[test]
    fn heat_can_be_bought_down_and_notoriety_never_can() {
        // GDD 12, open question 5, settled: the ledger is monotonic, the
        // short-term half is not.
        let (data, mut session) = setup(8);
        session.heat = 60;
        session.notoriety = 40;
        session.budget = 1_000_000;

        let cost = bribe_cost(&session, &data.config.law);
        assert!(grease_palms(&mut session, &data.config).is_ok());

        assert_eq!(session.heat, 60 - data.config.law.bribe_heat_relief);
        assert_eq!(session.notoriety, 40, "notoriety is the ledger");
        assert_eq!(session.budget, 1_000_000 - cost);
    }

    #[test]
    fn silence_gets_more_expensive_as_the_name_grows() {
        let (data, mut session) = setup(9);
        session.notoriety = 5;
        let early = bribe_cost(&session, &data.config.law);
        session.notoriety = 90;

        assert!(bribe_cost(&session, &data.config.law) > early);
    }

    #[test]
    fn there_is_nothing_to_buy_when_nobody_is_looking() {
        let (data, mut session) = setup(10);
        session.heat = 0;
        session.budget = 1_000_000;

        assert!(grease_palms(&mut session, &data.config).is_err());
        assert_eq!(session.budget, 1_000_000);
    }
}
