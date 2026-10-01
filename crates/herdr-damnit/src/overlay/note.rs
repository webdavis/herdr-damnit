use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use herdr_damnit_domain::Oid;

use super::Answer;

pub struct NoteBox {
    pub oid: Oid,
    pub text: String,
}

impl NoteBox {
    pub fn answer(&mut self, key: KeyEvent) -> Answer {
        match key.code {
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Answer::Take;
            }
            KeyCode::Esc => return Answer::Cancel,
            KeyCode::Enter => self.text.push('\n'),
            KeyCode::Char(character) => self.text.push(character),
            KeyCode::Backspace => {
                self.text.pop();
            }
            _ => {}
        }
        Answer::Open
    }
}
