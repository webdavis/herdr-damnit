use crossterm::event::KeyEvent;
use herdr_damnit_application::{JobKind, SyncKind, argv};

use super::{After, App};
use crate::overlay::{Answer, LineBox, LinePurpose, Overlay};

impl App {
    pub(super) fn overlay_key(&mut self, overlay: Overlay, key: KeyEvent) -> After {
        match overlay {
            Overlay::View(mut picker) => match picker.answer(key.code) {
                Answer::Open => self.reopen(Overlay::View(picker)),
                Answer::Take => self.show_view(picker.selected),
                Answer::Cancel => After::Stay,
            },
            Overlay::Line(mut line) => match line.answer(key.code) {
                Answer::Open => self.reopen(Overlay::Line(line)),
                Answer::Take => self.take_line(line),
                Answer::Cancel => After::Stay,
            },
        }
    }

    pub(super) fn open_commit_box(&mut self) -> After {
        let staged = self.stage.staged_count();
        if staged == 0 {
            self.message =
                "nothing staged; press <Space> on a row or A to stage everything.".to_string();
            return After::Stay;
        }
        self.reopen(Overlay::Line(LineBox {
            title: format!(
                "commit {staged} staged change{}",
                if staged == 1 { "" } else { "s" }
            ),
            hint: "<CR> commits, <Esc> cancels".to_string(),
            text: String::new(),
            purpose: LinePurpose::Commit,
        }))
    }

    fn take_line(&mut self, line: LineBox) -> After {
        let text = line.text.trim().to_string();
        match line.purpose {
            LinePurpose::Commit if text.is_empty() => {
                self.message = "a commit needs a message.".to_string();
                self.reopen(Overlay::Line(line))
            }
            LinePurpose::Commit => {
                self.submit(JobKind::Exclusive(SyncKind::Commit), argv::commit(&text));
                After::Stay
            }
        }
    }

    fn reopen(&mut self, overlay: Overlay) -> After {
        self.overlay = Some(overlay);
        After::Stay
    }
}
