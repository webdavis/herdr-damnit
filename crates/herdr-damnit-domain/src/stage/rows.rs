use crate::{IconSet, Mark, Oid, Row, Segment, Slot};

use super::{Change, Notice, Op, Stage};

pub fn status_rows(stage: &Stage, icons: IconSet) -> Vec<Row> {
    let mut rows = Vec::new();
    section(&mut rows, "Staged", &stage.staged, Mark::Staged, icons);
    section(&mut rows, "Working", &stage.unstaged, Mark::Working, icons);
    if stage.unpushed.iter().any(|remote| remote.commits > 0) {
        rows.push(Row::Heading("Unpushed".to_string()));
        for remote in stage.unpushed.iter().filter(|remote| remote.commits > 0) {
            let text = format!(
                "{}  {} commit{}",
                remote.remote,
                remote.commits,
                if remote.commits == 1 { "" } else { "s" }
            );
            rows.push(marked(None, Mark::Unpushed, text, icons));
        }
    }
    if !stage.conflicts.is_empty() || !stage.notices.is_empty() {
        rows.push(Row::Heading("Notices".to_string()));
        for conflict in &stage.conflicts {
            let text = format!(
                "{}  {}  ours: \"{}\"  theirs: \"{}\"",
                conflict.oid.short(),
                conflict.remote,
                newlines_as_spaces(&conflict.ours),
                newlines_as_spaces(&conflict.theirs)
            );
            rows.push(marked(Some(&conflict.oid), Mark::Conflict, text, icons));
        }
        rows.extend(stage.notices.iter().map(|notice| notice.row(icons)));
    }
    if rows.is_empty() {
        rows.push(plain("nothing staged, nothing changed".to_string()));
    }
    rows
}

fn section(rows: &mut Vec<Row>, heading: &str, changes: &[Change], mark: Mark, icons: IconSet) {
    if changes.is_empty() {
        return;
    }
    rows.push(Row::Heading(heading.to_string()));
    rows.extend(
        changes
            .iter()
            .map(|change| marked(Some(&change.oid), mark, change.line(), icons)),
    );
}

fn marked(oid: Option<&Oid>, mark: Mark, text: String, icons: IconSet) -> Row {
    Row::Line {
        oid: oid.cloned(),
        segments: vec![
            Segment {
                text: "  ".to_string(),
                slot: Slot::Text,
            },
            Segment {
                text: format!("{} ", mark.glyph(icons)),
                slot: mark.slot(),
            },
            Segment {
                text,
                slot: Slot::Text,
            },
        ],
    }
}

fn plain(text: String) -> Row {
    Row::Line {
        oid: None,
        segments: vec![Segment {
            text,
            slot: Slot::Text,
        }],
    }
}

fn newlines_as_spaces(value: &str) -> String {
    value.replace(['\n', '\r'], " ")
}

impl Change {
    fn line(&self) -> String {
        let word = match self.op {
            Op::Create => "new     ",
            Op::Update => "changed ",
            Op::Delete => "removed ",
        };
        let fields = if self.fields.is_empty() {
            String::new()
        } else {
            format!("  ({})", self.fields.join(", "))
        };
        format!("{word} {}  {}{fields}", self.oid.short(), self.subject)
    }
}

impl Notice {
    fn row(&self, icons: IconSet) -> Row {
        match &self.oid {
            Some(oid) => marked(
                Some(oid),
                Mark::Notice,
                format!("{}  {}", oid.short(), self.message),
                icons,
            ),
            None => plain(format!("  {}", self.message)),
        }
    }
}
