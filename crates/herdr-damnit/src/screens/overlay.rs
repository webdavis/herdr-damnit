use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, List, ListItem, ListState};

use herdr_damnit_domain::{IconSet, Mark};

use crate::overlay::{Overlay, Picker};
use crate::theme::Palette;

pub fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    overlay: &Overlay,
    icons: IconSet,
    palette: &Palette,
) {
    frame.render_widget(Clear, area);
    match overlay {
        Overlay::View(picker) => picked(frame, area, picker, icons, palette),
    }
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
    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::bordered()
                    .title(picker.title.clone())
                    .border_style(Style::new().fg(palette.dim1)),
            )
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED)),
        area,
        &mut state,
    );
}
