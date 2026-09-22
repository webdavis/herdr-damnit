//! The Status screen: the four sections `dam status --json` reports, in the order `dam`'s own
//! human output uses. Every row's mark is the one its change carries, never one inferred from the
//! section it sits under.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::ListItem;

use crate::app::App;
use crate::screens::list::spans;
use crate::theme::Palette;
use herdr_damnit_domain::{Mark, Segment, Slot, StatusRow};

/// The indent every marked row takes, with the mark drawn inside it.
const INDENT: &str = "  ";

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App, palette: &Palette) {
    frame.render_widget(
        ratatui::widgets::List::new(
            app.stage
                .rows()
                .iter()
                .map(|row| item(row, app.config.icons(), palette, area.width)),
        ),
        area,
    );
}

fn item(
    row: &StatusRow,
    icons: herdr_damnit_domain::IconSet,
    palette: &Palette,
    width: u16,
) -> ListItem<'static> {
    let segments = match row {
        StatusRow::Heading(text) => {
            return ListItem::new(Line::styled(
                text.clone(),
                Style::new().fg(palette.blue).add_modifier(Modifier::BOLD),
            ));
        }
        StatusRow::Change { mark, text, .. } => marked(Some(*mark), text, icons),
        StatusRow::Line { mark, text } => marked(*mark, text, icons),
    };
    ListItem::new(Line::from(spans(&segments, palette, width as usize)))
}

/// One row as its mark and its text. A mark sits inside the row's own indent, which is where the
/// spec's Status screen draws it. A row carrying no mark draws its text as the staging model wrote
/// it, so the mark column is never filled in from the heading above it.
fn marked(mark: Option<Mark>, text: &str, icons: herdr_damnit_domain::IconSet) -> Vec<Segment> {
    let Some(mark) = mark else {
        return vec![Segment {
            text: text.to_string(),
            slot: Slot::Text,
        }];
    };
    vec![
        Segment {
            text: INDENT.to_string(),
            slot: Slot::Text,
        },
        Segment {
            text: format!("{} ", mark.glyph(icons)),
            slot: mark.slot(),
        },
        Segment {
            text: text.trim_start().to_string(),
            slot: Slot::Text,
        },
    ]
}
