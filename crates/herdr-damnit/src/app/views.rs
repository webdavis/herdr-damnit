use std::time::{Duration, Instant};

use herdr_damnit_application::{Phase, Request};

use super::{After, App, Screen};
use crate::overlay::{Overlay, Picker, PickerEntry};

impl App {
    pub(super) fn show_numbered_view(&mut self, number: usize) -> After {
        match self.list.views().name_of_number(number) {
            Some(_) => self.show_view(number - 1),
            None => {
                self.message = format!("the config has {} views.", self.list.views().count());
                After::Stay
            }
        }
    }

    pub(super) fn show_view(&mut self, index: usize) -> After {
        if self.list.select_view(index) {
            self.read_list();
        }
        self.show(Screen::List)
    }

    pub(super) fn open_view_picker(&mut self) -> After {
        let entries = self.list.views().names().map(|name| PickerEntry {
            text: name.to_string(),
            marked: false,
        });
        let selected = self.list.views().showing();
        self.reopen(Overlay::View(Picker {
            title: "view".to_string(),
            entries: entries.collect(),
            selected,
        }))
    }

    pub(crate) fn reread(&mut self) -> After {
        self.rereads.forget();
        match self.phase {
            Phase::Ready => {
                self.read_list();
                self.submit(Request::Status);
                if self.done.opened() {
                    self.read_done();
                }
            }
            _ if self.work.in_flight() == 0 => crate::open::start(self),
            _ => {}
        }
        After::Stay
    }

    fn read_list(&mut self) {
        if self.phase == Phase::Ready {
            let query = self.list.views().current().query.clone();
            self.submit(Request::List(query));
        }
    }

    fn read_done(&mut self) {
        self.submit(Request::Done);
        self.submit(Request::Log);
    }

    pub(super) fn take_view_request(&mut self) {
        let Some(name) = self.rereads.requested_view() else {
            return;
        };
        if name == crate::pane::STATUS_REQUEST {
            self.show(Screen::Status);
            return;
        }
        let index = self.list.views().names().position(|found| found == name);
        if let Some(index) = index {
            self.show_view(index);
        }
    }

    pub(super) fn reread_on_the_interval(&mut self, now: Instant) {
        let interval = Duration::from_secs(self.config.refresh_seconds());
        let held_back = interval.is_zero()
            || self.showing.is_layered()
            || self.showing.screen() != Screen::List;
        if self.rereads.due(now, interval, held_back) {
            self.reread();
            self.rereads.restart(now);
        }
    }

    pub(super) fn show(&mut self, screen: Screen) -> After {
        self.showing.show(screen);
        if screen == Screen::Done && self.phase == Phase::Ready && self.done.open_the_first_time() {
            self.read_done();
        }
        After::Stay
    }
}
