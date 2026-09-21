//! The pane's rules, over `std` and `jiff` alone: no serde, no ratatui, no process call.

mod cursor;
mod dates;
mod marks;
mod object;
mod oid;
mod priority;
mod rows;
mod slot;
mod views;

pub use cursor::Cursor;
pub use dates::{Date, DueState, due_state, long, parse_date, short};
pub use marks::{IconSet, Mark};
pub use object::{Attendee, EventFields, Kind, Object, TaskFields};
pub use oid::Oid;
pub use priority::Priority;
pub use rows::{ObjectRow, Row, RowStyle, Segment, StagingMarks, rows};
pub use slot::Slot;
pub use views::{DONE_QUERY, MAX_NUMBERED_VIEW, OPEN, OPEN_QUERY, View, Views};
