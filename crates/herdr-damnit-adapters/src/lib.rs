//! Everything the pane touches outside itself: `dam`, `herdr`, the config, the state and the clock.

mod clock;
pub mod config;
mod dam_runner;
pub mod herdr_cli;
pub mod state;
pub mod wire;

pub use clock::SystemClock;
pub use config::Config;
pub use dam_runner::{Cancel, ProcessDamRunner};
pub use herdr_cli::CliHerdr;
