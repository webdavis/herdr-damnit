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
    pub fn move_by(&mut self, steps: isize) {
        let last = self.entries.len().saturating_sub(1) as isize;
        self.selected = (self.selected as isize)
            .saturating_add(steps)
            .clamp(0, last) as usize;
    }
}
