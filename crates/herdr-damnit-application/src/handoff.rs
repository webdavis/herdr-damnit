//! `S`: hand the object under the cursor to the workspace's agent pane. The brief goes into that
//! pane's input as one bracketed paste and is never submitted, so the operator reads it, adds to
//! it and presses return themselves.

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

/// The two pane facts herdr puts in this process's environment.
#[derive(Clone, Debug)]
pub struct Workspace {
    pub workspace: Option<String>,
    pub me: String,
}

#[derive(Debug)]
pub enum HandOff {
    Sent {
        agent: Agent,
        /// The argv of the label write that records the hand-off, for the caller to submit as an
        /// ordinary write job.
        label: Option<Vec<String>>,
    },
    Refused(String),
}

/// One row of `herdr agent list`. `name` is absent until herdr sets one; `display_agent` is the
/// auth profile a pane authenticated with rather than the agent, so the true kind outranks it.
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
    let agent = match agent_in(&listing, workspace, &here.me) {
        Ok(agent) => agent,
        Err(error) => return HandOff::Refused(error),
    };
    let text = pasted(&brief(object, note));
    if herdr
        .call(&["pane", "send-text", &agent.pane, &text])
        .is_err()
    {
        return HandOff::Refused(format!("herdr refused the send to {}.", agent.pane));
    }
    // Focus is a convenience once the text is delivered: a refused focus leaves the brief in the
    // agent's input, so failing here would report a hand-off that did happen as one that did not.
    let _ = herdr.call(&["agent", "focus", &agent.pane]);
    let label =
        (!handoff_label.trim().is_empty()).then(|| argv::label(&object.oid, handoff_label.trim()));
    HandOff::Sent { agent, label }
}

/// The agent pane of this workspace. A pane herdr lists no agent for is not a candidate, and
/// neither is this one; with several, the first herdr names wins.
fn agent_in(listing: &str, workspace: &str, me: &str) -> Result<Agent, String> {
    let listing: Listing =
        serde_json::from_str(listing).map_err(|error| format!("herdr agent list: {error}"))?;
    listing
        .agents()
        .filter(|listed| listed.agent.is_some() && listed.pane_id != me)
        .find(|listed| listed.workspace_id == workspace)
        .map(|listed| Agent {
            name: named(listed),
            pane: listed.pane_id.clone(),
        })
        .ok_or_else(|| "no agent pane in this workspace.".to_string())
}

impl Listing {
    fn agents(&self) -> impl Iterator<Item = &Listed> {
        self.result.agents.iter()
    }
}

fn named(listed: &Listed) -> String {
    [&listed.name, &listed.agent, &listed.display_agent]
        .into_iter()
        .flatten()
        .find(|name| !name.is_empty())
        .cloned()
        .unwrap_or_else(|| "the agent".to_string())
}

/// The brief as one bracketed paste: a paste is inserted verbatim by any input, where the raw
/// bytes of a multi-line brief would each be read as a key and a newline would submit it. A
/// terminator inside the brief would end the frame early, so the body is rebuilt with a suffix
/// check per character and none can survive, not even one spliced together by an earlier removal.
fn pasted(text: &str) -> String {
    let mut body = String::with_capacity(text.len());
    for character in text.chars() {
        body.push(character);
        if body.ends_with(PASTE_END) {
            body.truncate(body.len() - PASTE_END.len());
        }
    }
    format!("{PASTE_START}{body}{PASTE_END}")
}

#[cfg(test)]
mod tests;
