//! The week, and what a job does to a crew.
//!
//! Everything here draws from the session's seeded RNG in a fixed order, so a
//! seed plus a plan replays exactly (GDD 5.7).

pub mod awards;
pub mod campaign;
pub mod delegation;
pub mod fence;
pub mod infirmary;
pub mod job;
pub mod kit;
pub mod law;
pub mod leads;
pub mod loot;
pub mod payroll;
pub mod plan;
pub mod rivals;
pub mod week;

pub use awards::{award, AwardDef, CampaignTally};
pub use campaign::{play, CampaignLog};
pub use delegation::DelegationMiss;
pub use fence::{quote as fence_quote, sell, Offer};
pub use infirmary::{quote as treatment_quote, treat, Treatment};
pub use job::{auto_assign, run_job, DoorOutcome, JobPlan, JobReport};
pub use kit::{refit, refit_quote, wear_penalty};
pub use law::{attention_chance, bail_cost, bribe_cost, grease_palms, post_bail, LawEvent};
pub use leads::{chance_of_a_lead, Lead};
pub use payroll::{
    bonus_cost, crew_cut, crew_cut_for, pay_bonus, retainer_for, runway_after_hiring,
    safehouse_upkeep, weekly_outgoings, weeks_of_runway, CrewCut,
};
pub use plan::{candidates, Candidate, PlanDraft};
pub use rivals::{interest_in as rival_interest, RivalJob};
pub use week::{advance_week, WeekSummary};
