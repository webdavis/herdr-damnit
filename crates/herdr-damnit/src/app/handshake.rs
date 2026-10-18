use herdr_damnit_adapters::wire;
use herdr_damnit_application::{Handshake, Phase, Request, check_version};

use super::App;

impl App {
    pub(super) fn take_version(&mut self, stdout: &str) {
        match check_version(stdout) {
            Handshake::Refuse(said) => self.phase = Phase::Refused(said),
            Handshake::Ready { warning, .. } => {
                if let Some(warning) = warning {
                    self.message = warning;
                }
                self.submit(Request::Status);
            }
        }
    }

    pub(super) fn take_first_status(&mut self, stdout: &str) {
        match wire::stage(stdout) {
            Err(error) => self.phase = Phase::Refused(status_refusal(&error)),
            Ok(stage) => {
                self.take_stage(stage);
                self.phase = Phase::Ready;
                let query = self.views.current().query.clone();
                self.submit(Request::List(query));
                self.show(self.screen);
            }
        }
    }
}

pub(crate) fn status_refusal(error: &str) -> String {
    format!("dam status is missing what this pane reads: {error}")
}
