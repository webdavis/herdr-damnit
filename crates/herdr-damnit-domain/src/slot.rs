//! The palette slot a drawn piece of the pane asks for by name. The colours themselves are the
//! binary crate's, resolved from the herdr theme the config names.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Text,
    Dim1,
    Red,
    Green,
    Yellow,
    Orange,
    Purple,
    Blue,
}
