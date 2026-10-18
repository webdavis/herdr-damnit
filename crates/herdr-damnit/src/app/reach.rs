use herdr_damnit_application::{Agent, AgentPanes, HandOff, Workspace, agent_pane, hand_off};
use herdr_damnit_domain::Object;

pub(super) struct AgentReach {
    panes: Box<dyn AgentPanes>,
    here: Workspace,
}

impl AgentReach {
    pub(super) fn new(panes: Box<dyn AgentPanes>, here: Workspace) -> Self {
        Self { panes, here }
    }

    pub(super) fn pane(&self) -> Result<Agent, String> {
        agent_pane(&*self.panes, &self.here)
    }

    pub(super) fn hand_off(&self, object: &Object, note: &str, label: &str) -> HandOff {
        hand_off(&*self.panes, &self.here, object, note, label)
    }
}
