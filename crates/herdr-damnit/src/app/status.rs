use herdr_damnit_domain::{Cursor, IconSet, Stage, status_rows};

pub struct StatusScreen {
    stage: Stage,
    cursor: Cursor,
}

impl StatusScreen {
    pub(super) fn new(icons: IconSet) -> Self {
        Self {
            stage: Stage::default(),
            cursor: Cursor::new(status_rows(&Stage::default(), icons)),
        }
    }

    pub fn stage(&self) -> &Stage {
        &self.stage
    }

    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    pub(super) fn cursor_mut(&mut self) -> &mut Cursor {
        &mut self.cursor
    }

    pub(super) fn take(&mut self, stage: Stage, icons: IconSet) {
        self.stage = stage;
        self.cursor.replace(status_rows(&self.stage, icons));
    }
}
