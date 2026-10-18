use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::ListState;

use crate::app::App;
use crate::screens::list::item;
use crate::theme::bar;

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut state = ListState::default()
        .with_selected(app.status.selected_oid().map(|_| app.status.selected()));
    frame.render_stateful_widget(
        ratatui::widgets::List::new(app.status.rows().iter().map(|row| item(row, area.width)))
            .highlight_style(bar()),
        area,
        &mut state,
    );
}
