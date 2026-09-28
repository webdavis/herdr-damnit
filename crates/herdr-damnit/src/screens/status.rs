use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::ListItem;

use crate::app::App;
use crate::screens::list::spans;
use crate::theme::Palette;
use herdr_damnit_domain::{Mark, Segment, Slot, StatusRow};

const MARKED_ROW_INDENT: &str = "  ";

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

fn marked(mark: Option<Mark>, text: &str, icons: herdr_damnit_domain::IconSet) -> Vec<Segment> {
    let Some(mark) = mark else {
        return vec![Segment {
            text: text.to_string(),
            slot: Slot::Text,
        }];
    };
    vec![
        Segment {
            text: MARKED_ROW_INDENT.to_string(),
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
