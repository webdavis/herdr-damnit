use herdr_damnit_adapters::wire;
use herdr_damnit_application::{
    Completion, Handshake, JobKind, SyncKind, argv, check_status, check_version,
};
use herdr_damnit_domain::{
    DONE_QUERY, Failure, ObjectRow, Row, RowStyle, StatusRow, classify, message, rows,
};

use super::{App, Screen};

impl App {
    pub(super) fn apply(&mut self, completion: Completion) {
        if completion.finished.code != Some(0) {
            self.message = message(&classify(
                completion.finished.code,
                wire::error_document(&completion.finished.stderr),
                &completion.finished.stderr,
            ));
            return;
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

    fn applied_as_handshake(&mut self, completion: &Completion) -> bool {
        match completion.kind {
            JobKind::Version => {
                match check_version(&completion.finished.stdout) {
                    Handshake::Refuse(said) => self.refusal = Some(said),
                    Handshake::Ready { version, warning } => {
                        self.version = Some(version);
                        if let (Some(warning), false) = (warning, self.newer_dam_warning_shown) {
                            self.message = warning;
                            self.newer_dam_warning_shown = true;
                        }
                        self.submit(JobKind::Handshake, argv::status());
                    }
                }
                true
            }
            JobKind::Handshake => {
                match check_status(&completion.finished.stdout) {
                    Err(said) => self.refusal = Some(said),
                    Ok(()) => {
                        if let Err(said) = self.read_status(&completion.finished.stdout) {
                            self.message = said;
                            return true;
                        }
                        let query = self.views.current().query.clone();
                        self.submit(JobKind::ReadList, argv::list(&query));
                        self.handshake_accepted = true;
                        self.show(self.screen);
                    }
                }
                true
            }
            _ => false,
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

    fn read_status(&mut self, stdout: &str) -> Result<(), String> {
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
