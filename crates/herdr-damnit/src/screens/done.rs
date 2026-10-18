use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{ListItem, ListState, Paragraph};

use crate::app::App;
use crate::screens::list::spans;
use crate::theme::{bar, color};
use herdr_damnit_domain::{Row, Slot};

const EMPTY: &str = "nothing completed yet";

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let rows = app.done.rows();
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::styled(EMPTY, Style::new().fg(color(Slot::Dim1)))),
            area,
        );
        return;
    }
    let mut state = ListState::default().with_selected(Some(app.done.selected()));
    frame.render_stateful_widget(
        ratatui::widgets::List::new(rows.iter().map(|row| item(row, area.width)))
            .highlight_style(bar()),
        area,
        &mut state,
    );
}

fn item(row: &Row, width: u16) -> ListItem<'static> {
    match row {
        Row::Heading(text) => ListItem::new(Line::styled(
            text.clone(),
            Style::new()
                .fg(color(Slot::Blue))
                .add_modifier(Modifier::BOLD),
        )),
        Row::Object(object) => ListItem::new(Line::from(spans(&object.segments, width as usize))),
    }
}
