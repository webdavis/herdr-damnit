use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use herdr_damnit_application::{JobKind, SyncKind, argv};
use herdr_damnit_domain::{Cursor, Oid};

use super::{After, App, Screen};
use crate::overlay::LinePurpose;

impl App {
    pub fn key(&mut self, key: KeyEvent) -> After {
        self.message.clear();

        if self.refusal.is_some() {
            return After::Stay;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.jobs.cancel_current();
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
            KeyCode::Char(' ') => self.toggle_staged(),
            KeyCode::Char('A') => self.write(argv::stage_all()),
            KeyCode::Char('U') => self.write(argv::unstage_all()),
            KeyCode::Char('c') => self.open_commit_box(),
            KeyCode::Char('P') => self.sync(SyncKind::Push, argv::push()),
            KeyCode::Char('L') => self.sync(SyncKind::Pull, argv::pull()),
            KeyCode::Char('x') => self.complete(false),
            KeyCode::Char('X') => self.complete(true),
            KeyCode::Char('d') => self.ask_delete(),
            KeyCode::Char('p') => self.cycle_priority(),
            KeyCode::Char('s') => self.open_date_box("due", LinePurpose::Due),
            KeyCode::Char('D') => self.open_date_box("deadline", LinePurpose::Deadline),
            KeyCode::Char('l') => self.open_label_picker(),
            KeyCode::Char('m') => self.open_path_picker(),
            KeyCode::Char('a') => self.open_new_box(),
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

    fn move_cursor(&mut self, steps: isize) -> After {
        self.cursor_mut().move_by(steps);
        After::Stay
    }

    fn read_detail(&mut self) -> After {
        if let Some(oid) = self.selected_oid().cloned() {
            self.submit(JobKind::ReadShow(oid.clone()), argv::show(&oid));
        }
        After::Stay
    }

    fn toggle_staged(&mut self) -> After {
        match self.selected_oid().cloned() {
            Some(oid) if self.stage.is_staged(&oid) => self.write(argv::unstage(&oid)),
            Some(oid) => self.write(argv::stage(&oid)),
            None => After::Stay,
        }
    }

    pub(super) fn write(&mut self, argv: Vec<String>) -> After {
        self.submit(JobKind::Write, argv);
        After::Stay
    }

    fn sync(&mut self, kind: SyncKind, argv: Vec<String>) -> After {
        self.submit(JobKind::Exclusive(kind), argv);
        After::Stay
    }

    pub(super) fn cursor_mut(&mut self) -> &mut Cursor {
        match self.screen {
            Screen::Status => &mut self.status,
            Screen::Done => &mut self.done,
            Screen::List | Screen::Detail => &mut self.list,
        }
    }

    pub(super) fn selected_oid(&self) -> Option<&Oid> {
        match self.screen {
            Screen::List => self.list.selected_oid(),
            Screen::Status => self.status.selected_oid(),
            Screen::Done => self.done.selected_oid(),
            Screen::Detail => None,
        }
    }
}
