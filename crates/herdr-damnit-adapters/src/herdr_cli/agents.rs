use herdr_damnit_application::{AgentPanes, ListedAgent};
use serde::Deserialize;

use super::CliHerdr;

#[derive(Deserialize)]
struct Listed {
    pane_id: String,
    workspace_id: String,
    #[serde(default)]
    agent: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    display_agent: Option<String>,
}

#[derive(Deserialize)]
struct Listing {
    result: Agents,
}

#[derive(Deserialize)]
struct Agents {
    agents: Vec<Listed>,
}

pub(super) const AGENT_LIST: [&str; 2] = ["agent", "list"];

impl AgentPanes for CliHerdr {
    fn agents(&self) -> Result<Vec<ListedAgent>, String> {
        agents_of_listing(&self.run(&AGENT_LIST)?)
    }

    fn send_text(&self, pane: &str, text: &str) -> Result<(), String> {
        self.run(&send_text_args(pane, text)).map(|_| ())
    }

    fn focus_agent(&self, pane: &str) -> Result<(), String> {
        self.run(&focus_agent_args(pane)).map(|_| ())
    }
}

pub(super) fn send_text_args<'a>(pane: &'a str, text: &'a str) -> [&'a str; 4] {
    ["pane", "send-text", pane, text]
}

pub(super) fn focus_agent_args(pane: &str) -> [&str; 3] {
    ["agent", "focus", pane]
}

pub(super) fn agents_of_listing(output: &str) -> Result<Vec<ListedAgent>, String> {
    let listing: Listing =
        serde_json::from_str(output).map_err(|error| format!("herdr agent list: {error}"))?;
    Ok(listing
        .result
        .agents
        .into_iter()
        .map(|listed| ListedAgent {
            pane: listed.pane_id,
            workspace: listed.workspace_id,
            kind: listed.agent,
            name: listed.name,
            profile: listed.display_agent,
        })
        .collect())
}
