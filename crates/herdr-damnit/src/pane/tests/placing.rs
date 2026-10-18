use herdr_damnit_adapters::Config;
use herdr_damnit_application::{PaneFocus, PaneOpen, PanePlacement, SplitSide, Workspace};

use super::fakes::{Call, FakeMemory, FakePanes, Log, here};
use crate::pane::{Mode, run};

fn default_config() -> Config {
    Config::parse("").expect("the default config")
}

fn beside_the_caller(focus: PaneFocus) -> Call {
    Call::Open(PaneOpen {
        workspace: "w".to_string(),
        neighbor: "w:p1".to_string(),
        placement: PanePlacement::Split(SplitSide::Right),
        focus,
    })
}

fn w(text: &str) -> String {
    text.to_string()
}

#[test]
fn open_with_no_pane_live_opens_beside_the_caller_and_remembers_the_pane() {
    let log = Log::default();
    let herdr = FakePanes::showing(&log, &["w:p1"]);
    let memory = FakeMemory::remembering(&log, "w", "w:pGone");

    let outcome = run(Mode::Open, &here(), &herdr, &memory, &default_config()).expect("opened");

    assert_eq!(outcome, "opened w:p9");
    assert_eq!(
        *log.borrow(),
        vec![
            Call::Live(w("w")),
            Call::Remembered(w("w")),
            beside_the_caller(PaneFocus::Take),
            Call::Remember(w("w"), w("w:p9")),
        ]
    );
}

#[test]
fn auto_open_opens_without_taking_the_focus() {
    let log = Log::default();
    let herdr = FakePanes::showing(&log, &["w:p1"]);
    let memory = FakeMemory::sharing(&log);
    let config = Config::parse("auto_open = true").expect("a config");

    let outcome = run(Mode::AutoOpen, &here(), &herdr, &memory, &config).expect("opened");

    assert_eq!(outcome, "opened w:p9");
    assert_eq!(log.borrow()[2], beside_the_caller(PaneFocus::Leave));
}

#[test]
fn a_focus_herdr_rejects_forgets_the_pane_and_opens_a_new_one() {
    let log = Log::default();
    let mut herdr = FakePanes::showing(&log, &["w:p1", "w:p2"]);
    herdr.focus_refused = true;
    let memory = FakeMemory::remembering(&log, "w", "w:p2");

    let outcome = run(Mode::Open, &here(), &herdr, &memory, &default_config()).expect("reopened");

    assert_eq!(outcome, "opened w:p9");
    assert_eq!(
        *log.borrow(),
        vec![
            Call::Live(w("w")),
            Call::Remembered(w("w")),
            Call::Focus(w("w:p2")),
            Call::Forget(w("w")),
            beside_the_caller(PaneFocus::Take),
            Call::Remember(w("w"), w("w:p9")),
        ]
    );
}

#[test]
fn a_focus_rejected_in_focus_mode_reports_the_pane_gone() {
    let log = Log::default();
    let mut herdr = FakePanes::showing(&log, &["w:p1", "w:p2"]);
    herdr.focus_refused = true;
    let memory = FakeMemory::remembering(&log, "w", "w:p2");

    let outcome = run(Mode::Focus, &here(), &herdr, &memory, &default_config()).expect("reported");

    assert_eq!(outcome, "no dam pane in w: w:p2 is gone");
    assert_eq!(
        *log.borrow(),
        vec![
            Call::Live(w("w")),
            Call::Remembered(w("w")),
            Call::Focus(w("w:p2")),
            Call::Forget(w("w")),
        ]
    );
}

#[test]
fn a_close_herdr_rejects_forgets_the_pane_and_opens_a_new_one() {
    let log = Log::default();
    let mut herdr = FakePanes::showing(&log, &["w:p1", "w:p2"]);
    herdr.close_refused = true;
    let memory = FakeMemory::remembering(&log, "w", "w:p2");

    let outcome = run(Mode::Toggle, &here(), &herdr, &memory, &default_config()).expect("reopened");

    assert_eq!(outcome, "opened w:p9");
    assert_eq!(
        *log.borrow(),
        vec![
            Call::Live(w("w")),
            Call::Remembered(w("w")),
            Call::Close(w("w:p2")),
            Call::Forget(w("w")),
            beside_the_caller(PaneFocus::Take),
            Call::Remember(w("w"), w("w:p9")),
        ]
    );
}

#[test]
fn the_pane_id_is_remembered_only_after_a_successful_open() {
    let log = Log::default();
    let mut herdr = FakePanes::showing(&log, &["w:p1"]);
    herdr.opens = Err("herdr plugin pane open named no pane".to_string());
    let memory = FakeMemory::sharing(&log);

    let error = run(Mode::Open, &here(), &herdr, &memory, &default_config()).expect_err("failed");

    assert_eq!(error, "herdr plugin pane open named no pane");
    assert_eq!(
        *log.borrow(),
        vec![
            Call::Live(w("w")),
            Call::Remembered(w("w")),
            beside_the_caller(PaneFocus::Take),
        ]
    );
}

#[test]
fn a_refused_resize_adds_its_note_to_the_result() {
    let log = Log::default();
    let mut herdr = FakePanes::showing(&log, &["w:p1"]);
    herdr.resizes = Ok(false);
    let memory = FakeMemory::sharing(&log);
    let config = Config::parse("width = 0.3").expect("a config");

    let outcome = run(Mode::Open, &here(), &herdr, &memory, &config).expect("opened");

    assert_eq!(
        outcome,
        "opened w:p9, placement refused: herdr did not resize the pane"
    );
    assert_eq!(
        log.borrow()[3],
        Call::Resize(w("w:p1"), SplitSide::Right, 0.7)
    );
    assert_eq!(log.borrow()[4], Call::Remember(w("w"), w("w:p9")));
}

#[test]
fn a_resize_herdr_cannot_do_adds_its_error_to_the_result() {
    let log = Log::default();
    let mut herdr = FakePanes::showing(&log, &["w:p1"]);
    herdr.resizes = Err("herdr pane layout named no ratio for w:p1".to_string());
    let memory = FakeMemory::sharing(&log);
    let config = Config::parse("width = 0.3").expect("a config");

    let outcome = run(Mode::Open, &here(), &herdr, &memory, &config).expect("opened");

    assert_eq!(
        outcome,
        "opened w:p9, placement refused: herdr pane layout named no ratio for w:p1"
    );
}

#[test]
fn outside_a_workspace_every_action_refuses_before_calling_herdr() {
    let log = Log::default();
    let herdr = FakePanes::showing(&log, &["w:p1"]);
    let memory = FakeMemory::sharing(&log);
    let nowhere = Workspace {
        workspace: None,
        me: String::new(),
    };

    let error = run(Mode::Open, &nowhere, &herdr, &memory, &default_config()).expect_err("refused");

    assert_eq!(
        error,
        "no workspace context: run this action from inside herdr"
    );
    assert_eq!(*log.borrow(), vec![]);
}
