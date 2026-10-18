use herdr_damnit_application::{Completion, Handshake, Request, check_status, check_version};

use super::App;

impl App {
    pub(super) fn applied_as_handshake(&mut self, completion: &Completion) -> bool {
        match completion.request {
            Request::Version => {
                match check_version(&completion.finished.stdout) {
                    Handshake::Refuse(said) => self.refusal = Some(said),
                    Handshake::Ready { warning, .. } => {
                        if let (Some(warning), false) = (warning, self.newer_dam_warning_shown) {
                            self.message = warning;
                            self.newer_dam_warning_shown = true;
                        }
                        self.submit(Request::Status);
                    }
                }
                true
            }
            Request::Status if !self.handshake_accepted => {
                match check_status(&completion.finished.stdout) {
                    Err(said) => self.refusal = Some(said),
                    Ok(()) => {
                        if let Err(said) = self.read_status(&completion.finished.stdout) {
                            self.message = said;
                            return true;
                        }
                        self.refusal = None;
                        self.handshake_accepted = true;
                        let query = self.views.current().query.clone();
                        self.submit(Request::List(query));
                        self.show(self.screen);
                    }
                }
                true
            }
            _ => false,
        }
    }
}
