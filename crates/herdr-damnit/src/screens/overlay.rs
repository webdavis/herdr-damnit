use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap};

use herdr_damnit_domain::{IconSet, Mark, Slot};

use super::placement::Placement;
use crate::overlay::{LineBox, Overlay, Picker};
use crate::theme::{bar, color, ground};

const LINE_BOX_HEIGHT: u16 = 3;

const BORDERS: u16 = 2;

pub fn draw(frame: &mut Frame<'_>, placement: &Placement, overlay: &Overlay, icons: IconSet) {
    match overlay {
        Overlay::View(picker) | Overlay::Label(_, picker) | Overlay::Path(_, picker) => {
            picked(frame, placement, picker, icons)
        }
        Overlay::Line(line) => typed(frame, placement.place(LINE_BOX_HEIGHT), line),
        Overlay::Confirm(confirm) => asked(frame, placement, &confirm.question),
        Overlay::Note(note) => noted(frame, placement, &note.text),
    }
}

fn shown(frame: &mut Frame<'_>, placement: &Placement, paragraph: Paragraph<'_>) {
    let height = paragraph.line_count(placement.width().saturating_sub(BORDERS)) as u16;
    let area = placement.place(height);
    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, area);
}

fn bordered(title: &str) -> Block<'static> {
    Block::bordered()
        .style(ground())
        .title(title.to_string())
        .border_style(Style::new().fg(color(Slot::Dim1)))
}

fn picked(frame: &mut Frame<'_>, placement: &Placement, picker: &Picker, icons: IconSet) {
    let area = placement.place((picker.entries.len() as u16).saturating_add(BORDERS));
    let mark = format!("{} ", Mark::Staged.glyph(icons));
    let blank = " ".repeat(mark.chars().count());
    let items = picker.entries.iter().map(|entry| {
        let lead = if entry.marked { &mark } else { &blank };
        ListItem::new(Line::styled(
            format!("{lead}{}", entry.text),
            Style::new().fg(color(Slot::Text)),
        ))
    });
    let mut state = ListState::default().with_selected(Some(picker.selected));
    frame.render_widget(Clear, area);
    frame.render_stateful_widget(
        List::new(items)
            .block(bordered(&picker.title))
            .highlight_style(bar()),
        area,
        &mut state,
    );
}

fn typed(frame: &mut Frame<'_>, area: Rect, line: &LineBox) {
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Line::styled(
            format!("{}_", line.text),
            Style::new().fg(color(Slot::Text)),
        ))
        .block(bordered(&line.title).title_bottom(line.hint.clone())),
        area,
    );
}

fn asked(frame: &mut Frame<'_>, placement: &Placement, question: &str) {
    shown(
        frame,
        placement,
        Paragraph::new(Line::styled(
            question.to_string(),
            Style::new().fg(color(Slot::Text)),
        ))
        .wrap(Wrap { trim: true })
        .block(bordered("confirm")),
    );
}

fn noted(frame: &mut Frame<'_>, placement: &Placement, text: &str) {
    shown(
        frame,
        placement,
        Paragraph::new(format!("{text}_"))
            .style(Style::new().fg(color(Slot::Text)))
            .wrap(Wrap { trim: false })
            .block(bordered("note for the agent")),
    );
}
