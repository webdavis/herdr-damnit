//! The List screen: the objects of the showing view, grouped by path, each row's marks drawn in
//! the palette colour its slot resolves to.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{ListItem, ListState};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::App;
use crate::screens::ELLIPSIS;
use crate::theme::Palette;
use herdr_damnit_domain::{Row, Segment};

/// What an empty view says, so a view whose query matches nothing is distinguishable from a read
/// that has not answered yet.
const EMPTY: &str = "nothing in this view";

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App, palette: &Palette) {
    if app.list.rows().is_empty() {
        frame.render_widget(
            ratatui::widgets::Paragraph::new(Line::styled(EMPTY, Style::new().fg(palette.dim1))),
            area,
        );
        return;
    }
    let mut state = ListState::default().with_selected(Some(app.list.selected()));
    frame.render_stateful_widget(
        ratatui::widgets::List::new(
            app.list
                .rows()
                .iter()
                .map(|row| item(row, palette, area.width)),
        )
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

/// One row as coloured runs, stopping at `width` terminal cells. Width is counted in cells rather
/// than characters, so a subject holding a double-width character is cut where the terminal would
/// wrap it.
fn spans(segments: &[Segment], palette: &Palette, width: usize) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut left = width;
    for segment in segments {
        if left == 0 {
            break;
        }
        let style = Style::new().fg(palette.color(segment.slot));
        if segment.text.width() <= left {
            left -= segment.text.width();
            spans.push(Span::styled(segment.text.clone(), style));
            continue;
        }
        spans.push(Span::styled(
            format!("{}{ELLIPSIS}", cut(&segment.text, left.saturating_sub(1))),
            style,
        ));
        left = 0;
    }
    spans
}

/// `text` cut to `width` terminal cells, ending in an ellipsis when anything was dropped.
pub fn cut_to(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    format!("{}{ELLIPSIS}", cut(text, width.saturating_sub(1)))
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
