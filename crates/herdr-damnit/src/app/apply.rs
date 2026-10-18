use herdr_damnit_adapters::wire;
use herdr_damnit_application::{Completion, Phase, Request, SyncKind};
use herdr_damnit_domain::{Failure, Stage, classify, message};

use super::{App, Pending};
use crate::overlay::{LineBox, LinePurpose, Overlay};

impl App {
    pub(super) fn apply(&mut self, completion: Completion) {
        let pending = self.work.settle(&completion.id);
        if completion.finished.code != Some(0) {
            let failure = classify(
                completion.finished.code,
                wire::error_document(&completion.finished.stderr),
                &completion.finished.stderr,
            );
            self.message = message(&failure);
            match pending {
                Some(Pending::Label { agent }) => {
                    self.message = format!("sent to {agent}, label refused: {}.", self.message);
                }
                Some(Pending::Box(line)) => self.bring_the_box_back(line, &failure),
                None => {}
            }
            return;
        }
        if let Some(Pending::Box(LineBox {
            purpose: LinePurpose::New(_),
            ..
        })) = pending
        {
            self.name_what_dam_made(&completion.finished.stdout);
        }
        if let Err(said) = self.read_into_model(&completion) {
            self.message = said;
            return;
        }
        if completion.request.rereads_after() {
            self.reread();
        }
    }

    fn read_into_model(&mut self, completion: &Completion) -> Result<(), String> {
        let unreadable = || message(&Failure::Unreadable);
        match (&self.phase, &completion.request) {
            (_, Request::Version) => self.take_version(&completion.finished.stdout),
            (Phase::Ready, Request::Status) => self.read_status(&completion.finished.stdout)?,
            (_, Request::Status) => self.take_first_status(&completion.finished.stdout),
            (_, Request::List(_)) => {
                let objects =
                    wire::objects(&completion.finished.stdout).map_err(|_| unreadable())?;
                self.list
                    .take(objects, self.status.stage(), self.config.icons());
            }
            (_, Request::Done) => {
                self.done.take_objects(
                    wire::objects(&completion.finished.stdout).map_err(|_| unreadable())?,
                );
            }
            (_, Request::Show(_)) => {
                self.showing.open_detail(
                    wire::object(&completion.finished.stdout).map_err(|_| unreadable())?,
                );
            }
            (_, Request::Log) => {
                self.done.take_completion_days(
                    wire::completions(&completion.finished.stdout).map_err(|_| unreadable())?,
                );
            }
            (_, Request::Sync(SyncKind::Push, _)) => {
                self.message =
                    wire::push_summary(&completion.finished.stdout).map_err(|_| unreadable())?;
            }
            (_, Request::Sync(SyncKind::Pull, _)) => {
                self.message =
                    wire::pull_summary(&completion.finished.stdout).map_err(|_| unreadable())?;
            }
            _ => {}
        }
        Ok(())
    }

    fn bring_the_box_back(&mut self, line: LineBox, failure: &Failure) {
        if !matches!(failure, Failure::Store(_)) && self.showing.overlay().is_none() {
            self.showing.open(Overlay::Line(line));
        }
    }

    fn name_what_dam_made(&mut self, stdout: &str) {
        if let Ok(made) = wire::object(stdout) {
            self.message = format!("made {}", made.oid.short());
        }
    }

    fn read_status(&mut self, stdout: &str) -> Result<(), String> {
        let stage = wire::stage(stdout).map_err(|_| message(&Failure::Unreadable))?;
        self.take_stage(stage);
        Ok(())
    }

    pub(super) fn take_stage(&mut self, stage: Stage) {
        self.status.take(stage, self.config.icons());
        self.list.redraw(self.status.stage(), self.config.icons());
    }
}
