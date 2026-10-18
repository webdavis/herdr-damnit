use crate::overlay::LineBox;

pub(super) enum Pending {
    Box(LineBox),
    Label { agent: String },
}
