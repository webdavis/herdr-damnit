use crossterm::event::{KeyCode, KeyEvent};
use herdr_damnit_application::JobKind;
use herdr_damnit_domain::{Cursor, Oid};

use super::{After, App, Screen};
use crate::overlay::Overlay;

impl App {
    pub fn key(&mut self, key: KeyEvent) -> After {
        if self.refusal.is_some() {
            return After::Stay;
        }
        if let Some(overlay) = self.overlay.take() {
            return self.overlay_key(overlay, key);
        }
        if self.screen == Screen::Detail {
            return self.detail_key(key);
        }
        match key.code {
            KeyCode::Tab => self.show(self.screen.next()),
            KeyCode::BackTab => self.show(self.screen.previous()),
            KeyCode::Char('j') | KeyCode::Down => self.move_cursor(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_cursor(-1),
            KeyCode::Char(digit @ '1'..='9') => {
                self.show_numbered_view(digit as usize - '0' as usize)
            }
            KeyCode::Char('v') if matches!(self.screen, Screen::List | Screen::Done) => {
                self.open_view_picker()
            }
            KeyCode::Char('R' | 'r') => self.reread(),
            KeyCode::Enter => self.read_detail(),
            _ => After::Stay,
        }
    }

    fn overlay_key(&mut self, overlay: Overlay, key: KeyEvent) -> After {
        let Overlay::View(mut picker) = overlay;
        match key.code {
            KeyCode::Esc => After::Stay,
            KeyCode::Enter => self.show_view(picker.selected),
            code => {
                match code {
                    KeyCode::Char('j') | KeyCode::Down => picker.move_by(1),
                    KeyCode::Char('k') | KeyCode::Up => picker.move_by(-1),
                    _ => {}
                }
                self.overlay = Some(Overlay::View(picker));
                After::Stay
            }
        }
    }

    fn detail_key(&mut self, key: KeyEvent) -> After {
        if key.code == KeyCode::Esc {
            self.screen = self.under_detail;
            self.detail = None;
        }
        After::Stay
    }

    fn move_cursor(&mut self, steps: isize) -> After {
        self.cursor_mut().move_by(steps);
        After::Stay
    }

    fn read_detail(&mut self) -> After {
        if let Some(oid) = self.selected_oid().cloned() {
            let argv = herdr_damnit_application::argv::show(&oid);
            self.submit(JobKind::ReadShow(oid), argv);
        }
        After::Stay
    }

    pub(super) fn cursor_mut(&mut self) -> &mut Cursor {
        match self.screen {
            Screen::Status => &mut self.status,
            Screen::Done => &mut self.done,
            Screen::List | Screen::Detail => &mut self.list,
        }
    }

    fn selected_oid(&self) -> Option<&Oid> {
        match self.screen {
            Screen::List => self.list.selected_oid(),
            Screen::Status => self.status.selected_oid(),
            Screen::Done => self.done.selected_oid(),
            Screen::Detail => None,
        }
    }
}
