use crossterm::event::KeyCode;

use super::Answer;

pub struct Picker {
    pub title: String,
    pub entries: Vec<PickerEntry>,
    pub selected: usize,
}

pub struct PickerEntry {
    pub text: String,
    pub marked: bool,
}

impl Picker {
    pub fn answer(&mut self, code: KeyCode) -> Answer {
        match code {
            KeyCode::Enter => return Answer::Take,
            KeyCode::Esc => return Answer::Cancel,
            KeyCode::Char('j') | KeyCode::Down => self.move_by(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_by(-1),
            _ => {}
        }
        Answer::Open
    }

    fn move_by(&mut self, steps: isize) {
        let last = self.entries.len().saturating_sub(1) as isize;
        self.selected = (self.selected as isize)
            .saturating_add(steps)
            .clamp(0, last) as usize;
    }
}
