use super::*;
use crate::config;
use crate::icons::Marks;
use crate::list;
use crate::list::tests::{marks, plain_marks, task};
use crate::prompt::Input;
use unicode_width::UnicodeWidthStr;

/// The rows of an off-screen terminal, each trimmed of the blank cells to its right.
pub(crate) fn lines_of(terminal: &ratatui::Terminal<ratatui::backend::TestBackend>) -> Vec<String> {
    let buffer = terminal.backend().buffer().clone();
    (0..buffer.area.height)
        .map(|row| {
            (0..buffer.area.width)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

fn list_of_two() -> List {
    let project = serde_json::from_str(r#"{"id":"p1","name":"First"}"#).expect("project");
    List::new(list::build(
        &[
            task(r#"{"id":"1","content":"first","project_id":"p1","child_order":1}"#),
            task(r#"{"id":"2","content":"second","project_id":"p1","child_order":2}"#),
        ],
        &[project],
        &[],
        &crate::list::tests::marks(),
    ))
}

fn views_of(names: &[&str]) -> Views {
    Views::new(
        &names
            .iter()
            .map(|name| config::View {
                name: name.to_string(),
                filter: format!("#{name}"),
            })
            .collect::<Vec<_>>(),
    )
}

/// Draw one frame into an off-screen terminal and report the buffer as lines.
fn frame_of(views: &Views, list: &List, prompt: Option<&Prompt>) -> Vec<String> {
    frame_of_screen(views, list, prompt, false)
}

fn frame_of_screen(
    views: &Views,
    list: &List,
    prompt: Option<&Prompt>,
    completed: bool,
) -> Vec<String> {
    frame_of_width(views, list, prompt, completed, 80, "2 open tasks")
}

/// Draw into an off-screen terminal the width of a real side pane, roughly a third of a
/// terminal, so a hint or status line too wide to fit shows up truncated.
#[allow(clippy::too_many_arguments)]
fn frame_of_width(
    views: &Views,
    list: &List,
    prompt: Option<&Prompt>,
    completed: bool,
    width: u16,
    status: &str,
) -> Vec<String> {
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 8)).expect("terminal");
    let mut state = ListState::default();
    terminal
        .draw(|frame| {
            draw(
                frame,
                status,
                &Chrome {
                    views,
                    palette: &crate::theme::resolve(None),
                },
                list,
                &mut state,
                prompt,
                completed,
            )
        })
        .expect("draw");
    lines_of(&terminal)
}

/// Draw into an off-screen terminal and report which line came out highlighted.
/// The status line and the rows a pane draws when it is showing what it last read and holds a
/// write the network has not taken yet.
#[test]
fn a_stale_pane_with_a_write_waiting_says_both_inside_a_side_pane_s_columns() {
    let mut list = list_of_two();
    crate::list::mark_waiting(list.rows_mut(), &["1"]);

    let frame = frame_of_width(
        &views_of(&["today"]),
        &list,
        None,
        false,
        NARROW,
        "stale 5m +1",
    );

    assert_eq!(frame[0], "todoist  all  stale 5m +1");
    assert_eq!(
        frame[2], "  + first",
        "the waiting mark leads the task's line"
    );
    assert_eq!(frame[3], "  second", "an unwaiting row is untouched");
    for line in &frame {
        assert!(
            line.width() <= NARROW as usize,
            "{line:?} is wider than the pane"
        );
    }
}

fn highlighted_line(list: &List) -> String {
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 6)).expect("terminal");
    let mut state = ListState::default();
    terminal
        .draw(|frame| {
            draw(
                frame,
                "2 open tasks",
                &Chrome {
                    views: &views_of(&[]),
                    palette: &crate::theme::resolve(None),
                },
                list,
                &mut state,
                None,
                false,
            )
        })
        .expect("draw");
    let buffer = terminal.backend().buffer().clone();
    let mut line = String::new();
    for row in 0..buffer.area.height {
        for column in 0..buffer.area.width {
            let cell = &buffer[(column, row)];
            if cell.modifier.contains(Modifier::REVERSED) {
                line.push_str(cell.symbol());
            }
        }
    }
    line.trim().to_string()
}

#[test]
fn the_highlight_is_drawn_on_the_selected_task() {
    let mut list = list_of_two();

    assert_eq!(highlighted_line(&list), "first");
    list.move_cursor(1);
    assert_eq!(highlighted_line(&list), "second");
}

#[test]
fn an_empty_list_draws_no_highlight() {
    assert_eq!(highlighted_line(&List::new(Vec::new())), "");
}

#[test]
fn the_status_line_names_the_showing_view() {
    let mut views = views_of(&["today"]);
    views.select(1);

    let frame = frame_of(&views, &list_of_two(), None);

    assert!(
        frame[0].contains("todoist  today  2 open tasks"),
        "{frame:?}"
    );
}

#[test]
fn the_picker_draws_every_view_by_number_over_the_list() {
    let views = views_of(&["today", "work"]);
    let prompt = Prompt::views(&views);

    let frame = frame_of(&views, &list_of_two(), Some(&prompt));

    assert!(frame[1].contains("views"), "{frame:?}");
    assert!(frame[2].contains("1 all"), "{frame:?}");
    assert!(frame[3].contains("2 today"), "{frame:?}");
    assert!(frame[4].contains("3 work"), "{frame:?}");
    assert!(
        frame.last().expect("a hint line").contains("<CR> pick"),
        "{frame:?}"
    );
}

/// 32 columns: roughly a third of a normal terminal, the width a side pane opens at, so a
/// line too wide for it shows up truncated here.
const NARROW: u16 = 32;

fn narrow_frame(prompt: Option<&Prompt>) -> Vec<String> {
    frame_of_width(
        &views_of(&["today"]),
        &list_of_two(),
        prompt,
        false,
        NARROW,
        "2 open tasks",
    )
}

#[test]
fn every_hint_line_fits_a_narrow_side_pane() {
    let views = views_of(&["today"]);
    let prompts = [
        Prompt::views(&views),
        Prompt::delete("1", "first"),
        Prompt::due("1"),
        Prompt::add(),
        Prompt::labels("1", &["home".to_string()], &[]),
        Prompt::move_to("1", &[], &[]),
        Prompt::note(),
    ];

    let expected = hints();
    let open = narrow_frame(None);
    let drawn = open.last().expect("a hint line");
    assert!(drawn.chars().count() <= NARROW as usize, "{drawn}");
    assert_eq!(drawn.trim(), expected, "the open list's hints wrapped");

    for prompt in &prompts {
        let frame = narrow_frame(Some(prompt));
        let hints = frame.last().expect("a hint line");
        assert_eq!(
            hints.trim(),
            prompt.hints(),
            "the {} prompt's hints wrapped",
            prompt.title()
        );
    }
}

#[test]
fn the_confirm_names_the_task_over_the_rows_it_would_leave_alone() {
    let frame = narrow_frame(Some(&Prompt::delete("1", "first")));

    assert!(frame[1].contains("delete"), "{frame:?}");
    assert!(frame[2].contains("delete first?"), "{frame:?}");
    assert!(
        frame.last().expect("a hint line").contains("dd delete"),
        "{frame:?}"
    );
}

#[test]
fn an_input_draws_the_line_typed_so_far_in_a_box_of_its_own() {
    let mut prompt = Prompt::due("1");
    let input: &mut Input = prompt.input_mut().expect("an input");
    for character in "next mon".chars() {
        input.push(character);
    }

    let frame = narrow_frame(Some(&prompt));

    assert!(frame[1].contains("due"), "{frame:?}");
    assert!(frame[2].contains("next mon_"), "{frame:?}");
}

#[test]
fn the_label_picker_draws_a_mark_against_the_labels_the_task_carries() {
    let frame = narrow_frame(Some(&Prompt::labels(
        "1",
        &["home".to_string()],
        &[serde_json::from_str(r#"{"name":"home"}"#).expect("label")],
    )));

    assert!(frame[1].contains("labels"), "{frame:?}");
    assert!(frame[2].contains("[x] home"), "{frame:?}");
}

#[test]
fn the_completed_list_is_named_in_the_status_line_and_hints_its_own_keys() {
    let mut views = views_of(&["today"]);
    views.select(1);

    // 32 columns: roughly a third of a normal terminal, the default `width` a side pane
    // opens at, so a hint line too wide for it shows up truncated here.
    let frame = frame_of_width(&views, &list_of_two(), None, true, 32, "2 open tasks");

    assert!(frame[0].contains("todoist  completed"), "{frame:?}");
    let hints = frame.last().expect("a hint line");
    assert_eq!(hints.trim(), COMPLETED_HINTS, "{hints}");
}

#[test]
fn the_open_list_hints_its_edits_the_editor_and_the_way_to_the_completed_one() {
    let frame = frame_of(&views_of(&[]), &list_of_two(), None);
    let hints = frame.last().expect("a hint line");

    assert!(hints.contains("x X dd p s l m a S e"), "{frame:?}");
    assert!(hints.contains("<CR>"), "{frame:?}");
    assert!(hints.contains("Tab"), "{frame:?}");
}

/// The worst line a real account produces: overdue, top priority, repeating, three labels, and a
/// title longer than the pane is wide.
const LONGEST: &str = r#"{"id":"1","content":"renew the vehicle registration","project_id":"p1",
     "priority":4,"labels":["home","errand","slow"],
     "due":{"date":"2026-09-11","is_recurring":true}}"#;

/// That task drawn at the width a side pane opens at, as the row the pane put it on.
fn longest_line(marks: &Marks) -> String {
    let project = serde_json::from_str(r#"{"id":"p1","name":"First"}"#).expect("project");
    let list = List::new(list::build(&[task(LONGEST)], &[project], &[], marks));
    let frame = frame_of_width(&views_of(&[]), &list, None, false, NARROW, "2 open tasks");
    frame[2].clone()
}

#[test]
fn the_longest_task_line_keeps_every_mark_inside_a_narrow_pane() {
    for (set, line) in [
        ("nerd font", longest_line(&marks())),
        ("plain", longest_line(&plain_marks())),
    ] {
        assert!(
            line.width() <= NARROW as usize,
            "{set}: {line:?} is wider than the pane"
        );
        // The marks lead, so what the width costs is the end of the title, said with an ellipsis.
        assert!(line.ends_with('\u{2026}'), "{set}: {line:?}");
        assert!(line.contains("renew the"), "{set}: {line:?}");
    }
}

#[test]
fn the_marks_of_the_longest_line_are_drawn_before_the_title() {
    let line = longest_line(&plain_marks());

    assert_eq!(line, "  ! <09-11 ~ @3 renew the vehic\u{2026}");
}

#[test]
fn a_title_short_enough_for_the_pane_is_drawn_whole() {
    let line = longest_line(&plain_marks());
    let short = {
        let project = serde_json::from_str(r#"{"id":"p1","name":"First"}"#).expect("project");
        let list = List::new(list::build(
            &[task(r#"{"id":"1","content":"pay rent","project_id":"p1"}"#)],
            &[project],
            &[],
            &plain_marks(),
        ));
        frame_of_width(&views_of(&[]), &list, None, false, NARROW, "2 open tasks")[2].clone()
    };

    assert_eq!(short, "  pay rent");
    assert!(line.len() > short.len());
}
