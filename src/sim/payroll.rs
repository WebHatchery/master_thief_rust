//! The standing bill: what the outfit costs to keep standing whether or not it
//! works this week.
//!
//! Lying low used to be free — fatigue fell off, injuries closed, heat cooled,
//! and nothing asked for anything back. The safehouse has rent and the crew
//! have retainers, so a quiet week is now a purchase like any other (GDD 5.6).
//! No draw here touches the run's RNG: the payroll is arithmetic, not luck.

use crate::data::{GameConfig, PayrollConfig};
use crate::model::CrewMember;
use crate::state::GameSession;
use macroquad_toolkit::ui::format_money;

/// What a week of payroll actually did, for the week summary the player reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PayrollOutcome {
    pub upkeep: i64,
    pub wages_due: i64,
    pub paid: i64,
    /// What the outfit could not cover. Zero on a good week.
    pub shortfall: i64,
    /// Hands who went unpaid, in roster order.
    pub unpaid: Vec<String>,
    /// Hands who said this week that they are done.
    pub notices: Vec<String>,
    /// Hands who left. They are off the roster by the time this is read.
    pub walkouts: Vec<String>,
}

impl PayrollOutcome {
    pub fn total_due(&self) -> i64 {
        self.upkeep + self.wages_due
    }

    pub fn was_short(&self) -> bool {
        self.shortfall > 0
    }
}

/// What one hand is owed each week. Level and standing both cost: a legendary
/// safecracker on their eighth job does not work for what they took at hiring.
pub fn retainer_for(member: &CrewMember, config: &PayrollConfig) -> i64 {
    config.retainer_base
        + config.retainer_per_level * (member.progression.level - 1).max(0) as i64
        + config.retainer_per_rarity * member.rarity.tier() as i64
}

/// The safehouse's share, inflated by the outfit's own name.
pub fn safehouse_upkeep(session: &GameSession, config: &PayrollConfig) -> i64 {
    config.safehouse_upkeep_base
        + config.safehouse_upkeep_per_reputation * session.reputation.max(0) as i64
}

/// Everything the outfit owes next time the week turns over.
pub fn weekly_outgoings(session: &GameSession, config: &PayrollConfig) -> i64 {
    safehouse_upkeep(session, config)
        + session
            .crew
            .iter()
            .map(|member| retainer_for(member, config))
            .sum::<i64>()
}

/// Weeks the outfit could sit still before it stops being able to pay. This is
/// the number that makes idling a decision.
pub fn weeks_of_runway(session: &GameSession, config: &PayrollConfig) -> i64 {
    let due = weekly_outgoings(session, config);
    if due <= 0 {
        return i64::MAX;
    }
    session.budget.max(0) / due
}

/// Weeks of runway the outfit would have left if it took this hand on: the
/// hiring fee comes out of the purse and their retainer joins the bill.
///
/// The hiring screen only ever showed the fee, which is the smaller half of the
/// question and the one that stops mattering after week three. A legendary hand
/// costs more every week, forever, than they cost to sign.
pub fn runway_after_hiring(
    session: &GameSession,
    config: &PayrollConfig,
    recruit: &CrewMember,
    fee: i64,
) -> i64 {
    let due = weekly_outgoings(session, config) + retainer_for(recruit, config);
    if due <= 0 {
        return i64::MAX;
    }
    (session.budget - fee).max(0) / due
}

/// What talking one hand round costs.
pub fn bonus_cost(member: &CrewMember, config: &PayrollConfig) -> i64 {
    retainer_for(member, config) * config.bonus_retainer_weeks.max(1)
}

/// The crew's share of one job, and who moved it. The retainer buys their week;
/// the cut is what they want for *this* job, and it is a negotiation rather than
/// a constant.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CrewCut {
    /// Share of the take the crew keeps, 0..1.
    pub share: f32,
    /// Every reason the share is not the base rate, named for the player.
    pub reasons: Vec<CutReason>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CutReason {
    pub label: String,
    /// Percentage points added to the crew's share. Negative is a discount.
    pub points: f32,
}

impl CrewCut {
    pub fn percent(&self) -> f32 {
        self.share * 100.0
    }

    /// What the fixer keeps of a given take.
    pub fn net_of(&self, payout: i64) -> i64 {
        payout - self.take_of(payout)
    }

    pub fn take_of(&self, payout: i64) -> i64 {
        (payout as f32 * self.share) as i64
    }
}

/// What the hands down for a job will want for it. Standing costs — a
/// legendary safecracker does not work a job for a beginner's share — and so
/// does discontent: a hand who is halfway out of the door holds out, and a
/// steady one does not haggle. Every term is named so the planning screen can
/// show the player what their roster choice is costing them (pillar 2).
pub fn crew_cut(session: &GameSession, config: &GameConfig, crew_on_job: &[String]) -> CrewCut {
    crew_cut_for(session, config, crew_on_job, false)
}

/// The same, told whether the crew planned the job themselves. They charge for
/// that: doing the fixer's thinking is work, and pillar 5 says delegation must
/// never come out ahead of a plan somebody actually made.
pub fn crew_cut_for(
    session: &GameSession,
    config: &GameConfig,
    crew_on_job: &[String],
    crew_planned: bool,
) -> CrewCut {
    let cut = &config.cut;
    let mut reasons = Vec::new();
    let mut share = cut.base_share;

    if crew_planned && !crew_on_job.is_empty() {
        share += cut.delegation_premium;
        reasons.push(CutReason {
            label: "They planned it themselves".to_owned(),
            points: cut.delegation_premium * 100.0,
        });
    }

    let members: Vec<&CrewMember> = crew_on_job
        .iter()
        .filter_map(|id| session.member(id))
        .collect();

    if members.len() > 1 {
        let extra = (members.len() - 1) as f32 * cut.per_extra_hand;
        share += extra;
        reasons.push(CutReason {
            label: format!("{} hands splitting it", members.len()),
            points: extra * 100.0,
        });
    }

    let standing: i32 = members.iter().map(|member| member.rarity.tier()).sum();
    if standing > 0 {
        let premium = standing as f32 * cut.per_rarity_tier;
        share += premium;
        reasons.push(CutReason {
            label: "Names worth paying for".to_owned(),
            points: premium * 100.0,
        });
    }

    let holdouts = members
        .iter()
        .filter(|member| member.condition.loyalty <= cut.holdout_loyalty)
        .count();
    if holdouts > 0 {
        let premium = holdouts as f32 * cut.holdout_premium;
        share += premium;
        reasons.push(CutReason {
            label: format!("{} holding out", holdouts),
            points: premium * 100.0,
        });
    }

    // A pair who have learned to read each other are worth more together than
    // apart, and they have noticed. This is what makes a good partnership
    // expensive to keep together rather than free (GDD 5.5).
    let partnerships = session.chemistry.partnerships_among(crew_on_job);
    if !partnerships.is_empty() {
        let premium = partnerships.len() as f32 * cut.partnership_premium;
        share += premium;
        let named = partnerships
            .iter()
            .filter_map(|(a, b)| {
                Some(format!(
                    "{} & {}",
                    session.member(a)?.name.split(' ').next()?,
                    session.member(b)?.name.split(' ').next()?
                ))
            })
            .collect::<Vec<_>>()
            .join(", ");
        reasons.push(CutReason {
            label: if named.is_empty() {
                "Working as a unit".to_owned()
            } else {
                format!("{} work as a pair", named)
            },
            points: premium * 100.0,
        });
    }

    let steady = members
        .iter()
        .filter(|member| member.condition.loyalty >= cut.steady_loyalty)
        .count();
    if steady > 0 {
        let discount = steady as f32 * cut.steady_discount;
        share -= discount;
        reasons.push(CutReason {
            label: format!("{} not haggling", steady),
            points: -discount * 100.0,
        });
    }

    CrewCut {
        share: share.clamp(cut.min_share, cut.max_share),
        reasons,
    }
}

/// Pay the week's bill. The safehouse is covered first — there is nowhere to
/// work without it — then the crew in roster order, so a short week is short
/// for the same people until the fixer fixes it.
pub fn settle_payroll(session: &mut GameSession, config: &GameConfig) -> PayrollOutcome {
    let payroll = &config.payroll;
    let mut outcome = PayrollOutcome {
        upkeep: safehouse_upkeep(session, payroll),
        ..PayrollOutcome::default()
    };

    let mut purse = session.budget.max(0);
    let paid_upkeep = outcome.upkeep.min(purse);
    purse -= paid_upkeep;
    outcome.paid += paid_upkeep;
    outcome.shortfall += outcome.upkeep - paid_upkeep;

    for member in &mut session.crew {
        let due = retainer_for(member, payroll);
        outcome.wages_due += due;

        if purse >= due {
            purse -= due;
            outcome.paid += due;
            member.condition.weeks_unpaid = 0;
            continue;
        }

        // Part of a wage is not a wage. They notice.
        outcome.shortfall += due;
        outcome.unpaid.push(member.name.clone());
        member.condition.weeks_unpaid += 1;
        member
            .condition
            .adjust_loyalty(-payroll.unpaid_loyalty_cost * member.condition.weeks_unpaid);
    }

    session.budget -= outcome.paid;
    session.tally.wages_paid += outcome.paid;
    if outcome.was_short() {
        session.tally.weeks_missed_payroll += 1;
    }

    resign_or_stay(session, payroll, &mut outcome);
    outcome
}

/// Who is leaving, who is only saying so, and who was talked round. A hand
/// gives a week's notice before they go: the fixer always gets one chance to
/// pay them out of it.
fn resign_or_stay(session: &mut GameSession, config: &PayrollConfig, outcome: &mut PayrollOutcome) {
    let mut leaving: Vec<String> = Vec::new();

    for member in &mut session.crew {
        let sullen = member.condition.loyalty <= config.notice_loyalty_threshold;

        if !sullen {
            member.condition.notice_given = false;
        } else if member.condition.notice_given {
            leaving.push(member.id.clone());
        } else {
            member.condition.notice_given = true;
            outcome.notices.push(member.name.clone());
        }
    }

    for id in leaving {
        if let Some(index) = session.crew.iter().position(|member| member.id == id) {
            let gone = session.crew.remove(index);
            outcome.walkouts.push(gone.name.clone());
            session.tally.walkouts += 1;
        }
    }
}

/// What paying somebody off would cost.
pub fn severance_cost(member: &CrewMember, config: &PayrollConfig) -> i64 {
    retainer_for(member, config) * config.severance_weeks.max(1)
}

/// Let somebody go.
///
/// Retainers made a crew member a standing cost, and nothing anywhere could
/// stop one. A hand the outfit could not use and could not afford was a
/// permanent drain whose only exit was to stop paying the *whole* crew until
/// that one quit — which is a perverse thing for a game to reward, and it was
/// the optimal play.
///
/// Paying somebody off costs their notice in cash and costs goodwill with
/// everybody still on the books, because a crew who watch somebody let go draw
/// the obvious conclusion. So clearing house is possible, priced, and never
/// free of consequences.
pub fn dismiss(
    session: &mut GameSession,
    config: &GameConfig,
    member_id: &str,
) -> Result<String, String> {
    let payroll = &config.payroll;
    let Some(member) = session.member(member_id) else {
        return Err("They are not on the payroll".to_owned());
    };
    let cost = severance_cost(member, payroll);
    let name = member.name.clone();

    if session.crew.len() <= 1 {
        return Err(format!("{} is the whole outfit", name));
    }
    if session.budget < cost {
        return Err(format!("Paying {} off costs {}", name, format_money(cost)));
    }

    session.budget -= cost;
    session.tally.severance_paid += cost;
    session.tally.dismissals += 1;
    session.crew.retain(|other| other.id != member_id);

    // Everybody left saw it happen.
    for other in &mut session.crew {
        other
            .condition
            .adjust_loyalty(-payroll.dismissal_loyalty_cost);
    }

    Ok(format!(
        "{} paid off for {} — the rest noticed",
        name,
        format_money(cost)
    ))
}

/// Buy back some goodwill. Costs real money, cancels a notice, and is the only
/// thing that moves loyalty upward on demand.
pub fn pay_bonus(
    session: &mut GameSession,
    config: &GameConfig,
    member_id: &str,
) -> Result<String, String> {
    let payroll = &config.payroll;
    let Some(member) = session.member(member_id) else {
        return Err("They are not on the payroll".to_owned());
    };
    let cost = bonus_cost(member, payroll);
    let name = member.name.clone();

    if session.budget < cost {
        return Err(format!(
            "{} wants {} to stay, and the outfit has less",
            name,
            format_money(cost)
        ));
    }

    session.budget -= cost;
    session.tally.bonuses_paid += cost;
    if let Some(member) = session.member_mut(member_id) {
        member
            .condition
            .adjust_loyalty(payroll.bonus_loyalty_restored);
        member.condition.notice_given = false;
        member.condition.weeks_unpaid = 0;
    }
    Ok(format!("{} squared away for {}", name, format_money(cost)))
}

#[cfg(test)]
mod tests;
