use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{ListItem, ListState};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::screens::ELLIPSIS;
use crate::theme::{bar, color};
use herdr_damnit_domain::{Cursor, Row, Segment, Slot};

pub fn draw(frame: &mut Frame<'_>, area: Rect, cursor: &Cursor, empty: &str) {
    if cursor.rows().is_empty() {
        frame.render_widget(
            ratatui::widgets::Paragraph::new(Line::styled(
                empty,
                Style::new().fg(color(Slot::Dim1)),
            )),
            area,
        );
        return;
    }
    let mut state =
        ListState::default().with_selected(cursor.selected_oid().map(|_| cursor.selected()));
    frame.render_stateful_widget(
        ratatui::widgets::List::new(cursor.rows().iter().map(|row| item(row, area.width)))
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
        Row::Line { segments, .. } => ListItem::new(Line::from(spans(segments, width as usize))),
    }
}

fn spans(segments: &[Segment], width: usize) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut left = width;
    for segment in segments {
        if left == 0 {
            break;
        }
        let style = Style::new().fg(color(segment.slot));
        if segment.text.width() <= left {
            left -= segment.text.width();
            spans.push(Span::styled(segment.text.clone(), style));
            continue;
        }
        spans.push(Span::styled(
            format!(
                "{}{ELLIPSIS}",
                longest_prefix_fitting(&segment.text, left.saturating_sub(1))
            ),
            style,
        ));
        left = 0;
    }
    spans
}

pub(super) fn cut_to(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    format!(
        "{}{ELLIPSIS}",
        longest_prefix_fitting(text, width.saturating_sub(1))
    )
}

fn longest_prefix_fitting(text: &str, width: usize) -> String {
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
