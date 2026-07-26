//! The week, and what a job does to a crew.
//!
//! Everything here draws from the session's seeded RNG in a fixed order, so a
//! seed plus a plan replays exactly (GDD 5.7).

pub mod job;
pub mod week;

pub use job::{auto_assign, run_job, Assignment, DoorOutcome, JobPlan, JobReport};
pub use week::advance_week;
