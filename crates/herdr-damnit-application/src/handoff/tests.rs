use std::sync::Mutex;

use herdr_damnit_domain::{Kind, Object, Oid, Priority, TaskFields};

use super::*;

const LISTING: &str = r#"{"result":{"agents":[
  {"pane_id":"w1:p1","workspace_id":"w1","agent":null},
  {"pane_id":"w1:p2","workspace_id":"w1","agent":"claude","name":"planner"},
  {"pane_id":"w2:p9","workspace_id":"w2","agent":"codex"}
]}}"#;

/// `Herdr` is `Send + Sync`, so the fake records through a `Mutex`.
struct FakeHerdr {
    calls: Mutex<Vec<Vec<String>>>,
    listing: String,
    refuse_send: bool,
    refuse_focus: bool,
}

impl FakeHerdr {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            listing: LISTING.to_string(),
            refuse_send: false,
            refuse_focus: false,
        }
    }

    fn verbs(&self) -> Vec<String> {
        self.calls
            .lock()
            .expect("the calls")
            .iter()
            .map(|call| call[..2.min(call.len())].join(" "))
            .collect()
    }

    fn sent_text(&self) -> String {
        self.calls
            .lock()
            .expect("the calls")
            .iter()
            .find(|call| call.first().map(String::as_str) == Some("pane"))
            .and_then(|call| call.last().cloned())
            .unwrap_or_default()
    }
}

impl Herdr for FakeHerdr {
    fn call(&self, args: &[&str]) -> Result<String, String> {
        self.calls
            .lock()
            .expect("the calls")
            .push(args.iter().map(|word| word.to_string()).collect());
        match args {
            ["agent", "list"] => Ok(self.listing.clone()),
            ["pane", "send-text", ..] if self.refuse_send => Err("refused".to_string()),
            ["agent", "focus", ..] if self.refuse_focus => Err("refused".to_string()),
            _ => Ok(r#"{"result":{}}"#.to_string()),
        }
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
            attached: None,
        }),
        event: None,
    }
}

#[test]
fn the_brief_goes_to_the_named_agent_of_this_workspace() {
    let herdr = FakeHerdr::new();
    let HandOff::Sent { agent, .. } = hand_off(&herdr, &here(), &object(), "", "") else {
        panic!("expected a send");
    };

    assert_eq!(agent.pane, "w1:p2");
    assert_eq!(agent.name, "planner");
    assert_eq!(
        herdr.verbs(),
        vec![
            "agent list".to_string(),
            "pane send-text".to_string(),
            "agent focus".to_string(),
        ]
    );
}

#[test]
fn an_agent_pane_in_another_workspace_is_not_a_candidate() {
    let herdr = FakeHerdr::new();
    let elsewhere = Workspace {
        workspace: Some("w3".to_string()),
        me: "w3:p1".to_string(),
    };
    let HandOff::Refused(message) = hand_off(&herdr, &elsewhere, &object(), "", "") else {
        panic!("expected a refusal");
    };
    assert_eq!(message, "no agent pane in this workspace.");
}

#[test]
fn a_pane_herdr_lists_no_agent_for_is_passed_over_for_the_one_it_does() {
    let herdr = FakeHerdr::new();
    let HandOff::Sent { agent, .. } = hand_off(&herdr, &here(), &object(), "", "") else {
        panic!("expected a send");
    };
    assert_ne!(agent.pane, "w1:p1");
}

#[test]
fn the_brief_is_one_bracketed_paste_and_carries_no_return() {
    let herdr = FakeHerdr::new();
    hand_off(&herdr, &here(), &object(), "start here", "");

    let sent = herdr.sent_text();
    assert!(sent.starts_with("\u{1b}[200~"), "{sent:?}");
    assert!(sent.ends_with("\u{1b}[201~"), "{sent:?}");
    assert!(sent.contains("dam task: file taxes"), "{sent:?}");
    assert!(sent.contains("note: start here"), "{sent:?}");
    assert!(
        !sent.trim_end_matches("\u{1b}[201~").ends_with('\n'),
        "{sent:?}"
    );
}

#[test]
fn a_paste_terminator_inside_the_brief_cannot_end_the_frame_early() {
    let framed = pasted("before \u{1b}[201~ after");
    assert_eq!(framed.matches("\u{1b}[201~").count(), 1, "{framed:?}");
}

/// A terminator spliced together by removing the one before it must not survive either.
#[test]
fn two_terminators_sharing_their_characters_are_both_removed() {
    let framed = pasted("\u{1b}[20\u{1b}[201~1~");
    assert_eq!(framed.matches("\u{1b}[201~").count(), 1, "{framed:?}");
}

#[test]
fn a_workspace_with_no_agent_pane_says_so_and_sends_nothing() {
    let herdr = FakeHerdr {
        listing: r#"{"result":{"agents":[]}}"#.to_string(),
        ..FakeHerdr::new()
    };
    let HandOff::Refused(message) = hand_off(&herdr, &here(), &object(), "", "") else {
        panic!("expected a refusal");
    };

    assert_eq!(message, "no agent pane in this workspace.");
    assert_eq!(herdr.verbs(), vec!["agent list".to_string()]);
}

#[test]
fn this_pane_is_never_its_own_agent() {
    let herdr = FakeHerdr::new();
    let mine = Workspace {
        workspace: Some("w1".to_string()),
        me: "w1:p2".to_string(),
    };
    let HandOff::Refused(message) = hand_off(&herdr, &mine, &object(), "", "") else {
        panic!("expected a refusal");
    };
    assert_eq!(message, "no agent pane in this workspace.");
}

#[test]
fn a_listing_herdr_answered_with_something_else_is_refused_rather_than_guessed_at() {
    let herdr = FakeHerdr {
        listing: "not json".to_string(),
        ..FakeHerdr::new()
    };
    let HandOff::Refused(message) = hand_off(&herdr, &here(), &object(), "", "") else {
        panic!("expected a refusal");
    };
    assert!(message.starts_with("herdr agent list: "), "{message}");
}

#[test]
fn an_agent_pane_with_no_name_of_its_own_is_called_by_its_kind() {
    let herdr = FakeHerdr {
        listing:
            r#"{"result":{"agents":[{"pane_id":"w1:p7","workspace_id":"w1","agent":"codex"}]}}"#
                .to_string(),
        ..FakeHerdr::new()
    };
    let HandOff::Sent { agent, .. } = hand_off(&herdr, &here(), &object(), "", "") else {
        panic!("expected a send");
    };
    assert_eq!(agent.name, "codex");
}

#[test]
fn an_agent_pane_that_names_nothing_at_all_is_called_the_agent() {
    let herdr = FakeHerdr {
        listing: r#"{"result":{"agents":[{"pane_id":"w1:p7","workspace_id":"w1","agent":""}]}}"#
            .to_string(),
        ..FakeHerdr::new()
    };
    let HandOff::Sent { agent, .. } = hand_off(&herdr, &here(), &object(), "", "") else {
        panic!("expected a send");
    };
    assert_eq!(agent.name, "the agent");
}

#[test]
fn a_refused_send_is_a_refusal_and_a_refused_focus_is_not() {
    let refused_send = FakeHerdr {
        refuse_send: true,
        ..FakeHerdr::new()
    };
    assert!(matches!(
        hand_off(&refused_send, &here(), &object(), "", ""),
        HandOff::Refused(_)
    ));

    let refused_focus = FakeHerdr {
        refuse_focus: true,
        ..FakeHerdr::new()
    };
    assert!(matches!(
        hand_off(&refused_focus, &here(), &object(), "", ""),
        HandOff::Sent { .. }
    ));
}

#[test]
fn a_configured_label_comes_back_as_the_write_the_caller_submits() {
    let herdr = FakeHerdr::new();
    let HandOff::Sent { label, .. } = hand_off(&herdr, &here(), &object(), "", "handed-off") else {
        panic!("expected a send");
    };

    assert_eq!(
        label,
        Some(vec![
            "edit".to_string(),
            "1a2b3c4".to_string(),
            "--label".to_string(),
            "handed-off".to_string(),
            "--json".to_string(),
        ])
    );
}

#[test]
fn an_empty_label_writes_nothing_and_the_status_line_is_the_whole_record() {
    let herdr = FakeHerdr::new();
    let HandOff::Sent { label, .. } = hand_off(&herdr, &here(), &object(), "", "  ") else {
        panic!("expected a send");
    };
    assert_eq!(label, None);
}

#[test]
fn a_pane_outside_herdr_says_so_rather_than_guessing_a_workspace() {
    let herdr = FakeHerdr::new();
    let nowhere = Workspace {
        workspace: None,
        me: String::new(),
    };
    let HandOff::Refused(message) = hand_off(&herdr, &nowhere, &object(), "", "") else {
        panic!("expected a refusal");
    };
    assert_eq!(message, "no workspace context: run this pane inside herdr.");
    assert!(herdr.verbs().is_empty());
}
