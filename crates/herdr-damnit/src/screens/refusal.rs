use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};

use crate::theme::Palette;

pub fn draw(frame: &mut Frame<'_>, area: Rect, said: &str, palette: &Palette) {
    frame.render_widget(
        Paragraph::new(Line::styled(said.to_string(), Style::new().fg(palette.red)))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        area,
    );
}
