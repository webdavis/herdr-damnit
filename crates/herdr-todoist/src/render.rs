//! Drawing the pane: the status line, the task list, the view picker over it, and the key hints.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, ListItem, ListState, Paragraph, Wrap};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::detail::Detail;

use crate::cursor::List;
use crate::list::{Row, TaskRow};
use crate::prompt::Prompt;
use crate::theme::Palette;
use crate::views::Views;

/// What a truncated line ends in, so a title cut off by the pane's width says it was cut.
const ELLIPSIS: &str = "\u{2026}";

const COMPLETED_HINTS: &str = "j/k  u reopen  <Tab> open  R  q";

/// The detail screen's own keys.
const DETAIL_HINTS: &str = "j/k  c comment  <Esc> back  R  q";

/// What the status line calls the detail screen.
const DETAIL_LABEL: &str = "task";

/// What every screen draws with: the views the pane knows, which name the showing one, and the
/// palette every line takes its colors from.
pub struct Chrome<'a> {
    pub views: &'a Views,
    pub palette: &'a Palette,
}

/// What the status line calls the completed list, which has no filter query of its own.
const COMPLETED_LABEL: &str = "completed";

/// The open list's own keys, inside the 32 columns a side pane opens at: the eight edits, `S` to
/// hand the task to an agent, `e` into the editor, `<CR>` into the detail, the view picker and
/// `Tab` to the completed list, written bare so the line still fits. The view numbers are left to
/// the picker, which lists them; j/k, R, `q` and the arrow keys are left unsaid, `q` because
/// `<Esc>` closes the pane too and both other screens name it.
fn hints() -> &'static str {
    "x X dd p s l m a S e <CR> v Tab"
}

/// The status line, naming the showing view. Every failure the client can report (a rejected
/// token, a rate limit with its retry delay, a network outage) arrives in `status` as its own
/// message, so a message about a refused filter says which view was refused.
fn status_line(status: &str, view: &str, palette: &Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            "todoist",
            Style::new().fg(palette.blue).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("  {view}  {status}"), Style::new().fg(palette.text)),
    ])
}

/// The pane's whole frame: the status line, the list, the prompt when one is open, and the hints.
/// `completed` says which of the pane's two lists is on screen, which is what the status line
/// names and what the hints are for.
pub fn draw(
    frame: &mut ratatui::Frame<'_>,
    status: &str,
    chrome: &Chrome<'_>,
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
    let palette = chrome.palette;
    let label = if completed {
        COMPLETED_LABEL
    } else {
        &chrome.views.current().name
    };
    frame.render_widget(
        Paragraph::new(status_line(status, label, palette)),
        status_area,
    );
    row.select(list.selected_id().map(|_| list.selected()));
    frame.render_stateful_widget(
        ratatui::widgets::List::new(
            list.rows()
                .iter()
                .map(|row| item(row, palette, body_area.width)),
        )
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED)),
        body_area,
        row,
    );
    if let Some(prompt) = prompt {
        draw_prompt(frame, body_area, chrome.views, prompt);
    }
    let hints = match (prompt, completed) {
        (Some(prompt), _) => prompt.hints().to_string(),
        (None, true) => COMPLETED_HINTS.to_string(),
        (None, false) => hints().to_string(),
    };
    frame.render_widget(
        Paragraph::new(Line::styled(hints, Style::new().fg(palette.dim1))),
        hint_area,
    );
}

/// The detail screen: one task's description and its comment thread, as one wrapped paragraph so
/// a long link, a table row or a line of code breaks inside the pane rather than running off it.
/// The comment box is the same bordered overlay every prompt draws in.
pub fn draw_detail(frame: &mut ratatui::Frame<'_>, chrome: &Chrome<'_>, detail: &Detail) {
    let palette = chrome.palette;
    let [status_area, body_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    frame.render_widget(
        Paragraph::new(status_line(detail.status(), DETAIL_LABEL, palette)),
        status_area,
    );
    frame.render_widget(
        Paragraph::new(detail.lines())
            .style(Style::new().fg(palette.text))
            .wrap(Wrap { trim: false })
            .scroll((detail.scroll(), 0)),
        body_area,
    );
    let hints = match detail.prompt() {
        Some(prompt) => {
            draw_prompt(frame, body_area, chrome.views, prompt);
            prompt.hints().to_string()
        }
        None => DETAIL_HINTS.to_string(),
    };
    frame.render_widget(
        Paragraph::new(Line::styled(hints, Style::new().fg(palette.dim1))),
        hint_area,
    );
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

/// A heading stands out in bold; a task line is drawn in the colors its marks were built with,
/// cut to `width` with an ellipsis when it is longer, so a long title is visibly truncated rather
/// than silently running past the pane.
fn item(row: &Row, palette: &Palette, width: u16) -> ListItem<'static> {
    match row {
        Row::Header(_) => ListItem::new(Line::styled(
            row.text(),
            Style::new().fg(palette.blue).add_modifier(Modifier::BOLD),
        )),
        Row::Task(task) => ListItem::new(Line::from(spans(task, palette, width as usize))),
    }
}

/// A task line as colored runs, stopping at `width` columns. Width is counted in terminal cells,
/// not characters, so a title holding a double-width character is cut where the terminal would
/// wrap it.
fn spans(task: &TaskRow, palette: &Palette, width: usize) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut left = width;
    for part in &task.text {
        if left == 0 {
            break;
        }
        let style = Style::new().fg(palette.color(part.slot));
        if part.text.width() <= left {
            left -= part.text.width();
            spans.push(Span::styled(part.text.clone(), style));
            continue;
        }
        let kept = cut(&part.text, left.saturating_sub(1));
        spans.push(Span::styled(format!("{kept}{ELLIPSIS}"), style));
        left = 0;
    }
    spans
}

/// The longest prefix of `text` that fits `width` terminal cells.
fn cut(text: &str, width: usize) -> String {
    let mut kept = String::new();
    let mut spent = 0;
    for character in text.chars() {
        let cell = character.width().unwrap_or(0);
        if spent + cell > width {
            break;
        }
        spent += cell;
        kept.push(character);
    }
    kept
}

#[cfg(test)]
pub(crate) mod tests;
