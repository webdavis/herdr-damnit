//! The pane's use cases. Nothing here names a process, a file or a terminal.

mod ports;

pub use ports::{Clock, DamRunner, Finished, Herdr, RunningJob, SpawnError};
