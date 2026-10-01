use crossterm::event::KeyCode;
use herdr_damnit_domain::Oid;

use super::Answer;

pub struct LineBox {
    pub title: String,
    pub hint: String,
    pub text: String,
    pub purpose: LinePurpose,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LinePurpose {
    Commit,
    Due(Oid),
    Deadline(Oid),
    New(String),
}

impl LineBox {
    pub fn answer(&mut self, code: KeyCode) -> Answer {
        match code {
            KeyCode::Enter => return Answer::Take,
            KeyCode::Esc => return Answer::Cancel,
            KeyCode::Char(character) => self.text.push(character),
            KeyCode::Backspace => {
                self.text.pop();
            }
            _ => {}
        }
        Answer::Open
    }
}
