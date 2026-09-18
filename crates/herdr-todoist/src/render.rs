//! Drawing the pane: the status line, the task list, the view picker over it, and the key hints.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, ListItem, ListState, Paragraph};

use crate::cursor::List;
use crate::list::Row;
use crate::views::{MAX_NUMBERED_VIEW, Picker, Views};

const PICKER_HINTS: &str = "j/k move   <CR> show   <Esc> cancel";

fn hints() -> String {
    format!("j/k move   v views   1-{MAX_NUMBERED_VIEW} view   R refresh   q close")
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

/// The pane's whole frame: the status line, the list, the picker when it is open, and the hints.
pub fn draw(
    frame: &mut ratatui::Frame<'_>,
    status: &str,
    views: &Views,
    list: &List,
    row: &mut ListState,
    picker: Option<&Picker>,
) {
    let [status_area, body_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    frame.render_widget(
        Paragraph::new(status_line(status, &views.current().name)),
        status_area,
    );
    row.select(list.selected_id().map(|_| list.selected()));
    frame.render_stateful_widget(
        ratatui::widgets::List::new(list.rows().iter().map(item))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED)),
        body_area,
        row,
    );
    if let Some(picker) = picker {
        draw_picker(frame, body_area, views, picker);
    }
    let hints = if picker.is_some() {
        PICKER_HINTS.to_string()
    } else {
        hints()
    };
    frame.render_widget(Paragraph::new(Line::from(hints)), hint_area);
}

/// The view picker, over the list: every view by name and number, the cursor on one of them.
fn draw_picker(frame: &mut ratatui::Frame<'_>, body_area: Rect, views: &Views, picker: &Picker) {
    let height = (views.len() as u16 + 2).min(body_area.height);
    let area = Rect {
        height,
        ..body_area
    };
    let entries: Vec<ListItem<'_>> = views
        .names()
        .enumerate()
        .map(|(index, name)| ListItem::new(Line::from(format!(" {} {name}", index + 1))))
        .collect();
    let mut state = ListState::default();
    state.select(Some(picker.at()));
    frame.render_widget(Clear, area);
    frame.render_stateful_widget(
        ratatui::widgets::List::new(entries)
            .block(Block::default().borders(Borders::ALL).title("views"))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED)),
        area,
        &mut state,
    );
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
    fn frame_of(views: &Views, list: &List, picker: Option<&Picker>) -> Vec<String> {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 8)).expect("terminal");
        let mut state = ListState::default();
        terminal
            .draw(|frame| draw(frame, "2 open tasks", views, list, &mut state, picker))
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
        let picker = Picker::open(&views);

        let frame = frame_of(&views, &list_of_two(), Some(&picker));

        assert!(frame[1].contains("views"), "{frame:?}");
        assert!(frame[2].contains("1 all"), "{frame:?}");
        assert!(frame[3].contains("2 today"), "{frame:?}");
        assert!(frame[4].contains("3 work"), "{frame:?}");
        assert!(
            frame.last().expect("a hint line").contains("<CR> show"),
            "{frame:?}"
        );
    }
}
