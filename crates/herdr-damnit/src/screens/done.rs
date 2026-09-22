//! The Done screen: completed objects, newest completion first, the date leading each row so the
//! dates line up down the pane.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{ListItem, Paragraph};

use crate::app::App;
use crate::screens::list::spans;
use crate::theme::Palette;
use herdr_damnit_domain::Row;

/// What an empty Done screen says, so a store with nothing completed in it is distinguishable
/// from a read that has not answered yet.
const EMPTY: &str = "nothing completed yet";

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App, palette: &Palette) {
    let rows = app.done_rows();
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::styled(EMPTY, Style::new().fg(palette.dim1))),
            area,
        );
        return;
    }
    frame.render_widget(
        ratatui::widgets::List::new(rows.iter().map(|row| item(row, palette, area.width))),
        area,
    );
}

fn item(row: &Row, palette: &Palette, width: u16) -> ListItem<'static> {
    match row {
        Row::Heading(text) => ListItem::new(Line::styled(
            text.clone(),
            Style::new().fg(palette.blue).add_modifier(Modifier::BOLD),
        )),
        Row::Object(object) => {
            ListItem::new(Line::from(spans(&object.segments, palette, width as usize)))
        }
    }
}
