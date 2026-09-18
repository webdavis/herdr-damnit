//! The one-line inputs and pickers a quick edit is asked for in. Two widgets over one drawing:
//! an [`Input`] is a single line the pane draws and types into, and a picker is the list overlay
//! task 105 already draws for views, reused here for labels and for a move's destinations. Both
//! live in the same [`Prompt`], so only one is ever open and the pane never enters an editor.

use todoist::{Destination, Label, Project, Section};

use crate::views::{Picker, Views};

/// What the pane is asking for. Every variant carries the task it is about, except the views
/// picker, which is about the pane, and Quick Add, which is about a task that does not exist yet.
pub enum Prompt {
    /// Which view to show, the picker task 105 opens on `v`.
    Views(Picker),
    /// Whether to delete the task. Nothing is sent until the second `d`.
    Delete { id: String, content: String },
    /// A due date in Todoist's own words, which the API parses.
    Due { id: String, input: Input },
    /// A whole line of Quick Add syntax, which the API parses.
    Add { input: Input },
    /// Which label to toggle on the task. The picker stays open, so several can be toggled.
    Labels {
        id: String,
        choices: Vec<Choice>,
        picker: Picker,
    },
    /// Which project or section to move the task to.
    Move {
        id: String,
        choices: Vec<Move>,
        picker: Picker,
    },
}

/// One label in the picker and whether the task carries it.
pub struct Choice {
    pub name: String,
    pub on: bool,
}

/// One place a task can be moved to.
pub struct Move {
    pub label: String,
    pub to: Destination,
}

impl Prompt {
    pub fn views(views: &Views) -> Self {
        Self::Views(Picker::open(views))
    }

    pub fn delete(id: &str, content: &str) -> Self {
        Self::Delete {
            id: id.to_string(),
            content: content.trim().to_string(),
        }
    }

    pub fn due(id: &str) -> Self {
        Self::Due {
            id: id.to_string(),
            input: Input::new(),
        }
    }

    pub fn add() -> Self {
        Self::Add {
            input: Input::new(),
        }
    }

    /// `on` is the label set the task carries, which is what the picker marks.
    pub fn labels(id: &str, on: &[String], known: &[Label]) -> Self {
        let choices = choices(on, known);
        Self::Labels {
            id: id.to_string(),
            picker: Picker::over(choices.len()),
            choices,
        }
    }

    pub fn move_to(id: &str, projects: &[Project], sections: &[Section]) -> Self {
        let choices = destinations(projects, sections);
        Self::Move {
            id: id.to_string(),
            picker: Picker::over(choices.len()),
            choices,
        }
    }

    /// What the overlay is titled.
    pub fn title(&self) -> &'static str {
        match self {
            Self::Views(_) => "views",
            Self::Delete { .. } => "delete",
            Self::Due { .. } => "due",
            Self::Add { .. } => "add",
            Self::Labels { .. } => "labels",
            Self::Move { .. } => "move",
        }
    }

    /// The overlay's entries and the cursor over them, or `None` when the prompt is one line.
    pub fn entries(&self, views: &Views) -> Option<(Vec<String>, usize)> {
        match self {
            Self::Views(picker) => Some((
                views
                    .names()
                    .enumerate()
                    .map(|(index, name)| format!(" {} {name}", index + 1))
                    .collect(),
                picker.at(),
            )),
            Self::Labels {
                choices, picker, ..
            } => Some((
                choices
                    .iter()
                    .map(|choice| {
                        let mark = if choice.on { "x" } else { " " };
                        format!(" [{mark}] {}", choice.name)
                    })
                    .collect(),
                picker.at(),
            )),
            Self::Move {
                choices, picker, ..
            } => Some((
                choices
                    .iter()
                    .map(|choice| format!(" {}", choice.label))
                    .collect(),
                picker.at(),
            )),
            Self::Delete { .. } | Self::Due { .. } | Self::Add { .. } => None,
        }
    }

    /// The one line the prompt draws when it has no entries: a question for the confirm, and the
    /// text typed so far for an input.
    pub fn line(&self) -> Option<String> {
        match self {
            Self::Delete { content, .. } => Some(format!(" delete {content}?")),
            Self::Due { input, .. } | Self::Add { input } => Some(format!(" {}_", input.text())),
            Self::Views(_) | Self::Labels { .. } | Self::Move { .. } => None,
        }
    }

    /// The key hints for this prompt, short enough for a narrow side pane.
    pub fn hints(&self) -> &'static str {
        match self {
            Self::Views(_) | Self::Move { .. } => "j/k  <CR> pick  <Esc>",
            Self::Delete { .. } => "dd delete  <Esc> keep",
            Self::Due { .. } | Self::Add { .. } => "<CR> send  <Esc> cancel",
            Self::Labels { .. } => "j/k  <CR> toggle  <Esc>",
        }
    }

    /// Move the cursor of whichever picker is open. A one-line prompt has none.
    pub fn move_cursor(&mut self, steps: isize) {
        match self {
            Self::Views(picker) | Self::Labels { picker, .. } | Self::Move { picker, .. } => {
                picker.move_cursor(steps);
            }
            Self::Delete { .. } | Self::Due { .. } | Self::Add { .. } => {}
        }
    }

    /// Type into whichever input is open.
    pub fn input_mut(&mut self) -> Option<&mut Input> {
        match self {
            Self::Due { input, .. } | Self::Add { input } => Some(input),
            Self::Views(_) | Self::Delete { .. } | Self::Labels { .. } | Self::Move { .. } => None,
        }
    }
}

/// A one-line input: the text typed so far, and nothing else. There is no cursor to move and no
/// second line, which is what keeps a quick edit a key press and a line rather than an editor.
#[derive(Debug, Default)]
pub struct Input {
    text: String,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, character: char) {
        self.text.push(character);
    }

    pub fn backspace(&mut self) {
        self.text.pop();
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// An input holding nothing but spaces asks for nothing, so sending it is not a write.
    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }
}

/// Every label the picker offers: the account's own labels, then any label the task carries that
/// the account no longer lists, so a stray one can still be taken off.
fn choices(on: &[String], known: &[Label]) -> Vec<Choice> {
    let mut sorted: Vec<&Label> = known.iter().collect();
    sorted.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then(left.name.cmp(&right.name))
    });
    let mut choices: Vec<Choice> = sorted
        .into_iter()
        .map(|label| Choice {
            on: on.contains(&label.name),
            name: label.name.clone(),
        })
        .collect();
    for name in on {
        if !choices.iter().any(|choice| &choice.name == name) {
            choices.push(Choice {
                name: name.clone(),
                on: true,
            });
        }
    }
    choices
}

/// Every project, each followed by its own sections indented under it, which reads as a tree in a
/// pane too narrow for two columns.
fn destinations(projects: &[Project], sections: &[Section]) -> Vec<Move> {
    let mut ordered: Vec<&Project> = projects.iter().collect();
    ordered.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then(left.name.cmp(&right.name))
    });
    let mut choices = Vec::new();
    for project in ordered {
        choices.push(Move {
            label: project.name.clone(),
            to: Destination::Project(project.id.clone()),
        });
        let mut own: Vec<&Section> = sections
            .iter()
            .filter(|section| section.project_id == project.id)
            .collect();
        own.sort_by(|left, right| {
            left.order
                .cmp(&right.order)
                .then(left.name.cmp(&right.name))
        });
        choices.extend(own.into_iter().map(|section| Move {
            label: format!("  {}", section.name),
            to: Destination::Section(section.id.clone()),
        }));
    }
    choices
}

/// The label set a toggle writes: every label still marked, with the one under the cursor flipped.
pub fn toggled(choices: &[Choice], at: usize) -> Vec<String> {
    choices
        .iter()
        .enumerate()
        .filter(|(index, choice)| choice.on != (*index == at))
        .map(|(_, choice)| choice.name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(labels: &[&str]) -> Vec<String> {
        labels.iter().map(|name| name.to_string()).collect()
    }

    fn label(json: &str) -> Label {
        serde_json::from_str(json).expect("label")
    }

    #[test]
    fn the_label_picker_marks_the_labels_the_task_already_carries() {
        let prompt = Prompt::labels(
            "1",
            &on(&["home"]),
            &[
                label(r#"{"name":"errands","item_order":2}"#),
                label(r#"{"name":"home","item_order":1}"#),
            ],
        );

        let (entries, at) = prompt.entries(&Views::new(&[])).expect("a picker");

        assert_eq!(entries, [" [x] home", " [ ] errands"]);
        assert_eq!(at, 0);
    }

    #[test]
    fn a_label_the_account_no_longer_lists_is_still_offered_so_it_can_come_off() {
        let prompt = Prompt::labels("1", &on(&["retired"]), &[]);

        let (entries, _) = prompt.entries(&Views::new(&[])).expect("a picker");

        assert_eq!(entries, [" [x] retired"]);
    }

    #[test]
    fn a_toggle_adds_the_label_under_the_cursor_and_keeps_the_rest() {
        let choices = choices(
            &on(&["home"]),
            &[
                label(r#"{"name":"home","item_order":1}"#),
                label(r#"{"name":"errands","item_order":2}"#),
            ],
        );

        assert_eq!(toggled(&choices, 1), ["home", "errands"]);
    }

    #[test]
    fn a_toggle_of_a_marked_label_takes_it_off_and_keeps_the_rest() {
        let choices = choices(
            &on(&["home", "errands"]),
            &[
                label(r#"{"name":"home","item_order":1}"#),
                label(r#"{"name":"errands","item_order":2}"#),
            ],
        );

        assert_eq!(toggled(&choices, 0), ["errands"]);
    }

    #[test]
    fn the_move_picker_lists_every_project_with_its_sections_under_it() {
        let projects = vec![
            serde_json::from_str(r#"{"id":"p2","name":"Second","child_order":2}"#).expect("second"),
            serde_json::from_str(r#"{"id":"p1","name":"First","child_order":1}"#).expect("first"),
        ];
        let sections = vec![
            serde_json::from_str(r#"{"id":"s1","name":"Doing","project_id":"p1"}"#)
                .expect("section"),
        ];

        let prompt = Prompt::move_to("1", &projects, &sections);
        let (entries, _) = prompt.entries(&Views::new(&[])).expect("a picker");

        assert_eq!(entries, [" First", "   Doing", " Second"]);
        let Prompt::Move { choices, .. } = &prompt else {
            panic!("a move prompt");
        };
        assert_eq!(choices[1].to, Destination::Section("s1".to_string()));
        assert_eq!(choices[2].to, Destination::Project("p2".to_string()));
    }

    #[test]
    fn an_input_takes_a_line_and_takes_it_back_a_character_at_a_time() {
        let mut prompt = Prompt::due("1");
        let input = prompt.input_mut().expect("an input");

        assert!(input.is_blank());
        for character in "next mon".chars() {
            input.push(character);
        }
        input.backspace();

        assert_eq!(input.text(), "next mo");
        assert!(!input.is_blank());
        assert_eq!(prompt.line().as_deref(), Some(" next mo_"));
    }

    #[test]
    fn an_input_of_nothing_but_spaces_asks_for_nothing() {
        let mut input = Input::new();
        input.push(' ');

        assert!(input.is_blank());
    }

    #[test]
    fn the_confirm_names_the_task_it_would_delete() {
        let prompt = Prompt::delete("1", "  water the plants");

        assert_eq!(prompt.line().as_deref(), Some(" delete water the plants?"));
        assert!(prompt.entries(&Views::new(&[])).is_none());
    }
}
