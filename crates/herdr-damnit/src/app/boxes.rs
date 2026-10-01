use herdr_damnit_application::{JobKind, SyncKind, argv};
use herdr_damnit_domain::Oid;

use super::{After, App};
use crate::overlay::{LineBox, LinePurpose, Overlay};

const DATE_WORDS: &str = "today, tomorrow, YYYY-MM-DD or YYYY-MM-DDTHH:MM";

impl App {
    pub(super) fn open_commit_box(&mut self) -> After {
        let staged = self.stage.staged_count();
        if staged == 0 {
            self.message =
                "nothing staged; press <Space> on a row or A to stage everything.".to_string();
            return After::Stay;
        }
        let plural = if staged == 1 { "" } else { "s" };
        self.open_box(
            format!("commit {staged} staged change{plural}"),
            "<CR> commits, <Esc> cancels",
            LinePurpose::Commit,
        )
    }

    pub(super) fn open_date_box(&mut self, field: &str, purpose: fn(Oid) -> LinePurpose) -> After {
        let Some(task) = self.selected_task() else {
            return After::Stay;
        };
        let title = format!("{field} of {}", task.subject);
        let oid = task.oid.clone();
        self.open_box(title, DATE_WORDS, purpose(oid))
    }

    pub(super) fn open_new_box(&mut self) -> After {
        let path = self
            .selected_object()
            .map(|object| object.path.clone())
            .unwrap_or_default();
        self.open_box(
            format!("new task in {path}"),
            "<CR> adds, <Esc> cancels",
            LinePurpose::New(path),
        )
    }

    fn open_box(&mut self, title: String, hint: &str, purpose: LinePurpose) -> After {
        self.reopen(Overlay::Line(LineBox {
            title,
            hint: hint.to_string(),
            text: String::new(),
            purpose,
        }))
    }

    pub(super) fn take_line(&mut self, line: LineBox) -> After {
        let text = line.text.trim().to_string();
        let when = (!text.is_empty()).then_some(text.as_str());
        let (kind, argv) = match &line.purpose {
            LinePurpose::Commit if text.is_empty() => {
                return self.refuse_blank(line, "a commit needs a message.");
            }
            LinePurpose::New(_) if text.is_empty() => {
                return self.refuse_blank(line, "a new task needs a subject.");
            }
            LinePurpose::Commit => (JobKind::Exclusive(SyncKind::Commit), argv::commit(&text)),
            LinePurpose::New(path) => (JobKind::Write, argv::create(&text, path)),
            LinePurpose::Due(oid) => (JobKind::Write, argv::set_due(oid, when)),
            LinePurpose::Deadline(oid) => (JobKind::Write, argv::set_deadline(oid, when)),
        };
        if let Some(id) = self.submit(kind, argv) {
            self.awaiting = Some((id, line));
        }
        After::Stay
    }

    fn refuse_blank(&mut self, line: LineBox, said: &str) -> After {
        self.message = said.to_string();
        self.reopen(Overlay::Line(line))
    }
}
