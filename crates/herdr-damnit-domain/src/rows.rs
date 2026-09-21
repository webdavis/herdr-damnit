//! The rows the pane draws: every object of the showing view grouped by its path, with the marks
//! leading so the subject is what gets cut when the pane is narrow.

use crate::{Date, DueState, IconSet, Mark, Object, Oid, Slot, due_state, short};

#[cfg(test)]
mod tests;

/// The heading an object with no path of its own is grouped under.
const NO_PATH: &str = "(no path)";

/// A run of a row drawn in one colour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub slot: Slot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectRow {
    pub oid: Oid,
    /// The object's own subject, without the indentation and the marks, which is what a brief
    /// handed to an agent names it by.
    pub subject: String,
    pub segments: Vec<Segment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    Heading(String),
    Object(ObjectRow),
}

impl Row {
    pub fn oid(&self) -> Option<&Oid> {
        match self {
            Self::Heading(_) => None,
            Self::Object(row) => Some(&row.oid),
        }
    }

    /// The whole line as plain text, which is what a width is measured over and what a test
    /// compares.
    pub fn text(&self) -> String {
        match self {
            Self::Heading(text) => text.clone(),
            Self::Object(row) => row.segments.iter().map(|part| part.text.as_str()).collect(),
        }
    }
}

/// The staging mark an oid carries. The Status model implements it, so a row builder needs no
/// second read of `dam status`.
pub trait StagingMarks {
    fn mark_of(&self, oid: &Oid) -> Option<Mark>;
}

pub struct RowStyle {
    pub icons: IconSet,
    pub today: Date,
}

pub fn rows(objects: &[Object], marks: &dyn StagingMarks, style: RowStyle) -> Vec<Row> {
    let mut paths: Vec<&str> = objects.iter().map(heading).collect();
    paths.sort_unstable();
    paths.dedup();

    let mut rows = Vec::new();
    for path in paths {
        rows.push(Row::Heading(path.to_string()));
        let mut under: Vec<&Object> = objects
            .iter()
            .filter(|object| heading(object) == path)
            .collect();
        under.sort_by(|left, right| left.subject.cmp(&right.subject));
        rows.extend(
            under
                .into_iter()
                .map(|object| object_row(object, marks, &style)),
        );
    }
    rows
}

fn heading(object: &Object) -> &str {
    match object.path.trim_end_matches('/') {
        "" => NO_PATH,
        path => path,
    }
}

fn object_row(object: &Object, marks: &dyn StagingMarks, style: &RowStyle) -> Row {
    let mut segments = vec![Segment {
        text: "  ".to_string(),
        slot: Slot::Text,
    }];
    if let Some(mark) = marks.mark_of(&object.oid) {
        push(&mut segments, mark, style.icons);
    }
    if let Some(mark) = Mark::of_priority(object.priority()) {
        push(&mut segments, mark, style.icons);
    }
    let state = due_state(object.due(), style.today);
    if let Some(mark) = Mark::of_due(state) {
        push(&mut segments, mark, style.icons);
        if let (Some(date), DueState::Overdue | DueState::Upcoming) = (object.due(), state) {
            segments.push(Segment {
                text: format!("{} ", short(date)),
                slot: mark.slot(),
            });
        }
    }
    if object.recurrence.is_some() {
        push(&mut segments, Mark::Recurring, style.icons);
    }
    segments.push(Segment {
        text: object.subject.clone(),
        slot: Slot::Text,
    });
    if !object.labels.is_empty() {
        let mark = Mark::Labels(object.labels.len());
        segments.push(Segment {
            text: format!(" {}", mark.glyph(style.icons)),
            slot: mark.slot(),
        });
    }
    Row::Object(ObjectRow {
        oid: object.oid.clone(),
        subject: object.subject.clone(),
        segments,
    })
}

/// A mark and the space that separates it from the next one.
fn push(segments: &mut Vec<Segment>, mark: Mark, icons: IconSet) {
    segments.push(Segment {
        text: format!("{} ", mark.glyph(icons)),
        slot: mark.slot(),
    });
}
