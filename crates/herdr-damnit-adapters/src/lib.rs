//! Everything the pane touches outside itself: `dam`, `herdr`, the config, the state and the clock.

pub mod config;
mod dam_runner;
pub mod wire;

pub use dam_runner::{Cancel, ProcessDamRunner};
