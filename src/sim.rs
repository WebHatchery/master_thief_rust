//! The week, and what a job does to a crew.
//!
//! Everything here draws from the session's seeded RNG in a fixed order, so a
//! seed plus a plan replays exactly (GDD 5.7).

pub mod campaign;
pub mod job;
pub mod loot;
pub mod plan;
pub mod week;

pub use campaign::{play, CampaignLog};
pub use job::{auto_assign, run_job, DoorOutcome, JobPlan, JobReport};
pub use plan::{candidates, Candidate, PlanDraft};
pub use week::advance_week;
