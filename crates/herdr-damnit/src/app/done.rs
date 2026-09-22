//! The Done screen's own model: the completed objects, grouped by the day each was completed on.

use herdr_damnit_domain::{Date, Object, Row, Segment, Slot, long};

use super::App;

impl App {
    /// The Done screen's rows: newest completion first, the date leading each row so the dates
    /// line up down the pane. A task completed in the working layer sits in no commit and has no
    /// date, so it goes on top under its own heading, where it also reads as waiting to be staged.
    pub fn done_rows(&self) -> Vec<Row> {
        let mut dated: Vec<(&Date, &Object)> = Vec::new();
        let mut undated: Vec<&Object> = Vec::new();
        for object in &self.done_objects {
            match self.completed.get(&object.oid) {
                Some(day) => dated.push((day, object)),
                None => undated.push(object),
            }
        }
        dated.sort_by(|left, right| {
            right
                .0
                .cmp(left.0)
                .then_with(|| left.1.subject.cmp(&right.1.subject))
        });
        undated.sort_by(|left, right| left.subject.cmp(&right.subject));

        let mut rows = Vec::new();
        if !undated.is_empty() {
            rows.push(Row::Heading("not committed".to_string()));
            rows.extend(undated.into_iter().map(|object| done_row(object, None)));
        }
        rows.extend(
            dated
                .into_iter()
                .map(|(day, object)| done_row(object, Some(*day))),
        );
        rows
    }
}

/// One completed object as the Done screen draws it: the day it was completed on, then its
/// subject. An object with no commit behind it draws the indent the dates occupy instead.
fn done_row(object: &Object, day: Option<Date>) -> Row {
    let lead = match day {
        Some(day) => Segment {
            text: format!("{}  ", long(day)),
            slot: Slot::Dim1,
        },
        None => Segment {
            text: "  ".to_string(),
            slot: Slot::Text,
        },
    };
    Row::Object(herdr_damnit_domain::ObjectRow {
        oid: object.oid.clone(),
        subject: object.subject.clone(),
        segments: vec![
            lead,
            Segment {
                text: object.subject.clone(),
                slot: Slot::Text,
            },
        ],
    })
}
