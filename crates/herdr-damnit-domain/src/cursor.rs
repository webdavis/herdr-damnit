//! The pane's selection. The cursor belongs to an object's identity rather than to a row number,
//! so a re-read that inserts, removes or reorders rows leaves the highlight on the same object.

use crate::{Oid, Row};

pub struct Cursor {
    rows: Vec<Row>,
    selected: usize,
}

impl Cursor {
    pub fn new(rows: Vec<Row>) -> Self {
        let mut cursor = Self { rows, selected: 0 };
        cursor.selected = cursor.object_indices().first().copied().unwrap_or(0);
        cursor
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn selected_oid(&self) -> Option<&Oid> {
        self.rows.get(self.selected).and_then(Row::oid)
    }

    pub fn object_count(&self) -> usize {
        self.rows.iter().filter(|row| row.oid().is_some()).count()
    }

    /// Move by `steps` object rows, skipping headings and stopping at either end. Reports whether
    /// it moved.
    pub fn move_by(&mut self, steps: isize) -> bool {
        let objects = self.object_indices();
        if objects.is_empty() {
            return false;
        }
        let at = objects
            .iter()
            .position(|index| *index >= self.selected)
            .unwrap_or(objects.len() - 1) as isize;
        let target = at
            .saturating_add(steps)
            .clamp(0, objects.len() as isize - 1) as usize;
        let moved = objects[target] != self.selected;
        self.selected = objects[target];
        moved
    }

    /// Replace the rows, keeping the cursor on the object it was on. When that object is gone the
    /// cursor takes the nearest surviving object below it in the old order, or above it when it
    /// was the last.
    pub fn replace(&mut self, rows: Vec<Row>) {
        let preferred: Vec<Oid> = self.preferences();
        self.rows = rows;
        let objects = self.object_indices();
        self.selected = preferred
            .iter()
            .find_map(|oid| {
                objects
                    .iter()
                    .find(|index| self.rows[**index].oid() == Some(oid))
                    .copied()
            })
            .unwrap_or_else(|| objects.first().copied().unwrap_or(0));
    }

    /// The oid under the cursor, then every one below it in the old order, then every one above it
    /// in reverse, which is the order the cursor falls back through.
    fn preferences(&self) -> Vec<Oid> {
        let objects = self.object_indices();
        let at = objects.iter().position(|index| *index == self.selected);
        let Some(at) = at else {
            return Vec::new();
        };
        let oid = |index: &usize| self.rows[*index].oid().cloned();
        objects[at..]
            .iter()
            .filter_map(oid)
            .chain(objects[..at].iter().rev().filter_map(oid))
            .collect()
    }

    fn object_indices(&self) -> Vec<usize> {
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.oid().is_some())
            .map(|(index, _)| index)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ObjectRow, Segment, Slot};

    fn row(oid: &str) -> Row {
        Row::Object(ObjectRow {
            oid: Oid::new(oid),
            subject: oid.to_string(),
            segments: vec![Segment {
                text: oid.to_string(),
                slot: Slot::Text,
            }],
        })
    }

    fn listing(oids: &[&str]) -> Vec<Row> {
        let mut rows = vec![Row::Heading("home".to_string())];
        rows.extend(oids.iter().map(|oid| row(oid)));
        rows
    }

    #[test]
    fn the_cursor_starts_on_the_first_object_rather_than_the_heading() {
        let cursor = Cursor::new(listing(&["a", "b"]));
        assert_eq!(cursor.selected_oid(), Some(&Oid::new("a")));
    }

    #[test]
    fn moving_skips_headings_and_stops_at_either_end() {
        let mut cursor = Cursor::new(listing(&["a", "b"]));
        assert!(cursor.move_by(1));
        assert_eq!(cursor.selected_oid(), Some(&Oid::new("b")));
        assert!(!cursor.move_by(1), "it moved past the last object");
        assert!(cursor.move_by(-1));
        assert_eq!(cursor.selected_oid(), Some(&Oid::new("a")));
        assert!(!cursor.move_by(-1), "it moved past the first object");
    }

    #[test]
    fn a_reread_keeps_the_cursor_on_the_object_it_was_on() {
        let mut cursor = Cursor::new(listing(&["a", "b", "c"]));
        cursor.move_by(2);
        cursor.replace(listing(&["z", "a", "b", "c"]));
        assert_eq!(cursor.selected_oid(), Some(&Oid::new("c")));
    }

    #[test]
    fn an_object_that_is_gone_hands_the_cursor_to_the_next_one_below_it() {
        let mut cursor = Cursor::new(listing(&["a", "b", "c"]));
        cursor.move_by(1);
        cursor.replace(listing(&["a", "c"]));
        assert_eq!(cursor.selected_oid(), Some(&Oid::new("c")));
    }

    #[test]
    fn the_last_object_hands_the_cursor_upward_when_it_goes() {
        let mut cursor = Cursor::new(listing(&["a", "b"]));
        cursor.move_by(1);
        cursor.replace(listing(&["a"]));
        assert_eq!(cursor.selected_oid(), Some(&Oid::new("a")));
    }

    /// Task 31 binds a jump to the top and the bottom, whose natural spelling is the largest step
    /// the type holds.
    #[test]
    fn the_largest_step_lands_on_an_end_rather_than_overflowing() {
        let mut cursor = Cursor::new(listing(&["a", "b", "c"]));
        cursor.move_by(1);
        assert!(cursor.move_by(isize::MAX));
        assert_eq!(cursor.selected_oid(), Some(&Oid::new("c")));
        assert!(cursor.move_by(isize::MIN));
        assert_eq!(cursor.selected_oid(), Some(&Oid::new("a")));
    }

    #[test]
    fn an_empty_list_selects_nothing_and_moves_nowhere() {
        let mut cursor = Cursor::new(Vec::new());
        assert_eq!(cursor.selected_oid(), None);
        assert!(!cursor.move_by(1));
        assert_eq!(cursor.object_count(), 0);
    }
}
