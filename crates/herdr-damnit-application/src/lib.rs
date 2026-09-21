//! The pane's use cases. Nothing here names a process, a file or a terminal.

pub mod argv;
mod jobs;
mod ports;

pub use argv::Side;
pub use jobs::{Completion, JobId, JobKind, Jobs, Submitted, SyncKind};
pub use ports::{Clock, DamRunner, Finished, Herdr, RunningJob, SpawnError};
