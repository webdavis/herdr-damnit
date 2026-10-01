use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap};

use herdr_damnit_domain::{IconSet, Mark};

use crate::overlay::{LineBox, Overlay, Picker};
use crate::theme::Palette;

const LINE_BOX_HEIGHT: u16 = 3;

const CONFIRM_HEIGHT: u16 = 4;

pub fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    overlay: &Overlay,
    icons: IconSet,
    palette: &Palette,
) {
    match overlay {
        Overlay::View(picker) | Overlay::Label(_, picker) | Overlay::Path(_, picker) => {
            picked(frame, area, picker, icons, palette)
        }
        Overlay::Line(line) => typed(frame, area, line, palette),
        Overlay::Confirm(confirm) => asked(frame, area, &confirm.question, palette),
        Overlay::Note(note) => noted(frame, area, &note.text, palette),
    }
}

fn bordered(title: &str, palette: &Palette) -> Block<'static> {
    Block::bordered()
        .title(title.to_string())
        .border_style(Style::new().fg(palette.dim1))
}

fn picked(frame: &mut Frame<'_>, area: Rect, picker: &Picker, icons: IconSet, palette: &Palette) {
    let mark = format!("{} ", Mark::Staged.glyph(icons));
    let blank = " ".repeat(mark.chars().count());
    let items = picker.entries.iter().map(|entry| {
        let lead = if entry.marked { &mark } else { &blank };
        ListItem::new(Line::styled(
            format!("{lead}{}", entry.text),
            Style::new().fg(palette.text),
        ))
    });
    let mut state = ListState::default().with_selected(Some(picker.selected));
    frame.render_widget(Clear, area);
    frame.render_stateful_widget(
        List::new(items)
            .block(bordered(&picker.title, palette))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED)),
        area,
        &mut state,
    );
}

fn typed(frame: &mut Frame<'_>, area: Rect, line: &LineBox, palette: &Palette) {
    let area = Rect {
        height: LINE_BOX_HEIGHT.min(area.height),
        ..area
    };
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Line::styled(
            format!("{}_", line.text),
            Style::new().fg(palette.text),
        ))
        .block(bordered(&line.title, palette).title_bottom(line.hint.clone())),
        area,
    );
}

fn asked(frame: &mut Frame<'_>, area: Rect, question: &str, palette: &Palette) {
    let area = Rect {
        height: CONFIRM_HEIGHT.min(area.height),
        ..area
    };
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Line::styled(
            question.to_string(),
            Style::new().fg(palette.text),
        ))
        .wrap(Wrap { trim: true })
        .block(bordered("confirm", palette)),
        area,
    );
}

fn noted(frame: &mut Frame<'_>, area: Rect, text: &str, palette: &Palette) {
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(format!("{text}_"))
            .style(Style::new().fg(palette.text))
            .wrap(Wrap { trim: false })
            .block(
                bordered("note for the agent", palette).title_bottom("<C-d> sends, <Esc> cancels"),
            ),
        area,
    );
}
