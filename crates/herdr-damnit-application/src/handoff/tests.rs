mod choice;
mod paste;
mod send;

use std::sync::Mutex;

use herdr_damnit_domain::{Kind, Object, Oid, Priority, TaskFields};

use super::*;

struct FakeAgents {
    agents: Result<Vec<ListedAgent>, String>,
    refuse_send: bool,
    refuse_focus: bool,
    calls: Mutex<Vec<&'static str>>,
    sent: Mutex<String>,
}

impl FakeAgents {
    fn new() -> Self {
        Self::listing(vec![
            agent("w1:p1", "w1", None),
            ListedAgent {
                name: Some("planner".to_string()),
                ..agent("w1:p2", "w1", Some("claude"))
            },
            agent("w2:p9", "w2", Some("codex")),
        ])
    }

    fn listing(agents: Vec<ListedAgent>) -> Self {
        Self {
            agents: Ok(agents),
            refuse_send: false,
            refuse_focus: false,
            calls: Mutex::new(Vec::new()),
            sent: Mutex::new(String::new()),
        }
    }

    fn calls(&self) -> Vec<&'static str> {
        self.calls.lock().expect("the calls").clone()
    }

    fn sent_text(&self) -> String {
        self.sent.lock().expect("the text").clone()
    }

    fn record(&self, call: &'static str, refused: bool) -> Result<(), String> {
        self.calls.lock().expect("the calls").push(call);
        match refused {
            true => Err("refused".to_string()),
            false => Ok(()),
        }
    }
}

impl AgentPanes for FakeAgents {
    fn agents(&self) -> Result<Vec<ListedAgent>, String> {
        self.calls.lock().expect("the calls").push("agents");
        self.agents.clone()
    }

    fn send_text(&self, _pane: &str, text: &str) -> Result<(), String> {
        *self.sent.lock().expect("the text") = text.to_string();
        self.record("send_text", self.refuse_send)
    }

    fn focus_agent(&self, _pane: &str) -> Result<(), String> {
        self.record("focus_agent", self.refuse_focus)
    }
}

fn agent(pane: &str, workspace: &str, kind: Option<&str>) -> ListedAgent {
    ListedAgent {
        pane: pane.to_string(),
        workspace: workspace.to_string(),
        kind: kind.map(str::to_string),
        name: None,
        profile: None,
    }
}

fn here() -> Workspace {
    Workspace {
        workspace: Some("w1".to_string()),
        me: "w1:p1".to_string(),
    }
}

fn object() -> Object {
    Object {
        oid: Oid::new("1a2b3c4"),
        kind: Kind::Task,
        subject: "file taxes".to_string(),
        body: String::new(),
        path: String::new(),
        labels: Vec::new(),
        depends: Vec::new(),
        recurrence: None,
        task: Some(TaskFields {
            done: false,
            priority: Priority::default(),
            due: None,
            deadline: None,
            attached_event: None,
        }),
        event: None,
    }
}
