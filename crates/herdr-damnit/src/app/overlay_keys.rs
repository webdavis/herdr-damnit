use crossterm::event::{KeyCode, KeyEvent};

use super::{After, App};
use crate::overlay::{Answer, Overlay};

impl App {
    pub(super) fn overlay_key(&mut self, overlay: Overlay, key: KeyEvent) -> After {
        match overlay {
            Overlay::View(mut picker) => match picker.answer(key.code) {
                Answer::Open => self.reopen(Overlay::View(picker)),
                Answer::Take => self.show_view(picker.selected),
                Answer::Cancel => After::Stay,
            },
            Overlay::Label(oid, mut picker) => match picker.answer(key.code) {
                Answer::Open => self.reopen(Overlay::Label(oid, picker)),
                Answer::Take => self.take_label(oid, picker),
                Answer::Cancel => After::Stay,
            },
            Overlay::Path(oid, mut picker) => match picker.answer(key.code) {
                Answer::Open => self.reopen(Overlay::Path(oid, picker)),
                Answer::Take => self.take_path(oid, picker),
                Answer::Cancel => After::Stay,
            },
            Overlay::Line(mut line) => match line.answer(key.code) {
                Answer::Open => self.reopen(Overlay::Line(line)),
                Answer::Take => self.take_line(line),
                Answer::Cancel => After::Stay,
            },
            Overlay::Note(mut note) => match note.answer(key) {
                Answer::Open => self.reopen(Overlay::Note(note)),
                Answer::Take => self.send_note(note),
                Answer::Cancel => After::Stay,
            },
            Overlay::Confirm(confirm) => match key.code {
                KeyCode::Char(pressed) if pressed == confirm.key => self.confirmed(confirm.purpose),
                _ => After::Stay,
            },
        }
    }

    pub(super) fn reopen(&mut self, overlay: Overlay) -> After {
        self.overlay = Some(overlay);
        After::Stay
    }
}
