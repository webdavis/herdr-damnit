//! Drawing the pane: the status line, the task list, the view picker over it, and the key hints.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, ListItem, ListState, Paragraph, Wrap};

use crate::detail::Detail;

use crate::cursor::List;
use crate::list::Row;
use crate::prompt::Prompt;
use crate::views::{MAX_NUMBERED_VIEW, Views};

const COMPLETED_HINTS: &str = "j/k  u reopen  <Tab> open  R  q";

/// The detail screen's own keys.
const DETAIL_HINTS: &str = "j/k  c comment  <Esc> back  R  q";

/// What the status line calls the detail screen.
const DETAIL_LABEL: &str = "task";

/// What the status line calls the completed list, which has no filter query of its own.
const COMPLETED_LABEL: &str = "completed";

/// The open list's own keys. A side pane is about 32 columns wide and this line fills it exactly:
/// the eight edits, `S` to hand the task to an agent, `<CR>` into the detail, the view picker and
/// its numbers, and Tab to the completed list, which is written bare so the line still fits. j/k,
/// R, `q` and the arrow keys are left unsaid, `q` because `<Esc>` closes the pane too and both
/// other screens name it.
fn hints() -> String {
    format!("x X dd p s l m a S <CR> v1-{MAX_NUMBERED_VIEW} Tab")
}

/// The status line, naming the showing view. Every failure the client can report (a rejected
/// token, a rate limit with its retry delay, a network outage) arrives in `status` as its own
/// message, so a message about a refused filter says which view was refused.
fn status_line(status: &str, view: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled("todoist", Style::new().add_modifier(Modifier::BOLD)),
        Span::raw(format!("  {view}  {status}")),
    ])
}

/// The pane's whole frame: the status line, the list, the prompt when one is open, and the hints.
/// `completed` says which of the pane's two lists is on screen, which is what the status line
/// names and what the hints are for.
pub fn draw(
    frame: &mut ratatui::Frame<'_>,
    status: &str,
    views: &Views,
    list: &List,
    row: &mut ListState,
    prompt: Option<&Prompt>,
    completed: bool,
) {
    let [status_area, body_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    let label = if completed {
        COMPLETED_LABEL
    } else {
        &views.current().name
    };
    frame.render_widget(Paragraph::new(status_line(status, label)), status_area);
    row.select(list.selected_id().map(|_| list.selected()));
    frame.render_stateful_widget(
        ratatui::widgets::List::new(list.rows().iter().map(item))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED)),
        body_area,
        row,
    );
    if let Some(prompt) = prompt {
        draw_prompt(frame, body_area, views, prompt);
    }
    let hints = match (prompt, completed) {
        (Some(prompt), _) => prompt.hints().to_string(),
        (None, true) => COMPLETED_HINTS.to_string(),
        (None, false) => hints(),
    };
    frame.render_widget(Paragraph::new(Line::from(hints)), hint_area);
}

/// The detail screen: one task's description and its comment thread, as one wrapped paragraph so
/// a long link, a table row or a line of code breaks inside the pane rather than running off it.
/// The comment box is the same bordered overlay every prompt draws in.
pub fn draw_detail(frame: &mut ratatui::Frame<'_>, views: &Views, detail: &Detail) {
    let [status_area, body_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    frame.render_widget(
        Paragraph::new(status_line(detail.status(), DETAIL_LABEL)),
        status_area,
    );
    frame.render_widget(
        Paragraph::new(detail.lines())
            .wrap(Wrap { trim: false })
            .scroll((detail.scroll(), 0)),
        body_area,
    );
    let hints = match detail.prompt() {
        Some(prompt) => {
            draw_prompt(frame, body_area, views, prompt);
            prompt.hints().to_string()
        }
        None => DETAIL_HINTS.to_string(),
    };
    frame.render_widget(Paragraph::new(Line::from(hints)), hint_area);
}

/// The prompt, over the list: a bordered box holding either its entries with the cursor on one of
/// them, or the one line it is asking for. One drawing serves every prompt, so the views picker,
/// the label picker, the move picker, the confirm and the two inputs all sit in the same place.
fn draw_prompt(frame: &mut ratatui::Frame<'_>, body_area: Rect, views: &Views, prompt: &Prompt) {
    let block = Block::default().borders(Borders::ALL).title(prompt.title());
    match prompt.entries(views) {
        Some((entries, at)) => {
            let area = boxed(body_area, entries.len() as u16);
            let mut state = ListState::default();
            state.select(Some(at));
            let highlight = if prompt.highlights() {
                Style::new().add_modifier(Modifier::REVERSED)
            } else {
                Style::new()
            };
            frame.render_widget(Clear, area);
            frame.render_stateful_widget(
                ratatui::widgets::List::new(entries.into_iter().map(ListItem::new))
                    .block(block)
                    .highlight_style(highlight),
                area,
                &mut state,
            );
        }
        None => {
            let area = boxed(body_area, 1);
            frame.render_widget(Clear, area);
            frame.render_widget(
                Paragraph::new(Line::from(prompt.line().unwrap_or_default())).block(block),
                area,
            );
        }
    }
}

/// A box for `rows` rows at the top of the body, never taller than the body itself, so a long
/// picker in a short pane is cut off rather than drawn outside the pane.
fn boxed(body_area: Rect, rows: u16) -> Rect {
    Rect {
        height: rows.saturating_add(2).min(body_area.height),
        ..body_area
    }
}

/// A heading stands out in bold; a task line is drawn as it was built.
fn item(row: &Row) -> ListItem<'_> {
    let style = match row {
        Row::Header(_) => Style::new().add_modifier(Modifier::BOLD),
        Row::Task(_) => Style::new(),
    };
    ListItem::new(Line::styled(row.text(), style))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::list;
    use crate::list::tests::task;
    use crate::prompt::Input;

    fn list_of_two() -> List {
        let project = serde_json::from_str(r#"{"id":"p1","name":"First"}"#).expect("project");
        List::new(list::build(
            &[
                task(r#"{"id":"1","content":"first","project_id":"p1","child_order":1}"#),
                task(r#"{"id":"2","content":"second","project_id":"p1","child_order":2}"#),
            ],
            &[project],
            &[],
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
        frame_of_width(views, list, prompt, completed, 80)
    }

    /// Draw into an off-screen terminal the width of a real side pane, roughly a third of a
    /// terminal, so a hint or status line too wide to fit shows up truncated.
    fn frame_of_width(
        views: &Views,
        list: &List,
        prompt: Option<&Prompt>,
        completed: bool,
        width: u16,
    ) -> Vec<String> {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 8)).expect("terminal");
        let mut state = ListState::default();
        terminal
            .draw(|frame| {
                draw(
                    frame,
                    "2 open tasks",
                    views,
                    list,
                    &mut state,
                    prompt,
                    completed,
                )
            })
            .expect("draw");
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

    /// Draw into an off-screen terminal and report which line came out highlighted.
    fn highlighted_line(list: &List) -> String {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 6)).expect("terminal");
        let mut state = ListState::default();
        terminal
            .draw(|frame| {
                draw(
                    frame,
                    "2 open tasks",
                    &views_of(&[]),
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
        frame_of_width(&views_of(&["today"]), &list_of_two(), prompt, false, NARROW)
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
        let frame = frame_of_width(&views, &list_of_two(), None, true, 32);

        assert!(frame[0].contains("todoist  completed"), "{frame:?}");
        let hints = frame.last().expect("a hint line");
        assert_eq!(hints.trim(), COMPLETED_HINTS, "{hints}");
    }

    #[test]
    fn the_open_list_hints_its_edits_and_the_way_to_the_completed_one() {
        let frame = frame_of(&views_of(&[]), &list_of_two(), None);
        let hints = frame.last().expect("a hint line");

        assert!(hints.contains("x X dd p s l m a S"), "{frame:?}");
        assert!(hints.contains("<CR>"), "{frame:?}");
        assert!(hints.contains("Tab"), "{frame:?}");
    }
}
