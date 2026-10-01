use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Screen};
use crate::theme::Palette;

mod detail;
mod done;
mod list;
mod refusal;
mod status;

use list::cut_to;

pub const ELLIPSIS: &str = "\u{2026}";

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
        Screen::Detail => detail::draw(frame, body, app, &palette),
    }
    frame.render_widget(hint_line(hints.width, &palette), hints);
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

fn status_line(app: &App, width: u16, palette: &Palette) -> Paragraph<'static> {
    let width = width as usize;
    let counts_kept_first = cut_to(&counts(app), width);
    let room_left_for_the_header = width.saturating_sub(counts_kept_first.width());
    let header_cut_first = cut_to(&app.header(Instant::now()), room_left_for_the_header);
    let gap = room_left_for_the_header.saturating_sub(header_cut_first.width());
    Paragraph::new(Line::styled(
        format!("{header_cut_first}{}{counts_kept_first}", " ".repeat(gap)),
        Style::new().fg(palette.text),
    ))
}

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

fn hint_line(width: u16, palette: &Palette) -> Paragraph<'static> {
    Paragraph::new(Line::styled(
        cut_to(HINTS, width as usize),
        Style::new().fg(palette.dim1),
    ))
}

#[cfg(test)]
mod tests;
