//! Master Thief — a heist crew-dispatch sim with a d20 at its heart.
//!
//! The crate is a library with a thin binary on top so the rules engine can be
//! exercised headlessly: `rules` and `sim` never touch a window, and the ported
//! tests plus the distribution soak run without one (GDD 11).

pub mod audio;
pub mod data;
pub mod game;
pub mod heist_actions;
pub mod model;
pub mod prefs;
pub mod rules;
pub mod sim;
pub mod state;
pub mod ui;
