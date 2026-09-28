use super::*;

const FULL_STATUS: &str = r#"{
  "staged":[
    {"oid":"1a2b3c4","op":"create","before":null,
     "after":{"oid":"1a2b3c4","kind":"task","subject":"ship the pin bump"}}],
  "unstaged":[
    {"oid":"9a0b1c2","op":"update",
     "before":{"oid":"9a0b1c2","kind":"task","subject":"water the plants"},
     "after":{"oid":"9a0b1c2","kind":"task","subject":"water the plants"}}],
  "unpushed":[{"remote":"example","commits":1}],
  "conflicts":[{"oid":"3d4e5f6","remote":"example",
    "ours":{"oid":"3d4e5f6","kind":"task","subject":"mine"},
    "theirs":{"oid":"3d4e5f6","kind":"task","subject":"theirs"}}],
  "notices":[{"kind":"pull_failed","remote":"example","why":"unreachable"}]}"#;

fn on_status() -> crate::app::tests::Harness {
    let mut harness = harness_with(ascii_config());
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(0, 0, FULL_STATUS, "");
    harness.app.screen = Screen::Status;
    harness
}

#[test]
fn the_status_screen_draws_the_four_sections_in_dams_order() {
    let harness = on_status();

    let drawn = render_to_text(&harness.app, 32, 12);
    let headings: Vec<&str> = drawn
        .lines()
        .filter(|line| ["Staged", "Working", "Unpushed", "Notices"].contains(line))
        .collect();
    assert_eq!(headings, vec!["Staged", "Working", "Unpushed", "Notices"]);
    assert!(drawn.contains("example: pull failed"), "{drawn}");
}

#[test]
fn a_status_row_draws_its_own_mark_and_a_row_with_none_draws_none() {
    let harness = on_status();

    let drawn = render_to_text(&harness.app, 32, 12);
    assert!(
        drawn.lines().any(|line| line == "  ^ example  1 commit"),
        "{drawn}"
    );
    assert!(
        drawn
            .lines()
            .any(|line| line.starts_with("  example: pull failed")),
        "{drawn}"
    );
    assert!(
        drawn.lines().any(|line| line.starts_with("  + new")),
        "{drawn}"
    );
}

#[test]
fn the_status_screen_names_itself_and_counts_the_stage_in_dams_own_words() {
    let harness = on_status();

    assert_eq!(harness.app.header(Instant::now()), "dam  status");
    assert_eq!(
        counts(&harness.app),
        "1 staged  1 changed  1 unpushed  1 notice"
    );
}

#[test]
fn a_clean_status_screen_says_so_in_dams_own_words() {
    let mut harness = harness_with(ascii_config());
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(0, 0, CLEAN, "");
    harness.app.screen = Screen::Status;

    assert!(
        render_to_text(&harness.app, 32, 6).contains("nothing staged, nothing changed"),
        "{}",
        render_to_text(&harness.app, 32, 6)
    );
}
