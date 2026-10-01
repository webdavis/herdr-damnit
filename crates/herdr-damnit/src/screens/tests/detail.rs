use super::*;

const SHOW_WITH_EVERY_FIELD: &str = r##"{"oid":"5d6e7f8","kind":"task",
  "subject":"refresh the roster row","path":"proj/dotfiles","body":"# why\n\nthe pin moved",
  "labels":["slow","home"],"recurrence":"every week","depends":["1a2b3c4"],
  "task":{"done":false,"priority":2,"due":"2026-09-22","deadline":"2026-09-30"}}"##;

fn shown(show: &str) -> crate::app::tests::Harness {
    let mut harness = loaded();
    harness.press(KeyCode::Enter);
    harness.answer(3, 0, show, "");
    harness
}

#[test]
fn enter_on_a_row_reads_that_object_and_opens_the_detail() {
    let mut harness = loaded();
    harness.press(KeyCode::Enter);
    assert_eq!(
        harness.lines().last().map(String::as_str),
        Some("show 5d6e7f8 --json")
    );

    harness.answer(3, 0, SHOW_WITH_EVERY_FIELD, "");

    assert_eq!(harness.app.screen, Screen::Detail);
    let drawn = render_to_text(&harness.app, 32, 20);
    for expected in [
        "refresh the roster row",
        "path: proj/dotfiles",
        "kind: task",
        "priority: p2",
        "due: 2026-09-22",
        "deadline: 2026-09-30",
        "recurrence: every week",
        "labels: slow, home",
        "why",
        "the pin moved",
    ] {
        assert!(
            drawn.contains(expected),
            "{expected:?} missing from\n{drawn}"
        );
    }
}

#[test]
fn a_dependency_is_drawn_as_its_short_oid_and_the_subject_the_model_already_holds() {
    let harness = shown(SHOW_WITH_EVERY_FIELD);

    let drawn = render_to_text(&harness.app, 32, 20);
    assert!(drawn.contains("1a2b3c4  ship the pin bump"), "{drawn}");
}

#[test]
fn a_dependency_the_model_does_not_hold_is_drawn_as_its_short_oid_alone() {
    let harness = shown(
        r#"{"oid":"5d6e7f8","kind":"task","subject":"x","depends":["ffffffffff"],
          "task":{"done":false,"priority":4}}"#,
    );

    let drawn = render_to_text(&harness.app, 32, 20);
    assert!(
        drawn.lines().any(|line| line.trim() == "fffffff"),
        "{drawn}"
    );
}

#[test]
fn an_event_draws_its_own_fields_and_its_attendees_with_their_responses() {
    let harness = shown(
        r#"{"oid":"eee","kind":"event","subject":"stand-up","event":{
          "start":"2026-09-21T09:00","end":"2026-09-21T09:15","timezone":"UTC",
          "location":"the kitchen","status":"confirmed","transparency":"busy",
          "attendees":[{"email":"a@example.test","response":"accepted"}]}}"#,
    );

    let drawn = render_to_text(&harness.app, 32, 20);
    for expected in [
        "kind: event",
        "start: 2026-09-21T09:00",
        "end: 2026-09-21T09:15",
        "timezone: UTC",
        "location: the kitchen",
        "status: confirmed",
        "transparency: busy",
        "a@example.test: accepted",
    ] {
        assert!(
            drawn.contains(expected),
            "{expected:?} missing from\n{drawn}"
        );
    }
}

#[test]
fn a_field_the_object_has_nothing_for_is_left_out_of_the_detail() {
    let harness = shown(
        r#"{"oid":"5d6e7f8","kind":"task","subject":"bare","task":{"done":false,"priority":4}}"#,
    );

    let drawn = render_to_text(&harness.app, 32, 20);
    for absent in [
        "path:",
        "due:",
        "deadline:",
        "recurrence:",
        "labels:",
        "depends:",
        "priority:",
    ] {
        assert!(
            !drawn.contains(absent),
            "{absent:?} was drawn empty in\n{drawn}"
        );
    }
}

#[test]
fn a_long_body_line_wraps_rather_than_being_cut() {
    let harness = shown(
        r#"{"oid":"5d6e7f8","kind":"task","subject":"x","task":{"done":false,"priority":4},
          "body":"one two three four five six seven eight nine ten eleven"}"#,
    );

    let drawn = render_to_text(&harness.app, 32, 20);
    assert!(
        drawn.contains("one two three four five six\nseven eight nine ten eleven"),
        "{drawn}"
    );
}

#[test]
fn escape_leaves_the_detail_for_the_screen_under_it_with_its_cursor_untouched() {
    let mut harness = loaded();
    assert!(harness.app.list.move_by(1), "the list has a second row");
    let under = harness.app.list.selected_oid().cloned();
    harness.press(KeyCode::Enter);
    harness.answer(
        3,
        0,
        r#"{"oid":"1a2b3c4","kind":"task","subject":"x","task":{"done":false,"priority":4}}"#,
        "",
    );

    harness.press(KeyCode::Esc);
    assert_eq!(harness.app.screen, Screen::List);
    assert_eq!(harness.app.list.selected_oid().cloned(), under);
}

#[test]
fn enter_with_no_object_under_the_cursor_does_nothing_and_says_nothing() {
    let mut harness = harness_with(ascii_config());
    harness.press(KeyCode::Enter);

    assert!(harness.lines().is_empty());
    assert!(harness.app.message.is_empty());
}

#[test]
fn a_show_dam_refuses_leaves_the_screen_where_it_was() {
    let mut harness = loaded();
    harness.press(KeyCode::Enter);
    harness.answer(3, 2, "", "dam: no object matches \"5d6e7f8\"");

    assert_eq!(harness.app.screen, Screen::List);
    assert_eq!(harness.app.message, "no object matches \"5d6e7f8\"");
}
