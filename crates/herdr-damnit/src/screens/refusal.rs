use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};

use crate::theme::color;
use herdr_damnit_domain::Slot;

pub fn draw(frame: &mut Frame<'_>, area: Rect, said: &str) {
    frame.render_widget(
        Paragraph::new(Line::styled(
            said.to_string(),
            Style::new().fg(color(Slot::Red)),
        ))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true }),
        area,
    );
}
