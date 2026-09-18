//! The pane's selection. The cursor belongs to a task's identity rather than to a row number, so
//! a refresh that inserts, removes or reorders rows leaves the highlight on the same task.

use crate::list::Row;

pub struct List {
    rows: Vec<Row>,
    selected: usize,
}

impl List {
    pub fn new(rows: Vec<Row>) -> Self {
        let mut list = Self { rows, selected: 0 };
        list.selected = list.first_task().unwrap_or(0);
        list
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The rows, to mark in place. Nothing here reorders them, so the cursor still points at the
    /// row it did.
    pub fn rows_mut(&mut self) -> &mut [Row] {
        &mut self.rows
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    /// The task row under the cursor, which is what `<CR>` opens the detail for.
    pub fn selected_task(&self) -> Option<&crate::list::TaskRow> {
        self.rows.get(self.selected).and_then(Row::task)
    }

    pub fn selected_id(&self) -> Option<&str> {
        self.rows.get(self.selected).and_then(Row::task_id)
    }

    pub fn task_count(&self) -> usize {
        self.rows.iter().filter_map(Row::task_id).count()
    }

    /// Move the cursor by `steps` task rows, skipping headings and stopping at either end.
    /// Reports whether it moved, which is how the completed list knows the cursor is at the
    /// bottom and the next page is wanted.
    pub fn move_cursor(&mut self, steps: isize) -> bool {
        let tasks: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.task_id().is_some())
            .map(|(index, _)| index)
            .collect();
        if tasks.is_empty() {
            return false;
        }
        let at = tasks
            .iter()
            .position(|index| *index >= self.selected)
            .unwrap_or(tasks.len() - 1) as isize;
        let target = (at + steps).clamp(0, tasks.len() as isize - 1) as usize;
        let moved = tasks[target] != self.selected;
        self.selected = tasks[target];
        moved
    }

    /// Replace the rows, keeping the cursor on the task it was on. When that task is gone the
    /// cursor takes the nearest surviving task below it in the old list, or above it when the
    /// task was the last one.
    pub fn refresh(&mut self, rows: Vec<Row>) {
        let previous: Vec<String> = self
            .rows
            .iter()
            .filter_map(Row::task_id)
            .map(str::to_string)
            .collect();
        let anchor = self.selected_id().map(str::to_string);
        self.rows = rows;
        self.selected = anchor
            .and_then(|id| self.locate(&id).or_else(|| self.survivor(&previous, &id)))
            .or_else(|| self.first_task())
            .unwrap_or(0);
    }

    /// Replace the rows with another view's, keeping the cursor on its task when that task is in
    /// the new view too. When it is not, the cursor takes the new view's first task: a neighbour
    /// from the old view says nothing about where to land in a different list.
    pub fn switch(&mut self, rows: Vec<Row>) {
        let anchor = self.selected_id().map(str::to_string);
        self.rows = rows;
        self.selected = anchor
            .and_then(|id| self.locate(&id))
            .or_else(|| self.first_task())
            .unwrap_or(0);
    }

    fn locate(&self, id: &str) -> Option<usize> {
        self.rows.iter().position(|row| row.task_id() == Some(id))
    }

    /// The first task still in the list, looking down from the missing one and then up.
    fn survivor(&self, previous: &[String], missing: &str) -> Option<usize> {
        let at = previous.iter().position(|id| id == missing)?;
        previous[at + 1..]
            .iter()
            .chain(previous[..at].iter().rev())
            .find_map(|id| self.locate(id))
    }

    fn first_task(&self) -> Option<usize> {
        self.rows.iter().position(|row| row.task_id().is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::list::{
        self,
        tests::{marks, task},
    };
    use todoist::{Project, Task};

    fn project() -> Vec<Project> {
        vec![serde_json::from_str(r#"{"id":"p1","name":"First"}"#).expect("project")]
    }

    /// Open tasks named by their content, in the order given.
    fn tasks(contents: &[&str]) -> Vec<Task> {
        contents
            .iter()
            .enumerate()
            .map(|(index, content)| {
                task(&format!(
                    r#"{{"id":"{content}","content":"{content}","project_id":"p1","child_order":{index}}}"#
                ))
            })
            .collect()
    }

    fn list(contents: &[&str]) -> List {
        List::new(list::build(&tasks(contents), &project(), &[], &marks()))
    }

    fn refresh(list: &mut List, contents: &[&str]) {
        list.refresh(list::build(&tasks(contents), &project(), &[], &marks()));
    }

    #[test]
    fn the_cursor_starts_on_the_first_task_not_the_project_heading() {
        let list = list(&["a", "b"]);

        assert_eq!(list.selected_id(), Some("a"));
        assert_eq!(list.task_count(), 2);
    }

    #[test]
    fn the_cursor_moves_by_task_rows_and_stops_at_the_ends() {
        let mut list = list(&["a", "b"]);

        list.move_cursor(1);
        assert_eq!(list.selected_id(), Some("b"));
        list.move_cursor(1);
        assert_eq!(list.selected_id(), Some("b"));
        list.move_cursor(-5);
        assert_eq!(list.selected_id(), Some("a"));
    }

    #[test]
    fn a_task_inserted_above_the_cursor_does_not_move_it() {
        let mut list = list(&["a", "b"]);
        list.move_cursor(1);

        refresh(&mut list, &["a", "inserted", "b"]);

        assert_eq!(list.selected_id(), Some("b"));
        assert_eq!(
            list.selected(),
            3,
            "the row number changed, the task did not"
        );
    }

    #[test]
    fn a_reordered_group_does_not_move_the_cursor_off_its_task() {
        let mut list = list(&["a", "b", "c"]);
        list.move_cursor(2);

        refresh(&mut list, &["c", "b", "a"]);

        assert_eq!(list.selected_id(), Some("c"));
    }

    #[test]
    fn the_cursor_takes_the_next_task_when_its_own_disappears() {
        let mut list = list(&["a", "b", "c"]);
        list.move_cursor(1);

        refresh(&mut list, &["a", "c"]);

        assert_eq!(list.selected_id(), Some("c"));
    }

    #[test]
    fn the_cursor_takes_the_previous_task_when_the_last_one_disappears() {
        let mut list = list(&["a", "b"]);
        list.move_cursor(1);

        refresh(&mut list, &["a"]);

        assert_eq!(list.selected_id(), Some("a"));
    }

    #[test]
    fn the_cursor_falls_back_to_the_first_task_when_nothing_it_knew_survived() {
        let mut list = list(&["a", "b"]);
        list.move_cursor(1);

        refresh(&mut list, &["x", "y"]);

        assert_eq!(list.selected_id(), Some("x"));
    }

    #[test]
    fn a_view_switch_keeps_the_cursor_on_a_task_that_is_in_both_views() {
        let mut list = list(&["a", "b", "c"]);
        list.move_cursor(2);

        list.switch(list::build(&tasks(&["x", "c"]), &project(), &[], &marks()));

        assert_eq!(list.selected_id(), Some("c"));
    }

    #[test]
    fn a_view_switch_lands_on_the_first_task_when_the_cursor_s_task_is_not_there() {
        let mut list = list(&["a", "b", "c"]);
        list.move_cursor(1);

        list.switch(list::build(&tasks(&["x", "y"]), &project(), &[], &marks()));

        assert_eq!(list.selected_id(), Some("x"));
    }

    #[test]
    fn a_switch_to_an_empty_view_points_at_nothing() {
        let mut list = list(&["a"]);

        list.switch(Vec::new());

        assert_eq!(list.selected_id(), None);
        assert_eq!(list.task_count(), 0);
    }

    #[test]
    fn an_empty_refresh_leaves_a_selection_that_points_at_nothing() {
        let mut list = list(&["a"]);

        refresh(&mut list, &[]);

        assert_eq!(list.selected(), 0);
        assert_eq!(list.selected_id(), None);
        assert_eq!(list.task_count(), 0);
    }
}
