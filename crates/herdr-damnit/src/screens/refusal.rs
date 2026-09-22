//! The refusal screen: why the pane will not draw rows. It is the whole frame, with no hint line,
//! because none of the keys a hint line offers would reach a `dam` this pane has refused.

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
