mod choice;
mod paste;
mod send;

use herdr_damnit_domain::{Kind, Object, Oid, Priority, TaskFields};

use super::*;
use crate::fakes::FakeAgents;

fn three_agents() -> FakeAgents {
    FakeAgents::listing(vec![
        agent("w1:p1", "w1", None),
        ListedAgent {
            name: Some("planner".to_string()),
            ..agent("w1:p2", "w1", Some("claude"))
        },
        agent("w2:p9", "w2", Some("codex")),
    ])
}

fn call_names(agents: &FakeAgents) -> Vec<String> {
    agents
        .calls()
        .iter()
        .map(|call| call.split(' ').next().unwrap_or_default().to_string())
        .collect()
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
