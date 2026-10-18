use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Overlay, Screen};
use crate::legend::{self, Mode};
use crate::theme::{color, ground};
use herdr_damnit_domain::Slot;

mod detail;
mod done;
mod list;
mod overlay;
mod refusal;
mod status;

use list::cut_to;

pub const ELLIPSIS: &str = "\u{2026}";

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    frame.buffer_mut().set_style(area, ground());
    if let Some(said) = &app.refusal {
        refusal::draw(frame, frame.area(), said);
        return;
    }
    let [header, body, bottom] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    frame.render_widget(status_line(app, header.width), header);
    match app.screen {
        Screen::List => list::draw(frame, body, app),
        Screen::Status => status::draw(frame, body, app),
        Screen::Done => done::draw(frame, body, app),
        Screen::Detail => detail::draw(frame, body, app),
    }
    if let Some(open) = &app.overlay {
        overlay::draw(frame, body, open, app.config.icons());
    }
    frame.render_widget(bottom_line(app, bottom.width), bottom);
}

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

fn status_line(app: &App, width: u16) -> Paragraph<'static> {
    let width = width as usize;
    let counts_kept_first = cut_to(&counts(app), width);
    let room_left_for_the_header = width.saturating_sub(counts_kept_first.width());
    let header_cut_first = cut_to(&app.header(Instant::now()), room_left_for_the_header);
    let gap = room_left_for_the_header.saturating_sub(header_cut_first.width());
    Paragraph::new(Line::styled(
        format!("{header_cut_first}{}{counts_kept_first}", " ".repeat(gap)),
        Style::new().fg(color(Slot::Text)),
    ))
}

fn counts(app: &App) -> String {
    match app.screen {
        Screen::List => open_counts(app),
        Screen::Status => app.stage.summary(),
        Screen::Done => format!("{} done", app.done.object_count()),
        Screen::Detail => String::new(),
    }
}

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

fn bottom_line(app: &App, width: u16) -> Paragraph<'static> {
    let width = width as usize;
    let (said, colour) = match app.message.is_empty() {
        true => (legend::fitted(mode(app), width), color(Slot::Dim1)),
        false => (cut_to(&app.message, width), color(Slot::Text)),
    };
    Paragraph::new(Line::styled(said, Style::new().fg(colour)))
}

fn mode(app: &App) -> Mode {
    match (&app.overlay, app.screen) {
        (Some(Overlay::View(_) | Overlay::Label(..) | Overlay::Path(..)), _) => Mode::Picker,
        (Some(Overlay::Line(_)), _) => Mode::LineBox,
        (Some(Overlay::Note(_)), _) => Mode::NoteBox,
        (Some(Overlay::Confirm(confirm)), _) => Mode::Confirm(confirm.key),
        (None, Screen::List) => Mode::List,
        (None, Screen::Status) => Mode::Status,
        (None, Screen::Done) => Mode::Done,
        (None, Screen::Detail) => Mode::Detail,
    }
}

#[cfg(test)]
pub(crate) mod tests;
