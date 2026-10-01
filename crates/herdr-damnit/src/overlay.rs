mod confirm;
mod line;
mod note;
mod picker;

use herdr_damnit_domain::Oid;

pub use confirm::{Confirm, ConfirmPurpose};
pub use line::{LineBox, LinePurpose};
pub use note::NoteBox;
pub use picker::{Picker, PickerEntry};

pub enum Overlay {
    View(Picker),
    Label(Oid, Picker),
    Path(Oid, Picker),
    Line(LineBox),
    Note(NoteBox),
    Confirm(Confirm),
}

pub enum Answer {
    Open,
    Take,
    Cancel,
}
