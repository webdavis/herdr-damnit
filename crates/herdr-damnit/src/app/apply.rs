use herdr_damnit_adapters::wire;
use herdr_damnit_application::{Completion, JobKind, SyncKind, argv};
use herdr_damnit_domain::{
    DONE_QUERY, Failure, ObjectRow, Row, RowStyle, StatusRow, classify, message, rows,
};

use super::{App, Screen};
use crate::overlay::{LineBox, LinePurpose, Overlay};

impl App {
    pub(super) fn apply(&mut self, completion: Completion) {
        let awaited = self
            .awaiting
            .take_if(|(id, _)| *id == completion.id)
            .map(|(_, line)| line);
        if completion.finished.code != Some(0) {
            let failure = classify(
                completion.finished.code,
                wire::error_document(&completion.finished.stderr),
                &completion.finished.stderr,
            );
            self.message = message(&failure);
            self.bring_the_box_back(awaited, &failure);
            return;
        }
        if let Some(LineBox {
            purpose: LinePurpose::New(_),
            ..
        }) = awaited
        {
            self.name_what_dam_made(&completion.finished.stdout);
        }
        if self.applied_as_handshake(&completion) {
            return;
        }
        if let Err(said) = self.read_into_model(&completion) {
            self.message = said;
            return;
        }
        for kind in completion.follow_up {
            let argv = self.follow_up_argv(&kind);
            self.submit(kind, argv);
        }
    }

    fn read_into_model(&mut self, completion: &Completion) -> Result<(), String> {
        let unreadable = || message(&Failure::Unreadable);
        match &completion.kind {
            JobKind::ReadStatus => self.read_status(&completion.finished.stdout)?,
            JobKind::ReadList => {
                self.objects =
                    wire::objects(&completion.finished.stdout).map_err(|_| unreadable())?;
                self.redraw_list();
            }
            JobKind::ReadDone => {
                self.done_objects =
                    wire::objects(&completion.finished.stdout).map_err(|_| unreadable())?;
                self.done.replace(self.done_rows());
            }
            JobKind::ReadShow(_) => {
                self.detail =
                    Some(wire::object(&completion.finished.stdout).map_err(|_| unreadable())?);
                if self.screen != Screen::Detail {
                    self.under_detail = self.screen;
                    self.screen = Screen::Detail;
                }
            }
            JobKind::ReadLog => {
                self.completion_days =
                    wire::completions(&completion.finished.stdout).map_err(|_| unreadable())?;
                self.done.replace(self.done_rows());
            }
            JobKind::Exclusive(SyncKind::Push) => {
                self.message =
                    wire::push_summary(&completion.finished.stdout).map_err(|_| unreadable())?;
            }
            JobKind::Exclusive(SyncKind::Pull) => {
                self.message =
                    wire::pull_summary(&completion.finished.stdout).map_err(|_| unreadable())?;
            }
            _ => {}
        }
        Ok(())
    }

    /// A held store is the one failure the box stays shut for, so `R` reaches the retry.
    fn bring_the_box_back(&mut self, awaited: Option<LineBox>, failure: &Failure) {
        if let Some(line) = awaited
            && !matches!(failure, Failure::Store(_))
            && self.overlay.is_none()
        {
            self.overlay = Some(Overlay::Line(line));
        }
    }

    fn name_what_dam_made(&mut self, stdout: &str) {
        if let Ok(made) = wire::object(stdout) {
            self.message = format!("made {}", made.oid.short());
        }
    }

    pub(super) fn read_status(&mut self, stdout: &str) -> Result<(), String> {
        self.stage = wire::stage(stdout).map_err(|_| message(&Failure::Unreadable))?;
        self.status.replace(cursor_targets(&self.stage.rows()));
        self.redraw_list();
        Ok(())
    }

    pub(super) fn redraw_list(&mut self) {
        let style = RowStyle {
            icons: self.config.icons(),
            today: self.today,
        };
        self.list.replace(rows(&self.objects, &self.stage, style));
    }

    fn follow_up_argv(&self, kind: &JobKind) -> Vec<String> {
        match kind {
            JobKind::ReadList => argv::list(&self.views.current().query),
            JobKind::ReadDone => argv::list(DONE_QUERY),
            JobKind::ReadStatus => argv::status(),
            JobKind::ReadShow(oid) => argv::show(oid),
            JobKind::ReadLog => argv::log(),
            JobKind::Version => argv::version(),
            JobKind::Handshake => argv::status(),
            JobKind::Write => Vec::new(),
            JobKind::Exclusive(SyncKind::Commit) => argv::commit(""),
            JobKind::Exclusive(SyncKind::Push) => argv::push(),
            JobKind::Exclusive(SyncKind::Pull) => argv::pull(),
        }
    }
}

fn cursor_targets(status: &[StatusRow]) -> Vec<Row> {
    let target = |row: &StatusRow| match row {
        StatusRow::Change { oid, text, .. } => Row::Object(ObjectRow {
            oid: oid.clone(),
            subject: text.clone(),
            segments: Vec::new(),
        }),
        StatusRow::Heading(text) | StatusRow::Line { text, .. } => Row::Heading(text.clone()),
    };
    status.iter().map(target).collect()
}
