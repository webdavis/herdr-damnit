//! Drawing the pane. One module per screen; this file is the frame every screen shares and the
//! text renderer the golden tests compare.

use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Screen};
use crate::theme::Palette;

mod done;
mod list;
mod refusal;
mod status;

pub use list::cut_to;

/// What a line cut off by the pane's width ends in, so a subject the pane cut says it was cut.
pub const ELLIPSIS: &str = "\u{2026}";

/// The keys the hint line offers, cut to the pane's width.
const HINTS: &str = "x X dd p s D l m a S e <CR> <Space> c P L v Tab";

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let palette = crate::theme::resolve(app.config.theme.as_deref());
    if let Some(said) = &app.refusal {
        refusal::draw(frame, frame.area(), said, &palette);
        return;
    }
    let [header, body, hints] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    frame.render_widget(status_line(app, header.width, &palette), header);
    match app.screen {
        Screen::List => list::draw(frame, body, app, &palette),
        Screen::Status => status::draw(frame, body, app, &palette),
        Screen::Done => done::draw(frame, body, app, &palette),
    }
    frame.render_widget(hint_line(hints.width, &palette), hints);
}

/// The whole pane as plain text, which is what a golden compares. Trailing blanks are trimmed per
/// line so a golden is a picture of the screen rather than a block of padding.
#[cfg(test)]
pub fn render_to_text(app: &App, width: u16, height: u16) -> String {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height))
        .expect("a test terminal");
    terminal.draw(|frame| draw(frame, app)).expect("it drew");
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|row| {
            (0..width)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

/// The status line: what the pane is doing on the left, what it is holding on the right. The left
/// half is cut first, because a spinner and a timer matter less than the counts they run beside.
fn status_line(app: &App, width: u16, palette: &Palette) -> Paragraph<'static> {
    let width = width as usize;
    let right = cut_to(&counts(app), width);
    let room = width.saturating_sub(right.width());
    let left = cut_to(&app.header(Instant::now()), room);
    let gap = room.saturating_sub(left.width());
    Paragraph::new(Line::styled(
        format!("{left}{}{right}", " ".repeat(gap)),
        Style::new().fg(palette.text),
    ))
}

/// What the screen on show is holding. The List screen counts its own rows and marks what is
/// waiting behind them; the Status screen reports the stage in `dam`'s own words; the Done screen
/// counts what it lists.
fn counts(app: &App) -> String {
    match app.screen {
        Screen::List => open_counts(app),
        Screen::Status => app.stage.summary(),
        Screen::Done => format!(
            "{} done",
            app.done_rows()
                .iter()
                .filter(|row| row.oid().is_some())
                .count()
        ),
    }
}

/// The List screen's counts, in the order the spec's status line lists them. A count of nothing is
/// left out, so a clean store reads as the object count alone.
fn open_counts(app: &App) -> String {
    let mut counts = vec![format!("{} open", app.list.object_count())];
    if app.stage.staged_count() > 0 {
        counts.push(format!("+{} staged", app.stage.staged_count()));
    }
    if app.stage.unpushed_commits() > 0 {
        counts.push(format!("^{} unpushed", app.stage.unpushed_commits()));
    }
    counts.join("  ")
}

fn hint_line(width: u16, palette: &Palette) -> Paragraph<'static> {
    Paragraph::new(Line::styled(
        cut_to(HINTS, width as usize),
        Style::new().fg(palette.dim1),
    ))
}

#[cfg(test)]
mod tests;
