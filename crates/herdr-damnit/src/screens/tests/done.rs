use super::*;

const NO_COMMITS: &str = r#"{"commits":[]}"#;

const WATERED: &str = r#"{"objects":[
  {"oid":"9a0b1c2","kind":"task","subject":"water the plants","path":"proj/home",
   "task":{"done":true,"priority":4}}]}"#;

const EVERYTHING_SHOWN: [&str; 4] = [
    "log --json",
    "ls !done --json",
    "ls done --json",
    "status --json",
];

fn on_done() -> crate::app::tests::Harness {
    let mut harness = loaded();
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);
    harness.answer(3, 0, r#"{"objects":[]}"#, "");
    harness.answer(4, 0, NO_COMMITS, "");
    harness
}

fn asked_since(harness: &crate::app::tests::Harness, before: usize) -> Vec<String> {
    let mut asked = harness.lines()[before..].to_vec();
    asked.sort();
    asked
}

#[test]
fn entering_the_done_screen_reads_the_done_list_and_the_log() {
    let mut harness = loaded();
    let before = harness.lines().len();
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);

    assert_eq!(
        harness.lines()[before..],
        ["ls done --json".to_string(), "log --json".to_string()]
    );
}

#[test]
fn a_task_completed_after_done_was_opened_is_drawn_on_done() {
    let mut harness = on_done();
    let drawn = render_to_text(&harness.app, 32, 4);
    assert!(drawn.contains("nothing completed yet"), "{drawn}");

    harness.press(KeyCode::Tab);
    harness.select("9a0b1c2");
    harness.press(KeyCode::Char('X'));
    harness.answer(5, 0, "{}", "");
    harness.answer(6, 0, LS, "");
    harness.answer(7, 0, CLEAN, "");
    harness.answer(8, 0, WATERED, "");
    harness.answer(9, 0, NO_COMMITS, "");
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);

    let drawn = render_to_text(&harness.app, 32, 4);
    assert_eq!(
        drawn.lines().take(3).collect::<Vec<_>>(),
        [
            "dam  done                 1 done",
            "not committed",
            "  water the plants"
        ],
        "{drawn}"
    );
}

#[test]
fn r_on_done_reads_the_done_list_and_the_log_beside_the_list_and_the_status() {
    let mut harness = on_done();
    let before = harness.lines().len();
    harness.press(KeyCode::Char('R'));

    assert_eq!(asked_since(&harness, before), EVERYTHING_SHOWN);
}

#[test]
fn a_write_once_done_was_opened_refreshes_done_too_and_reads_nothing_twice() {
    let mut harness = on_done();
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Char('A'));
    let before = harness.lines().len();
    harness.answer(5, 0, "{}", "");

    assert_eq!(asked_since(&harness, before), EVERYTHING_SHOWN);
}
