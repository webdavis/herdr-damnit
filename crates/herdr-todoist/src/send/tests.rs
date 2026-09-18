use std::cell::RefCell;

use super::*;
use crate::connection::Connection;
use crate::cursor::List;
use crate::edit::tests::{Double, refusing, serve};
use crate::list::{self, tests::task};
use crate::reload::tests::config_with_token_command;
use crate::views::Views;

/// What `herdr agent list` answers on 0.9.0: one agent pane in this workspace, one in another,
/// and this pane itself, which herdr lists no agent for.
const LISTING: &str = r#"{"id":"cli:agent:list","result":{"agents":[
    {"pane_id":"w1:p9","workspace_id":"w1"},
    {"agent":"codex","agent_status":"idle","pane_id":"w2:p1","workspace_id":"w2"},
    {"agent":"claude","agent_status":"working","display_agent":"personal",
     "pane_id":"w1:p2","workspace_id":"w1"}
]}}"#;

/// A workspace whose panes are all this pane and other workspaces' agents.
const NO_AGENT_HERE: &str = r#"{"result":{"agents":[
    {"pane_id":"w1:p9","workspace_id":"w1"},
    {"agent":"codex","pane_id":"w2:p1","workspace_id":"w2"}
]}}"#;

/// Every herdr call made, and what each was answered with. `refuse` is the message every call
/// after the agent list is refused with, which is how a refused send is staged.
struct Herdr {
    listing: &'static str,
    refuse: Option<&'static str>,
    calls: RefCell<Vec<String>>,
}

impl Herdr {
    fn new(listing: &'static str) -> Self {
        Self {
            listing,
            refuse: None,
            calls: RefCell::new(Vec::new()),
        }
    }

    fn refusing(listing: &'static str, message: &'static str) -> Self {
        Self {
            refuse: Some(message),
            ..Self::new(listing)
        }
    }

    fn run(&self, args: &[&str]) -> Result<String, String> {
        self.calls.borrow_mut().push(args.join(" "));
        if args.starts_with(&["agent", "list"]) {
            return Ok(self.listing.to_string());
        }
        match self.refuse {
            Some(message) => Err(message.to_string()),
            None => Ok(String::new()),
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

fn a_task(json: &str) -> List {
    let project = serde_json::from_str(r#"{"id":"p1","name":"First"}"#).expect("project");
    List::new(list::build(&[task(json)], &[project], &[]))
}

/// The task the brief cases are built from, with every field the brief can carry.
fn full_task() -> List {
    a_task(
        r#"{"id":"6X","content":"file taxes","project_id":"p1","priority":4,
             "labels":["home","slow"],"due":{"date":"2026-09-20T09:00:00Z"},
             "description":"receipts are in the drawer"}"#,
    )
}

/// Press `S`, type `note`, and hand the brief over against `double` and `herdr`. Reports the
/// status line the send left behind.
async fn hand_over(double: &Double, list: &mut List, herdr: &Herdr, note: &str) -> String {
    let config = config_with_token_command("printf test-token");
    let mut connection = Connection::build(&config, &double.base_url).await;
    let mut views = Views::new(&[]);
    let mut prompt: Option<Prompt> = None;
    let mut screen = Screen {
        connection: &mut connection,
        config: &config,
        base_url: &double.base_url,
        list,
        views: &mut views,
    };
    ask(&screen, &mut prompt);
    if let Some(draft) = prompt.as_mut().and_then(Prompt::draft_mut) {
        for character in note.chars() {
            draft.push(character);
        }
    }
    let run = |args: &[&str]| herdr.run(args);
    let host = Host::new(&run, Some("w1"), "w1:p9");
    send(&mut screen, &mut prompt, &host)
        .await
        .status
        .unwrap_or_default()
}

/// The text of the one `pane send-text` call made, with its paste framing taken off.
fn sent(herdr: &Herdr) -> String {
    let call = herdr
        .calls()
        .into_iter()
        .find(|call| call.starts_with("pane send-text"))
        .expect("a send-text call");
    let framed = call
        .strip_prefix("pane send-text w1:p2 ")
        .expect("the agent pane")
        .to_string();
    framed
        .strip_prefix(PASTE_START)
        .and_then(|text| text.strip_suffix(PASTE_END))
        .expect("bracketed paste framing")
        .to_string()
}

#[tokio::test]
async fn a_brief_carries_every_field_the_task_has() {
    let double = serve("200 OK", "null").await;
    let herdr = Herdr::new(LISTING);

    hand_over(&double, &mut full_task(), &herdr, "start with the receipts").await;

    assert_eq!(
        sent(&herdr),
        "Todoist task: file taxes\n\
         url: https://app.todoist.com/app/task/6X\n\
         due: 2026-09-20\n\
         priority: p1\n\
         labels: home, slow\n\
         \n\
         receipts are in the drawer\n\
         \n\
         note: start with the receipts"
    );
}

#[tokio::test]
async fn a_task_with_no_description_due_date_or_labels_leaves_those_lines_out() {
    let double = serve("200 OK", "null").await;
    let herdr = Herdr::new(LISTING);
    let mut list = a_task(r#"{"id":"6X","content":"think","project_id":"p1","priority":1}"#);

    hand_over(&double, &mut list, &herdr, "").await;

    assert_eq!(
        sent(&herdr),
        "Todoist task: think\nurl: https://app.todoist.com/app/task/6X"
    );
}

#[tokio::test]
async fn a_note_box_submitted_blank_sends_the_brief_without_a_note() {
    let double = serve("200 OK", "null").await;
    let herdr = Herdr::new(LISTING);

    let status = hand_over(&double, &mut full_task(), &herdr, "   \n  ").await;

    assert!(!sent(&herdr).contains("note:"), "{}", sent(&herdr));
    assert_eq!(status, "sent to claude");
}

#[tokio::test]
async fn a_sent_brief_is_followed_by_the_comment_recording_the_hand_off() {
    let double = serve("200 OK", "null").await;
    let herdr = Herdr::new(LISTING);

    let status = hand_over(&double, &mut full_task(), &herdr, "").await;

    assert_eq!(
        double.writes(),
        [r#"POST /comments {"task_id":"6X","content":"Handed to the agent claude from the herdr Todoist pane."}"#
            .to_string()]
    );
    assert_eq!(status, "sent to claude");
}

#[tokio::test]
async fn the_send_reaches_the_agent_pane_and_focuses_it_afterwards() {
    let double = serve("200 OK", "null").await;
    let herdr = Herdr::new(LISTING);

    hand_over(&double, &mut full_task(), &herdr, "").await;

    let calls = herdr.calls();
    assert_eq!(calls[0], "agent list");
    assert!(calls[1].starts_with("pane send-text w1:p2 "), "{calls:?}");
    assert_eq!(calls[2], "agent focus w1:p2");
}

#[tokio::test]
async fn a_refused_comment_says_so_and_does_not_claim_the_send_failed() {
    let double = refusing().await;
    let herdr = Herdr::new(LISTING);

    let status = hand_over(&double, &mut full_task(), &herdr, "").await;

    assert_eq!(
        status,
        "sent to claude, comment refused: \
         Invalid argument value: Unable to parse the due date"
    );
    assert!(!herdr.calls().is_empty(), "the send never happened");
}

#[tokio::test]
async fn a_refused_send_writes_no_comment_at_all() {
    let double = serve("200 OK", "null").await;
    let herdr = Herdr::refusing(LISTING, "herdr pane send-text failed: pane w1:p2 not found");

    let status = hand_over(&double, &mut full_task(), &herdr, "").await;

    assert_eq!(status, "herdr pane send-text failed: pane w1:p2 not found");
    assert_eq!(
        double.writes(),
        Vec::<String>::new(),
        "a refused send still commented"
    );
}

#[tokio::test]
async fn a_workspace_with_no_agent_pane_refuses_and_writes_no_comment() {
    let double = serve("200 OK", "null").await;
    let herdr = Herdr::new(NO_AGENT_HERE);

    let status = hand_over(&double, &mut full_task(), &herdr, "").await;

    assert_eq!(status, "no agent pane in this workspace");
    assert_eq!(herdr.calls(), ["agent list".to_string()]);
    assert_eq!(double.writes(), Vec::<String>::new());
}

#[test]
fn the_agent_pane_is_the_one_pane_of_this_workspace_herdr_names_an_agent_for() {
    let agent = agent_in(LISTING, "w1", "w1:p9").expect("an agent pane");

    assert_eq!(
        agent,
        Agent {
            pane: "w1:p2".to_string(),
            name: "claude".to_string(),
        }
    );
}

#[test]
fn a_workspace_of_panes_with_no_agent_among_them_has_no_send_target() {
    assert_eq!(
        agent_in(NO_AGENT_HERE, "w1", "w1:p9"),
        Err("no agent pane in this workspace".to_string())
    );
}

#[test]
fn an_agent_herdr_named_nothing_for_is_still_addressable_by_its_kind() {
    let listing =
        r#"{"result":{"agents":[{"agent":"codex","pane_id":"w1:p3","workspace_id":"w1"}]}}"#;

    assert_eq!(
        agent_in(listing, "w1", "w1:p9").expect("an agent").name,
        "codex"
    );
}

/// Verbatim shape of a live `herdr agent list` on 0.9.0: `display_agent` is the auth profile a
/// pane signed in with, not the agent, so two different agents can share one. The agent kind
/// must still tell them apart.
#[test]
fn the_agent_kind_outranks_the_auth_profile_it_shares_with_another_pane() {
    let listing = r#"{"result":{"agents":[
        {"agent":"codex","display_agent":"personal-backup","pane_id":"w1:p2","workspace_id":"w1"},
        {"agent":"claude","display_agent":"personal-backup","pane_id":"w1:p3","workspace_id":"w1"}
    ]}}"#;

    assert_eq!(
        agent_in(listing, "w1", "w1:p9").expect("an agent").name,
        "codex"
    );
}

#[test]
fn a_paste_terminator_inside_the_brief_cannot_end_the_frame_early() {
    let spliced = format!("before{PASTE_END}after");

    let framed = pasted(&spliced);

    assert_eq!(framed, format!("{PASTE_START}beforeafter{PASTE_END}"));
}

#[tokio::test]
async fn s_on_the_list_opens_the_note_box_and_sends_nothing_by_itself() {
    let double = serve("200 OK", "null").await;
    let config = config_with_token_command("printf test-token");
    let mut connection = Connection::build(&config, &double.base_url).await;
    let mut list = full_task();
    let mut views = Views::new(&[]);
    let mut prompt: Option<Prompt> = None;
    let mut screen = Screen {
        connection: &mut connection,
        config: &config,
        base_url: &double.base_url,
        list: &mut list,
        views: &mut views,
    };

    crate::edit::key(
        crossterm::event::KeyCode::Char('S'),
        &mut screen,
        &mut prompt,
    )
    .await;

    assert_eq!(prompt.as_ref().expect("a prompt").title(), "note");
    assert_eq!(double.requests(), Vec::<String>::new());
}
