use ratatui::layout::Rect;

pub struct Placement {
    body: Rect,
    cursor: Option<u16>,
}

impl Placement {
    pub fn new(body: Rect, selected: Option<usize>) -> Self {
        let last = body.height.saturating_sub(1) as usize;
        let cursor = selected.map(|index| body.y + index.min(last) as u16);
        Self { body, cursor }
    }

    pub fn width(&self) -> u16 {
        self.body.width
    }

    pub fn place(&self, height: u16) -> Rect {
        let height = height.min(self.body.height);
        let on_the_legend = Rect {
            y: self.body.bottom() - height,
            height,
            ..self.body
        };
        match self.cursor {
            Some(row) if row >= on_the_legend.y => Rect {
                height,
                ..self.body
            },
            _ => on_the_legend,
        }
    }
}
