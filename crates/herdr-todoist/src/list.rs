//! The rows the pane draws: every open task grouped by project and then section, with subtasks
//! folded under their parent. Pure functions over what the client returned, so the whole layout
//! is testable without a terminal or a network.

use std::collections::{HashMap, HashSet};

use todoist::{Project, Section, Task};

use crate::icons::{self, Marks};
use crate::theme::Slot;

/// A deeply nested chain stops here. Real task trees are a few levels deep; the limit is what
/// keeps malformed input from recursing without end.
const MAX_DEPTH: usize = 8;

/// The API's priority range. It runs the other way from the app's wording: 4 is the app's p1, the
/// most urgent, and 1 is its p4, which the app draws as no priority at all.
pub const LOWEST_PRIORITY: u8 = 1;
pub const HIGHEST_PRIORITY: u8 = 4;

/// One line of the list. A header carries its own indentation, as does a task, so drawing a row
/// is printing its text.
#[derive(Debug, PartialEq, Eq)]
pub enum Row {
    /// A project or section heading.
    Header(String),
    Task(TaskRow),
}

/// A run of a task line drawn in one color. A line is a list of these, so the marks are painted
/// by what they mean while the title stays the pane's plain text.
#[derive(Debug, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub slot: Slot,
}

impl Segment {
    pub fn new(text: impl Into<String>, slot: Slot) -> Self {
        Self {
            text: text.into(),
            slot,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TaskRow {
    pub id: String,
    /// The drawn line, in the colors it is drawn in.
    pub text: Vec<Segment>,
    /// The task's own title, without the indentation and the decorations `text` carries, which is
    /// what a brief handed to an agent names the task by.
    pub content: String,
    /// The task's own long text, carried on the row so `<CR>` draws the detail without a second
    /// read of a task the list already fetched.
    pub description: String,
    /// The API's own priority, 1 to 4 with 4 the most urgent, which `p` cycles from.
    pub priority: u8,
    /// The task's label names, which the label picker marks and an update rewrites whole.
    pub labels: Vec<String>,
    /// The due date as the API's own first ten characters, absent when the task has none.
    pub due: Option<String>,
}

impl Row {
    /// The task this row is, or `None` for a heading.
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Self::Header(_) => None,
            Self::Task(task) => Some(&task.id),
        }
    }

    /// The task this row is, or `None` for a heading.
    pub fn task(&self) -> Option<&TaskRow> {
        match self {
            Self::Header(_) => None,
            Self::Task(task) => Some(task),
        }
    }

    /// The whole line as plain text, which is what a width is measured over and what a test
    /// compares.
    pub fn text(&self) -> String {
        match self {
            Self::Header(text) => text.clone(),
            Self::Task(task) => task.text.iter().map(|part| part.text.as_str()).collect(),
        }
    }
}

/// Build the rows. A project or section with no open task of its own gets no heading. A task whose
/// project is missing from `projects` is grouped under its project id rather than dropped, a task
/// with no project_id at all is grouped under a literal "(no project)" heading, and a task whose
/// section_id names no section in `sections` is treated as unfiled rather than dropped.
pub fn build(
    tasks: &[Task],
    projects: &[Project],
    sections: &[Section],
    marks: &Marks,
) -> Vec<Row> {
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
            marks,
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
            emit(&mut rows, in_section.into_iter(), 2, &children, marks);
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
    order.extend(unknown.into_iter().map(|id| {
        let name = if id.is_empty() {
            "(no project)".to_string()
        } else {
            id.clone()
        };
        (id, name)
    }));
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
    marks: &Marks,
) {
    if depth > MAX_DEPTH {
        return;
    }
    for task in tasks {
        let subtasks = children.get(task.id.as_str()).map_or(0, Vec::len);
        rows.push(Row::Task(TaskRow {
            id: task.id.clone(),
            text: task_line(task, depth, subtasks, marks),
            content: task.content.clone(),
            description: task.description.clone(),
            priority: task.priority.clamp(LOWEST_PRIORITY, HIGHEST_PRIORITY),
            labels: task.labels.clone(),
            due: task.due.as_ref().map(|due| date_of(&due.date)),
        }));
        if let Some(subtasks) = children.get(task.id.as_str()) {
            emit(rows, subtasks.iter().copied(), depth + 1, children, marks);
        }
    }
}

/// A due value's date, which is its first ten characters whether it carries a time or not.
fn date_of(due: &str) -> String {
    due.get(..10).unwrap_or(due).to_string()
}

/// The task's line: its marks first, then its title, then its subtask count. The marks lead so
/// that a title too long for the pane is what the drawing truncates, and the labels are counted
/// rather than named, which is what keeps the marks inside a side pane's width.
fn task_line(task: &Task, depth: usize, subtasks: usize, marks: &Marks) -> Vec<Segment> {
    let icons = &marks.icons;
    let mut line = vec![Segment::new(indent(depth), Slot::Text)];
    if let Some(icon) = icons.priority(task.priority) {
        mark(&mut line, icon.to_string(), priority_slot(task.priority));
    }
    let due = task.due.as_ref().map(|due| date_of(&due.date));
    let state = icons::due(due.as_deref(), &marks.today);
    if let Some(icon) = icons.due(state) {
        let slot = due_slot(state);
        let shown = due.as_deref().map_or(String::new(), |date| {
            when(date, &marks.today, state).to_string()
        });
        mark(&mut line, format!("{icon}{shown}"), slot);
    }
    if task.due.as_ref().is_some_and(|due| due.is_recurring) {
        mark(&mut line, icons.recurring.to_string(), Slot::Green);
    }
    if !task.labels.is_empty() {
        let count = task.labels.len();
        mark(&mut line, format!("{}{count}", icons.labels), Slot::Purple);
    }
    if line.len() > 1 {
        line.push(Segment::new(" ", Slot::Text));
    }
    line.push(Segment::new(task.content.clone(), Slot::Text));
    if subtasks > 0 {
        line.push(Segment::new(format!("  ({subtasks})"), Slot::Dim1));
    }
    line
}

/// Add a mark to the line, spaced off whatever is already there.
fn mark(line: &mut Vec<Segment>, text: String, slot: Slot) {
    if line.len() > 1 {
        line.push(Segment::new(" ", Slot::Text));
    }
    line.push(Segment::new(text, slot));
}

/// What the due mark carries beside it: nothing for a task due today, which the mark already
/// says; the month and day for another day of this year; the whole date when the year differs,
/// so a date a year old cannot read as one a few days away.
fn when<'a>(date: &'a str, today: &str, state: icons::Due) -> &'a str {
    match state {
        icons::Due::Today | icons::Due::None => "",
        _ if date.get(..4) == today.get(..4) => date.get(5..).unwrap_or(date),
        _ => date,
    }
}

/// The color a priority mark takes: the app's own red, orange and blue for p1, p2 and p3.
fn priority_slot(priority: u8) -> Slot {
    match priority {
        4 => Slot::Red,
        3 => Slot::Orange,
        _ => Slot::Blue,
    }
}

fn due_slot(state: icons::Due) -> Slot {
    match state {
        icons::Due::Overdue => Slot::Red,
        icons::Due::Today => Slot::Yellow,
        _ => Slot::Blue,
    }
}

fn indent(depth: usize) -> String {
    "  ".repeat(depth)
}

#[cfg(test)]
pub(crate) mod tests;
