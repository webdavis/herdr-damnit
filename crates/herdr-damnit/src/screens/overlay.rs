use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap};

use herdr_damnit_domain::{IconSet, Mark, Slot};

use crate::overlay::{LineBox, Overlay, Picker};
use crate::theme::{bar, color, ground};

const LINE_BOX_HEIGHT: u16 = 3;

const CONFIRM_HEIGHT: u16 = 4;

pub fn draw(frame: &mut Frame<'_>, area: Rect, overlay: &Overlay, icons: IconSet) {
    match overlay {
        Overlay::View(picker) | Overlay::Label(_, picker) | Overlay::Path(_, picker) => {
            picked(frame, area, picker, icons)
        }
        Overlay::Line(line) => typed(frame, area, line),
        Overlay::Confirm(confirm) => asked(frame, area, &confirm.question),
        Overlay::Note(note) => noted(frame, area, &note.text),
    }
}

fn bordered(title: &str) -> Block<'static> {
    Block::bordered()
        .style(ground())
        .title(title.to_string())
        .border_style(Style::new().fg(color(Slot::Dim1)))
}

fn picked(frame: &mut Frame<'_>, area: Rect, picker: &Picker, icons: IconSet) {
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
    let area = Rect {
        height: LINE_BOX_HEIGHT.min(area.height),
        ..area
    };
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

fn asked(frame: &mut Frame<'_>, area: Rect, question: &str) {
    let area = Rect {
        height: CONFIRM_HEIGHT.min(area.height),
        ..area
    };
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Line::styled(
            question.to_string(),
            Style::new().fg(color(Slot::Text)),
        ))
        .wrap(Wrap { trim: true })
        .block(bordered("confirm")),
        area,
    );
}

fn noted(frame: &mut Frame<'_>, area: Rect, text: &str) {
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(format!("{text}_"))
            .style(Style::new().fg(color(Slot::Text)))
            .wrap(Wrap { trim: false })
            .block(bordered("note for the agent").title_bottom("<C-d> sends, <Esc> cancels")),
        area,
    );
}
