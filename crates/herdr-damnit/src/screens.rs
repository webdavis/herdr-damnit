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
use herdr_damnit_domain::{Slot, counted};

mod detail;
mod overlay;
mod placement;
mod refusal;
mod rows;

use placement::Placement;
use rows::cut_to;

pub const ELLIPSIS: &str = "\u{2026}";

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    frame.buffer_mut().set_style(area, ground());
    let [header, body, bottom] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);

    frame.render_widget(status_line(app, header.width), header);
    match (
        app.refusal(),
        app.showing().detail(),
        app.showing().screen(),
    ) {
        (Some(said), ..) => refusal::draw(frame, body, said),
        (None, Some(object), _) => detail::draw(frame, body, object, app),
        (None, None, Screen::List) => {
            rows::draw(frame, body, app.list().cursor(), "nothing in this view")
        }
        (None, None, Screen::Status) => rows::draw(frame, body, &app.status, ""),
        (None, None, Screen::Done) => rows::draw(frame, body, &app.done, "nothing completed yet"),
    }
    if let (None, Some(open)) = (app.refusal(), app.showing().overlay()) {
        overlay::draw(
            frame,
            &Placement::new(body, cursor(app)),
            open,
            app.config.icons(),
        );
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
    let showing = app.showing();
    match (
        app.refusal().is_some() || showing.detail().is_some(),
        showing.screen(),
    ) {
        (true, _) => String::new(),
        (false, Screen::List) => open_counts(app),
        (false, Screen::Status) => app.stage.summary(),
        (false, Screen::Done) => format!("{} done", app.done.object_count()),
    }
}

fn open_counts(app: &App) -> String {
    counted(&[
        (app.list().cursor().object_count() as u64, "open"),
        (app.stage.staged_count() as u64, "staged"),
        (app.stage.unpushed_commits(), "unpushed"),
    ])
}

fn bottom_line(app: &App, width: u16) -> Paragraph<'static> {
    let width = width as usize;
    let (said, colour) = match app.message.is_empty() {
        true => (legend::fitted(mode(app), width), color(Slot::Dim1)),
        false => (cut_to(&app.message, width), color(Slot::Text)),
    };
    Paragraph::new(Line::styled(said, Style::new().fg(colour)))
}

fn cursor(app: &App) -> Option<usize> {
    match (app.showing().detail(), app.showing().screen()) {
        (Some(_), _) => None,
        (None, Screen::List) => Some(app.list().cursor().selected()),
        (None, Screen::Status) => app.status.selected_oid().map(|_| app.status.selected()),
        (None, Screen::Done) => Some(app.done.selected()),
    }
}

fn mode(app: &App) -> Mode {
    if app.refusal().is_some() {
        return Mode::Refusal;
    }
    let showing = app.showing();
    match (showing.overlay(), showing.detail(), showing.screen()) {
        (Some(Overlay::View(_) | Overlay::Label(..) | Overlay::Path(..)), ..) => Mode::Picker,
        (Some(Overlay::Line(_)), ..) => Mode::LineBox,
        (Some(Overlay::Note(_)), ..) => Mode::NoteBox,
        (Some(Overlay::Confirm(confirm)), ..) => Mode::Confirm(confirm.key),
        (None, Some(_), _) => Mode::Detail,
        (None, None, Screen::List) => Mode::List,
        (None, None, Screen::Status) => Mode::Status,
        (None, None, Screen::Done) => Mode::Done,
    }
}

#[cfg(test)]
pub(crate) mod tests;
