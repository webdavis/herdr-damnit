use herdr_damnit_application::{HandOff, Request};

use super::{After, App, Message, Pending};
use crate::overlay::{NoteBox, Overlay};

impl App {
    pub(super) fn open_note_box(&mut self) -> After {
        let Some(oid) = self.selected_object().map(|object| object.oid.clone()) else {
            return After::Stay;
        };
        match self.agents.pane() {
            Ok(_) => self.reopen(Overlay::Note(NoteBox {
                oid,
                text: String::new(),
            })),
            Err(said) => {
                self.message = Message::said(said);
                After::Stay
            }
        }
    }

    pub(super) fn send_note(&mut self, note: NoteBox) -> After {
        let Some(object) = self.object(&note.oid).cloned() else {
            return After::Stay;
        };
        let label = self.config.handoff_label.clone();
        match self.agents.hand_off(&object, note.text.trim(), &label) {
            HandOff::Refused(said) => {
                self.message = Message::said(said);
                return self.reopen(Overlay::Note(note));
            }
            HandOff::Sent { agent, label_write } => {
                self.message = Message::sent_to(&agent.name);
                if let Some(write) = label_write
                    && let Some(id) = self.submit(Request::Write(write))
                {
                    self.work.pend(id, Pending::Label { agent: agent.name });
                }
            }
        }
        After::Stay
    }
}
