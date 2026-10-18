use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Screen};
use crate::legend::{self, Mode};
use crate::theme::{color, ground};
use herdr_damnit_domain::{Oid, Slot};

mod body;
mod detail;
mod overlay;
mod placement;
mod refusal;
mod rows;

use body::Body;
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

    let showing = app.showing();
    let subject_of = |oid: &Oid| app.subject_of(oid);
    let shown = match (app.refusal(), showing.detail(), showing.screen()) {
        (Some(said), ..) => Body::Refusal(said),
        (None, Some(object), _) => Body::Detail(object, &subject_of),
        (None, None, Screen::List) => Body::List(app.list(), app.status().stage()),
        (None, None, Screen::Status) => Body::Status(app.status()),
        (None, None, Screen::Done) => Body::Done(app.done()),
    };
    frame.render_widget(
        status_line(&app.header(Instant::now()), &shown.counts(), header.width),
        header,
    );
    shown.draw(frame, body);
    if let (None, Some(open)) = (app.refusal(), showing.overlay()) {
        overlay::draw(
            frame,
            &Placement::new(body, shown.cursor()),
            open,
            app.config().icons(),
        );
    }
    frame.render_widget(
        bottom_line(app.message(), shown.mode(showing.overlay()), bottom.width),
        bottom,
    );
}

fn status_line(header: &str, counts: &str, width: u16) -> Paragraph<'static> {
    let width = width as usize;
    let counts_kept_first = cut_to(counts, width);
    let room_left_for_the_header = width.saturating_sub(counts_kept_first.width());
    let header_cut_first = cut_to(header, room_left_for_the_header);
    let gap = room_left_for_the_header.saturating_sub(header_cut_first.width());
    Paragraph::new(Line::styled(
        format!("{header_cut_first}{}{counts_kept_first}", " ".repeat(gap)),
        Style::new().fg(color(Slot::Text)),
    ))
}

fn bottom_line(message: &str, mode: Mode, width: u16) -> Paragraph<'static> {
    let width = width as usize;
    let (said, colour) = match message.is_empty() {
        true => (legend::fitted(mode, width), color(Slot::Dim1)),
        false => (cut_to(message, width), color(Slot::Text)),
    };
    Paragraph::new(Line::styled(said, Style::new().fg(colour)))
}

#[cfg(test)]
pub use tests::render_to_text;

#[cfg(test)]
pub(crate) mod tests;
