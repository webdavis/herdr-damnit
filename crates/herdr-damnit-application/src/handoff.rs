use crate::argv;
use crate::{AgentPanes, ListedAgent};
use herdr_damnit_domain::{Object, brief};

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

pub fn hand_off(
    panes: &dyn AgentPanes,
    here: &Workspace,
    object: &Object,
    note: &str,
    handoff_label: &str,
) -> HandOff {
    let agent = match agent_pane(panes, here) {
        Ok(agent) => agent,
        Err(error) => return HandOff::Refused(error),
    };
    let text = as_one_bracketed_paste(&brief(object, note));
    if panes.send_text(&agent.pane, &text).is_err() {
        return HandOff::Refused(format!("herdr refused the send to {}.", agent.pane));
    }
    let _ = panes.focus_agent(&agent.pane);
    let label_write =
        (!handoff_label.trim().is_empty()).then(|| argv::label(&object.oid, handoff_label.trim()));
    HandOff::Sent { agent, label_write }
}

pub fn agent_pane(panes: &dyn AgentPanes, here: &Workspace) -> Result<Agent, String> {
    let Some(workspace) = here.workspace.as_deref() else {
        return Err("no workspace context: run this pane inside herdr.".to_string());
    };
    first_listed_agent_pane_in(&panes.agents()?, workspace, &here.me)
}

fn first_listed_agent_pane_in(
    agents: &[ListedAgent],
    workspace: &str,
    me: &str,
) -> Result<Agent, String> {
    agents
        .iter()
        .filter(|listed| listed.kind.is_some() && listed.pane != me)
        .find(|listed| listed.workspace == workspace)
        .map(|listed| Agent {
            name: name_then_agent_kind_then_auth_profile(listed),
            pane: listed.pane.clone(),
        })
        .ok_or_else(|| "no agent pane in this workspace.".to_string())
}

fn name_then_agent_kind_then_auth_profile(listed: &ListedAgent) -> String {
    [&listed.name, &listed.kind, &listed.profile]
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
