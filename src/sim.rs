//! The week, and what a job does to a crew.
//!
//! Everything here draws from the session's seeded RNG in a fixed order, so a
//! seed plus a plan replays exactly (GDD 5.7).

pub mod awards;
pub mod campaign;
pub mod delegation;
pub mod job;
pub mod law;
pub mod loot;
pub mod payroll;
pub mod plan;
pub mod week;

pub use awards::{award, AwardDef, CampaignTally};
pub use campaign::{play, CampaignLog};
pub use delegation::DelegationMiss;
pub use job::{auto_assign, run_job, DoorOutcome, JobPlan, JobReport};
pub use law::{attention_chance, bail_cost, bribe_cost, grease_palms, post_bail, LawEvent};
pub use payroll::{
    bonus_cost, crew_cut, pay_bonus, retainer_for, safehouse_upkeep, weekly_outgoings,
    weeks_of_runway, CrewCut,
};
pub use plan::{candidates, Candidate, PlanDraft};
pub use week::{advance_week, WeekSummary};
