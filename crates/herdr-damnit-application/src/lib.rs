pub mod argv;
#[cfg(any(test, feature = "fakes"))]
pub mod fakes;
mod handoff;
mod handshake;
mod jobs;
mod ports;

pub use argv::Toward;
pub use handoff::{Agent, HandOff, Workspace, agent_pane, hand_off};
pub use handshake::{Handshake, Phase, check_version};
pub use jobs::{Completion, JobId, Jobs, Request, Submitted, SyncKind};
pub use ports::{
    AgentPanes, Clock, DamRunner, Finished, ListedAgent, PaneFocus, PaneMemory, PaneOpen,
    PanePlacement, PluginPanes, RunningJob, SpawnError, SplitSide,
};
