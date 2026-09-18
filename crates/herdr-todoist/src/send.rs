//! `S`: hand the task under the cursor to the workspace's agent pane, and record the hand-off as
//! a comment on the task.
//!
//! The brief is plain text because an agent pane is a shell, not a structure: it goes into that
//! pane's input as one bracketed paste and is never submitted, so the operator reads it, adds to
//! it, and presses return themselves. Nothing here runs on a timer or on an event.

use serde::Deserialize;
use todoist::{Client, task_url};

use crate::edit::Outcome;
use crate::list::{LOWEST_PRIORITY, TaskRow};
use crate::prompt::Prompt;
use crate::reload::Screen;

/// The herdr side of a send: the call it makes, and the two pane facts herdr puts in the pane's
/// environment. Production builds this from the environment and the CLI; a test builds it with its
/// own answer and its own workspace.
pub struct Host<'a> {
    run: &'a dyn Fn(&[&str]) -> Result<String, String>,
    workspace: Option<String>,
    me: String,
}

impl<'a> Host<'a> {
    pub fn new(
        run: &'a dyn Fn(&[&str]) -> Result<String, String>,
        workspace: Option<&str>,
        me: &str,
    ) -> Self {
        Self {
            run,
            workspace: workspace.map(str::to_string),
            me: me.to_string(),
        }
    }

    /// The live host: the `herdr` CLI, and the workspace and pane herdr named this process.
    pub fn from_env(run: &'a dyn Fn(&[&str]) -> Result<String, String>) -> Self {
        Self::new(
            run,
            std::env::var("HERDR_WORKSPACE_ID").ok().as_deref(),
            &std::env::var("HERDR_PANE_ID").unwrap_or_default(),
        )
    }
}

/// The agent pane a brief went to, named the way the pane and the comment refer to it.
#[derive(Debug, PartialEq, Eq)]
pub struct Agent {
    pub pane: String,
    pub name: String,
}

/// One row of `herdr agent list`. `name` and `display_agent` are absent until something sets them,
/// so the name falls back through them to the agent kind.
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

/// Open the note box over the task under the cursor. The note is the only thing the brief is still
/// missing, and a blank one means send the brief as it stands.
pub fn ask(screen: &Screen<'_>, prompt: &mut Option<Prompt>) -> Outcome {
    if screen.selected().is_some() {
        *prompt = Some(Prompt::note());
    }
    Outcome::quiet()
}

/// Hand the open note's brief to the agent pane, then comment on the task. The comment is written
/// only after the send succeeded, so a refused send leaves no record of a hand-off that did not
/// happen; a refused comment says so and does not pretend the send failed, because the agent has
/// the work either way.
pub async fn send(
    screen: &mut Screen<'_>,
    prompt: &mut Option<Prompt>,
    host: &Host<'_>,
) -> Outcome {
    let Some(Prompt::Note { draft }) = prompt.as_ref() else {
        return Outcome::quiet();
    };
    let Some(task) = screen.selected() else {
        return Outcome::quiet();
    };
    let id = task.id.clone();
    let text = with_note(&brief(task), &draft.text());
    let agent = match hand_off(host, &text) {
        Ok(agent) => agent,
        Err(error) => return Outcome::said(error),
    };
    *prompt = None;
    let noted = screen
        .request(async |client: &Client| client.add_comment(&id, &handed_off(&agent.name)).await)
        .await;
    match noted {
        Ok(()) => Outcome::said(format!("sent to {}", agent.name)),
        Err(error) => Outcome::said(format!("sent to {}, comment refused: {error}", agent.name)),
    }
}

/// The production herdr call, the CLI herdr names for its plugins.
pub fn cli(args: &[&str]) -> Result<String, String> {
    crate::herdr::call(args)
}

/// Write the brief into the workspace's agent pane and focus it.
fn hand_off(host: &Host<'_>, text: &str) -> Result<Agent, String> {
    let workspace = host
        .workspace
        .as_deref()
        .ok_or_else(|| "no workspace context: run this pane inside herdr".to_string())?;
    let agent = agent_in(&(host.run)(&["agent", "list"])?, workspace, &host.me)?;
    (host.run)(&["pane", "send-text", &agent.pane, &pasted(text)])?;
    // Focus is a convenience once the text is delivered: a refused focus leaves the brief in the
    // agent's input, so failing the send here would report a hand-off that did happen as one that
    // did not.
    let _ = (host.run)(&["agent", "focus", &agent.pane]);
    Ok(agent)
}

/// The agent pane of this workspace. A pane herdr lists no agent for is not a candidate, and
/// neither is our own; with several, the first herdr names wins and the status line says which.
fn agent_in(listing: &str, workspace: &str, me: &str) -> Result<Agent, String> {
    let listing: Listing =
        serde_json::from_str(listing).map_err(|error| format!("herdr agent list: {error}"))?;
    listing
        .result
        .agents
        .into_iter()
        .filter(|listed| listed.agent.is_some() && listed.pane_id != me)
        .find(|listed| listed.workspace_id == workspace)
        .map(|listed| Agent {
            name: named(&listed),
            pane: listed.pane_id,
        })
        .ok_or_else(|| "no agent pane in this workspace".to_string())
}

fn named(listed: &Listed) -> String {
    [&listed.name, &listed.display_agent, &listed.agent]
        .into_iter()
        .flatten()
        .find(|name| !name.is_empty())
        .cloned()
        .unwrap_or_else(|| "the agent".to_string())
}

/// The task as the agent reads it: its title and URL, then whichever of the due date, the
/// priority and the labels the task has, then its description. A field the task has nothing for is
/// left out rather than written empty, so the brief carries no line an agent has to discount.
fn brief(task: &TaskRow) -> String {
    let mut lines = vec![
        format!("Todoist task: {}", task.content.trim()),
        format!("url: {}", task_url(&task.id)),
    ];
    if let Some(due) = &task.due {
        lines.push(format!("due: {due}"));
    }
    if task.priority > LOWEST_PRIORITY {
        lines.push(format!("priority: p{}", 5 - task.priority));
    }
    if !task.labels.is_empty() {
        lines.push(format!("labels: {}", task.labels.join(", ")));
    }
    if !task.description.trim().is_empty() {
        lines.push(String::new());
        lines.push(task.description.trim().to_string());
    }
    lines.join("\n")
}

/// The brief with the operator's note under it. A note of nothing but whitespace is no note, which
/// is how a note box submitted blank sends the brief as it stands.
fn with_note(brief: &str, note: &str) -> String {
    if note.trim().is_empty() {
        return brief.to_string();
    }
    format!("{brief}\n\nnote: {}", note.trim())
}

/// What the comment records. The agent is named rather than its pane, which means nothing a day
/// later; WHEN is the comment's own `posted_at`, which Todoist stamps and the detail screen draws.
fn handed_off(agent: &str) -> String {
    format!("Handed to the agent {agent} from the herdr Todoist pane.")
}

const PASTE_START: &str = "\x1b[200~";
const PASTE_END: &str = "\x1b[201~";

/// The brief as one bracketed paste: a paste is inserted verbatim by any input, where the raw
/// bytes of a multi-line brief would each be read as a key, and a newline would submit it. A
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
