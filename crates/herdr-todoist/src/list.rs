//! The rows the pane draws: every open task grouped by project and then section, with subtasks
//! folded under their parent. Pure functions over what the client returned, so the whole layout
//! is testable without a terminal or a network.

use std::collections::{HashMap, HashSet};

use todoist::{Project, Section, Task};

/// A deeply nested chain stops here. Real task trees are a few levels deep; the limit is what
/// keeps malformed input from recursing without end.
const MAX_DEPTH: usize = 8;

/// One line of the list. A header carries its own indentation, as does a task, so drawing a row
/// is printing its text.
#[derive(Debug, PartialEq, Eq)]
pub enum Row {
    /// A project or section heading.
    Header(String),
    Task(TaskRow),
}

#[derive(Debug, PartialEq, Eq)]
pub struct TaskRow {
    pub id: String,
    pub text: String,
}

impl Row {
    /// The task this row is, or `None` for a heading.
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Self::Header(_) => None,
            Self::Task(task) => Some(&task.id),
        }
    }

    pub fn text(&self) -> &str {
        match self {
            Self::Header(text) => text,
            Self::Task(task) => &task.text,
        }
    }
}

/// Build the rows. A project or section with no open task of its own gets no heading. A task whose
/// project is missing from `projects` is grouped under its project id rather than dropped, and a
/// task whose section_id names no section in `sections` is treated as unfiled rather than dropped.
pub fn build(tasks: &[Task], projects: &[Project], sections: &[Section]) -> Vec<Row> {
    let visible: HashSet<&str> = tasks.iter().map(|task| task.id.as_str()).collect();
    let mut children: HashMap<&str, Vec<&Task>> = HashMap::new();
    let mut roots: Vec<&Task> = Vec::new();
    for task in tasks {
        match task.parent_id.as_deref().filter(|id| visible.contains(id)) {
            Some(parent) => children.entry(parent).or_default().push(task),
            None => roots.push(task),
        }
    }
    for siblings in children.values_mut() {
        sort_tasks(siblings);
    }
    sort_tasks(&mut roots);

    let mut rows = Vec::new();
    for (project_id, project_name) in project_order(&roots, projects) {
        let in_project: Vec<&Task> = roots
            .iter()
            .copied()
            .filter(|task| task.project_id.as_deref().unwrap_or_default() == project_id)
            .collect();
        if in_project.is_empty() {
            continue;
        }
        rows.push(Row::Header(project_name));
        let known_sections: HashSet<&str> = sections
            .iter()
            .filter(|section| section.project_id == project_id)
            .map(|section| section.id.as_str())
            .collect();
        emit(
            &mut rows,
            in_project.iter().copied().filter(|task| {
                task.section_id
                    .as_deref()
                    .is_none_or(|id| !known_sections.contains(id))
            }),
            1,
            &children,
        );
        for section in sorted_sections(sections, &project_id) {
            let in_section: Vec<&Task> = in_project
                .iter()
                .copied()
                .filter(|task| task.section_id.as_deref() == Some(section.id.as_str()))
                .collect();
            if in_section.is_empty() {
                continue;
            }
            rows.push(Row::Header(format!("{}{}", indent(1), section.name)));
            emit(&mut rows, in_section.into_iter(), 2, &children);
        }
    }
    rows
}

/// Every project that holds a task, in the API's own order, with any project the task list names
/// but `projects` does not last.
fn project_order(roots: &[&Task], projects: &[Project]) -> Vec<(String, String)> {
    let mut known: Vec<&Project> = projects.iter().collect();
    known.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then(left.name.cmp(&right.name))
    });
    let mut order: Vec<(String, String)> = known
        .into_iter()
        .map(|project| (project.id.clone(), project.name.clone()))
        .collect();
    let mut unknown: Vec<String> = roots
        .iter()
        .map(|task| task.project_id.clone().unwrap_or_default())
        .filter(|id| !order.iter().any(|(known, _)| known == id))
        .collect();
    unknown.sort();
    unknown.dedup();
    order.extend(unknown.into_iter().map(|id| (id.clone(), id)));
    order
}

fn sorted_sections<'a>(sections: &'a [Section], project_id: &str) -> Vec<&'a Section> {
    let mut of_project: Vec<&Section> = sections
        .iter()
        .filter(|section| section.project_id == project_id)
        .collect();
    of_project.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then(left.name.cmp(&right.name))
    });
    of_project
}

fn sort_tasks(tasks: &mut [&Task]) {
    tasks.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then(left.content.cmp(&right.content))
    });
}

/// Push a task and, below it, its subtasks one level deeper.
fn emit<'a>(
    rows: &mut Vec<Row>,
    tasks: impl Iterator<Item = &'a Task>,
    depth: usize,
    children: &HashMap<&str, Vec<&'a Task>>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    for task in tasks {
        let subtasks = children.get(task.id.as_str()).map_or(0, Vec::len);
        rows.push(Row::Task(TaskRow {
            id: task.id.clone(),
            text: task_text(task, depth, subtasks),
        }));
        if let Some(subtasks) = children.get(task.id.as_str()) {
            emit(rows, subtasks.iter().copied(), depth + 1, children);
        }
    }
}

/// The task's line: its content, then whichever of the due date, the priority, the labels and the
/// subtask count it has.
fn task_text(task: &Task, depth: usize, subtasks: usize) -> String {
    let mut parts = vec![format!("{}{}", indent(depth), task.content)];
    if let Some(due) = &task.due {
        parts.push(due.date.get(..10).unwrap_or(&due.date).to_string());
    }
    if task.priority > 1 {
        parts.push(format!("p{}", 5 - task.priority.min(4)));
    }
    parts.extend(task.labels.iter().map(|label| format!("@{label}")));
    if subtasks > 0 {
        parts.push(format!("({subtasks})"));
    }
    parts.join("  ")
}

fn indent(depth: usize) -> String {
    "  ".repeat(depth)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Tasks, projects and sections are built from API-shaped JSON, so these tests pin the field
    /// names the list reads as well as the layout it produces.
    pub(crate) fn task(json: &str) -> Task {
        serde_json::from_str(json).expect("task")
    }

    fn project(json: &str) -> Project {
        serde_json::from_str(json).expect("project")
    }

    fn section(json: &str) -> Section {
        serde_json::from_str(json).expect("section")
    }

    fn texts(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(Row::text).collect()
    }

    #[test]
    fn tasks_are_grouped_under_their_project_and_section() {
        let rows = build(
            &[
                task(r#"{"id":"1","content":"loose","project_id":"p1","child_order":1}"#),
                task(
                    r#"{"id":"2","content":"filed","project_id":"p1","section_id":"s1","child_order":2}"#,
                ),
                task(r#"{"id":"3","content":"later","project_id":"p2","child_order":1}"#),
            ],
            &[
                project(r#"{"id":"p2","name":"Second","child_order":2}"#),
                project(r#"{"id":"p1","name":"First","child_order":1}"#),
            ],
            &[section(
                r#"{"id":"s1","name":"Doing","project_id":"p1","section_order":1}"#,
            )],
        );

        assert_eq!(
            texts(&rows),
            vec![
                "First",
                "  loose",
                "  Doing",
                "    filed",
                "Second",
                "  later",
            ]
        );
    }

    #[test]
    fn a_project_with_no_open_task_gets_no_heading() {
        let rows = build(
            &[],
            &[project(r#"{"id":"p1","name":"Empty"}"#)],
            &[section(r#"{"id":"s1","name":"Doing","project_id":"p1"}"#)],
        );

        assert!(rows.is_empty(), "{rows:?}");
    }

    #[test]
    fn subtasks_are_folded_under_their_parent_and_counted_on_it() {
        let rows = build(
            &[
                task(r#"{"id":"1","content":"parent","project_id":"p1","child_order":1}"#),
                task(
                    r#"{"id":"3","content":"second child","project_id":"p1","parent_id":"1","child_order":2}"#,
                ),
                task(
                    r#"{"id":"2","content":"first child","project_id":"p1","parent_id":"1","child_order":1}"#,
                ),
                task(r#"{"id":"4","content":"grandchild","project_id":"p1","parent_id":"2"}"#),
            ],
            &[project(r#"{"id":"p1","name":"First"}"#)],
            &[],
        );

        assert_eq!(
            texts(&rows),
            vec![
                "First",
                "  parent  (2)",
                "    first child  (1)",
                "      grandchild",
                "    second child",
            ]
        );
    }

    #[test]
    fn a_subtask_whose_parent_is_not_open_stands_on_its_own() {
        let rows = build(
            &[task(
                r#"{"id":"2","content":"orphan","project_id":"p1","parent_id":"gone"}"#,
            )],
            &[project(r#"{"id":"p1","name":"First"}"#)],
            &[],
        );

        assert_eq!(texts(&rows), vec!["First", "  orphan"]);
    }

    #[test]
    fn a_line_carries_the_due_date_the_priority_and_the_labels() {
        let rows = build(
            &[task(
                r#"{"id":"1","content":"file taxes","project_id":"p1","priority":4,
                     "labels":["home","slow"],"due":{"date":"2026-09-18T09:00:00Z"}}"#,
            )],
            &[project(r#"{"id":"p1","name":"First"}"#)],
            &[],
        );

        assert_eq!(
            texts(&rows),
            vec!["First", "  file taxes  2026-09-18  p1  @home  @slow"]
        );
    }

    #[test]
    fn the_lowest_priority_is_left_off_the_line() {
        let rows = build(
            &[task(
                r#"{"id":"1","content":"someday","project_id":"p1","priority":1}"#,
            )],
            &[project(r#"{"id":"p1","name":"First"}"#)],
            &[],
        );

        assert_eq!(texts(&rows), vec!["First", "  someday"]);
    }

    #[test]
    fn a_task_whose_section_is_unknown_is_treated_as_unfiled() {
        let rows = build(
            &[task(
                r#"{"id":"1","content":"hidden","project_id":"p1","section_id":"s9"}"#,
            )],
            &[project(r#"{"id":"p1","name":"First"}"#)],
            &[],
        );

        assert_eq!(texts(&rows), vec!["First", "  hidden"]);
    }

    #[test]
    fn a_task_whose_project_is_unknown_is_grouped_under_its_project_id() {
        let rows = build(
            &[task(r#"{"id":"1","content":"stray","project_id":"p9"}"#)],
            &[],
            &[],
        );

        assert_eq!(texts(&rows), vec!["p9", "  stray"]);
    }
}
