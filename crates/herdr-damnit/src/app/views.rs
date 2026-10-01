use std::time::{Duration, Instant};

use herdr_damnit_adapters::state;
use herdr_damnit_application::{JobKind, argv};
use herdr_damnit_domain::DONE_QUERY;

use super::{After, App, Screen};
use crate::overlay::{Overlay, Picker, PickerEntry};

impl App {
    pub(super) fn show_numbered_view(&mut self, number: usize) -> After {
        match self.views.name_of_number(number) {
            Some(_) => self.show_view(number - 1),
            None => {
                self.message = format!("the config has {} views.", self.views.len());
                After::Stay
            }
        }
    }

    pub(super) fn show_view(&mut self, index: usize) -> After {
        if self.views.select(index) {
            self.read_list();
        }
        self.show(Screen::List)
    }

    pub(super) fn open_view_picker(&mut self) -> After {
        let entries = self.views.names().map(|name| PickerEntry {
            text: name.to_string(),
            marked: false,
        });
        self.overlay = Some(Overlay::View(Picker {
            title: "view".to_string(),
            entries: entries.collect(),
            selected: self.views.showing(),
        }));
        After::Stay
    }

    pub(crate) fn reread(&mut self) -> After {
        self.last_reread = None;
        if !self.handshake_accepted {
            crate::open::start(self);
            return After::Stay;
        }
        self.read_list();
        self.submit(JobKind::ReadStatus, argv::status());
        After::Stay
    }

    fn read_list(&mut self) {
        if self.handshake_accepted {
            let query = self.views.current().query.clone();
            self.submit(JobKind::ReadList, argv::list(&query));
        }
    }

    pub(super) fn take_view_request(&mut self) {
        let Some(name) = self
            .view_request
            .as_deref()
            .and_then(state::take_requested_view)
        else {
            return;
        };
        let index = self.views.names().position(|found| found == name);
        if let Some(index) = index {
            self.show_view(index);
        }
    }

    pub(super) fn reread_on_the_interval(&mut self, now: Instant) {
        let Some(since) = self.last_reread else {
            self.last_reread = Some(now);
            return;
        };
        let interval = Duration::from_secs(self.config.refresh_seconds());
        let held_back = self.overlay.is_some() || self.screen != Screen::List;
        if now.saturating_duration_since(since) >= interval && !held_back {
            self.reread();
            self.last_reread = Some(now);
        }
    }

    pub(super) fn show(&mut self, screen: Screen) -> After {
        self.screen = screen;
        if screen == Screen::Done && self.handshake_accepted {
            if !self.done_list_requested {
                self.done_list_requested = true;
                self.submit(JobKind::ReadDone, argv::list(DONE_QUERY));
            }
            if !self.done_log_requested {
                self.done_log_requested = true;
                self.submit(JobKind::ReadLog, argv::log());
            }
        }
        After::Stay
    }
}
