//! The trade: every rule that decides what a crew can do, kept honest.
//!
//! Nothing here touches macroquad, rendering, or the frame loop. It is a pure
//! library so the ported tests run headlessly and a soak test can play
//! thousands of jobs without opening a window.

pub mod attributes;
pub mod chemistry;
pub mod encounter;
pub mod environment;
pub mod outcome;

pub use attributes::attribute_modifier;
pub use outcome::Outcome;
