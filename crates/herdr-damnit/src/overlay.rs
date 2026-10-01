mod line;
mod picker;

pub use line::{LineBox, LinePurpose};
pub use picker::{Picker, PickerEntry};

pub enum Overlay {
    View(Picker),
    Line(LineBox),
}

pub enum Answer {
    Open,
    Take,
    Cancel,
}
