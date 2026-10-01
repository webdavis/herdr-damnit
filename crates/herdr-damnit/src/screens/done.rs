use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{ListItem, ListState, Paragraph};

use crate::app::App;
use crate::screens::list::spans;
use crate::theme::Palette;
use herdr_damnit_domain::Row;

const EMPTY: &str = "nothing completed yet";

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App, palette: &Palette) {
    let rows = app.done.rows();
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::styled(EMPTY, Style::new().fg(palette.dim1))),
            area,
        );
        return;
    }
    let mut state = ListState::default().with_selected(Some(app.done.selected()));
    frame.render_stateful_widget(
        ratatui::widgets::List::new(rows.iter().map(|row| item(row, palette, area.width)))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED)),
        area,
        &mut state,
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
