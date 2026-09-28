use herdr_damnit_domain::{Date, Object, Row, Segment, Slot, long};

use super::App;

impl App {
    pub fn done_rows(&self) -> Vec<Row> {
        let mut dated: Vec<(&Date, &Object)> = Vec::new();
        let mut undated: Vec<&Object> = Vec::new();
        for object in &self.done_objects {
            match self.completion_days.get(&object.oid) {
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
