//! The pane's use cases. Nothing here names a process, a file or a terminal.

pub mod argv;
mod handoff;
mod handshake;
mod jobs;
mod ports;

pub use argv::Side;
pub use handoff::{Agent, HandOff, Workspace, hand_off};
pub use handshake::{Handshake, handshake};
pub use jobs::{Completion, JobId, JobKind, Jobs, Submitted, SyncKind};
pub use ports::{Clock, DamRunner, Finished, Herdr, RunningJob, SpawnError};
