use herdr_damnit_domain::{Object, brief};
use serde::Deserialize;

use crate::Herdr;
use crate::argv;

const PASTE_START: &str = "\x1b[200~";
const PASTE_END: &str = "\x1b[201~";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Agent {
    pub pane: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct Workspace {
    pub workspace: Option<String>,
    pub me: String,
}

#[derive(Debug)]
pub enum HandOff {
    Sent {
        agent: Agent,
        label_write: Option<Vec<String>>,
    },
    Refused(String),
}

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

pub fn hand_off(
    herdr: &dyn Herdr,
    here: &Workspace,
    object: &Object,
    note: &str,
    handoff_label: &str,
) -> HandOff {
    let Some(workspace) = here.workspace.as_deref() else {
        return HandOff::Refused("no workspace context: run this pane inside herdr.".to_string());
    };
    let listing = match herdr.call(&["agent", "list"]) {
        Ok(listing) => listing,
        Err(error) => return HandOff::Refused(error),
    };
    let agent = match first_listed_agent_pane_in(&listing, workspace, &here.me) {
        Ok(agent) => agent,
        Err(error) => return HandOff::Refused(error),
    };
    let text = as_one_bracketed_paste(&brief(object, note));
    if herdr
        .call(&["pane", "send-text", &agent.pane, &text])
        .is_err()
    {
        return HandOff::Refused(format!("herdr refused the send to {}.", agent.pane));
    }
    let _ = herdr.call(&["agent", "focus", &agent.pane]);
    let label_write =
        (!handoff_label.trim().is_empty()).then(|| argv::label(&object.oid, handoff_label.trim()));
    HandOff::Sent { agent, label_write }
}

fn first_listed_agent_pane_in(listing: &str, workspace: &str, me: &str) -> Result<Agent, String> {
    let listing: Listing =
        serde_json::from_str(listing).map_err(|error| format!("herdr agent list: {error}"))?;
    listing
        .agents()
        .filter(|listed| listed.agent.is_some() && listed.pane_id != me)
        .find(|listed| listed.workspace_id == workspace)
        .map(|listed| Agent {
            name: name_then_agent_kind_then_auth_profile(listed),
            pane: listed.pane_id.clone(),
        })
        .ok_or_else(|| "no agent pane in this workspace.".to_string())
}

impl Listing {
    fn agents(&self) -> impl Iterator<Item = &Listed> {
        self.result.agents.iter()
    }
}

fn name_then_agent_kind_then_auth_profile(listed: &Listed) -> String {
    [&listed.name, &listed.agent, &listed.display_agent]
        .into_iter()
        .flatten()
        .find(|name| !name.is_empty())
        .cloned()
        .unwrap_or_else(|| "the agent".to_string())
}

fn as_one_bracketed_paste(text: &str) -> String {
    let body = without_paste_terminators_even_spliced_ones(text);
    format!("{PASTE_START}{body}{PASTE_END}")
}

fn without_paste_terminators_even_spliced_ones(text: &str) -> String {
    let mut body = String::with_capacity(text.len());
    for character in text.chars() {
        body.push(character);
        if body.ends_with(PASTE_END) {
            body.truncate(body.len() - PASTE_END.len());
        }
    }
    body
}

#[cfg(test)]
mod tests;
