use super::requests::note_then_open;
use super::*;
use herdr_damnit_application::fakes::{FakeMemory, FakePanes, PaneLog};
use support::{here, requested, scratch_state};

mod placing;
mod support;

fn default_config() -> Config {
    Config::parse("").expect("the default config")
}

fn panes(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

#[test]
fn open_opens_when_no_pane_is_live() {
    assert_eq!(decide(Mode::Open, None, &panes(&["w:p1"])), Decision::Open);
}

#[test]
fn open_focuses_the_pane_it_already_has() {
    assert_eq!(
        decide(Mode::Open, Some("w:p2"), &panes(&["w:p1", "w:p2"])),
        Decision::Focus("w:p2".to_string())
    );
}

#[test]
fn toggle_closes_a_live_pane_and_opens_when_there_is_none() {
    assert_eq!(
        decide(Mode::Toggle, Some("w:p2"), &panes(&["w:p2"])),
        Decision::Close("w:p2".to_string())
    );
    assert_eq!(
        decide(Mode::Toggle, None, &panes(&["w:p1"])),
        Decision::Open
    );
}

#[test]
fn a_pane_open_in_another_workspace_is_not_this_workspaces_pane_so_toggle_opens_a_second() {
    assert_eq!(
        decide(Mode::Toggle, None, &panes(&["w2:p1"])),
        Decision::Open
    );
}

#[test]
fn auto_open_opens_a_closed_pane_and_leaves_an_open_one_alone() {
    assert_eq!(
        decide(Mode::AutoOpen, None, &panes(&["w:p1"])),
        Decision::Open
    );
    assert_eq!(
        decide(Mode::AutoOpen, Some("w:p2"), &panes(&["w:p2"])),
        Decision::LeaveOpen("w:p2".to_string())
    );
}

#[test]
fn auto_open_does_nothing_at_all_when_the_config_has_not_asked_for_it() {
    let log = PaneLog::default();
    let herdr = FakePanes::showing(&log, &["w:p1"]);
    let memory = FakeMemory::sharing(&log);

    let outcome = auto_open(&here(), &herdr, &memory, &default_config()).expect("no call at all");

    assert_eq!(outcome, "auto_open is off");
    assert_eq!(*log.borrow(), vec![]);
}

#[test]
fn a_remembered_pane_that_exited_counts_as_no_pane() {
    assert_eq!(
        decide(Mode::Toggle, Some("w:pGone"), &panes(&["w:p1"])),
        Decision::Open
    );
    assert_eq!(
        decide(Mode::Focus, Some("w:pGone"), &panes(&["w:p1"])),
        Decision::NothingToFocus
    );
}

#[test]
fn focus_never_opens_a_pane() {
    assert_eq!(
        decide(Mode::Focus, None, &panes(&["w:p1"])),
        Decision::NothingToFocus
    );
}

#[test]
fn a_view_number_past_the_end_names_how_many_views_there_are() {
    let herdr = FakePanes::showing(&PaneLog::default(), &[]);
    let state = scratch_state("view-past-the-end");

    let error = view("4", &here(), &herdr, &state, &default_config()).expect_err("refuses");

    assert_eq!(error, "no view 4: this config has 1 views, 1 being open");
}

#[test]
fn a_view_argument_that_is_not_a_number_is_refused() {
    let herdr = FakePanes::showing(&PaneLog::default(), &[]);
    let state = scratch_state("view-not-a-number");

    let error = view("today", &here(), &herdr, &state, &default_config()).expect_err("refuses");

    assert!(error.contains("not a view number"), "{error}");
}

#[test]
fn a_failed_view_open_clears_its_own_request() {
    let state = scratch_state("failed-view");

    let error = note_then_open("today", &state, || Err("herdr is not there".to_string()))
        .expect_err("failed");

    assert_eq!(error, "herdr is not there");
    assert_eq!(
        requested(&state),
        None,
        "a failed open must clear its request"
    );
}

#[test]
fn a_view_that_opened_leaves_its_note_for_the_pane_and_says_what_it_shows() {
    let state = scratch_state("opened-view");

    let outcome = note_then_open("work", &state, || Ok("opened w:p3".to_string()))
        .expect("the open succeeded");

    assert_eq!(outcome, "opened w:p3, showing work");
    assert_eq!(requested(&state), Some("work".to_string()));
}

#[test]
fn the_status_action_leaves_the_status_request_for_the_pane() {
    let state = scratch_state("status-action");

    let outcome = note_then_open(STATUS_REQUEST, &state, || Ok("focused w:p3".to_string()))
        .expect("the open succeeded");

    assert_eq!(outcome, "focused w:p3, showing status");
    assert_eq!(requested(&state).as_deref(), Some("status"));
}
