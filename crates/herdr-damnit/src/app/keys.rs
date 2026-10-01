use crossterm::event::{KeyCode, KeyEvent};
use herdr_damnit_application::JobKind;

use super::{After, App, Screen};

impl App {
    pub fn key(&mut self, key: KeyEvent) -> After {
        if self.screen == Screen::Detail {
            return self.detail_key(key);
        }
        match key.code {
            KeyCode::Tab => self.show(self.screen.next()),
            KeyCode::BackTab => self.show(self.screen.previous()),
            KeyCode::Enter => self.read_detail(),
            _ => After::Stay,
        }
    }

    fn detail_key(&mut self, key: KeyEvent) -> After {
        if key.code == KeyCode::Esc {
            self.screen = self.under_detail;
            self.detail = None;
        }
        After::Stay
    }

    fn read_detail(&mut self) -> After {
        if let Some(oid) = self.selected_oid().cloned() {
            let argv = herdr_damnit_application::argv::show(&oid);
            self.submit(JobKind::ReadShow(oid), argv);
        }
        After::Stay
    }

    fn selected_oid(&self) -> Option<&herdr_damnit_domain::Oid> {
        match self.screen {
            Screen::List => self.list.selected_oid(),
            Screen::Status | Screen::Done | Screen::Detail => None,
        }
    }
}
