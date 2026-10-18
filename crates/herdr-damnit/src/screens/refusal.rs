use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use super::list::cut_to;
use crate::theme::color;
use herdr_damnit_domain::Slot;

pub fn draw(frame: &mut Frame<'_>, area: Rect, said: &str) {
    frame.render_widget(
        Paragraph::new(Line::styled(
            cut_to(said, area.width as usize),
            Style::new().fg(color(Slot::Red)),
        )),
        area,
    );
}
