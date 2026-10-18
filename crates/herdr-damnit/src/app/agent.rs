use herdr_damnit_application::{HandOff, Request, agent_pane, hand_off};

use super::{After, App, Pending};
use crate::overlay::{NoteBox, Overlay};

impl App {
    pub(super) fn open_note_box(&mut self) -> After {
        let Some(oid) = self.selected_object().map(|object| object.oid.clone()) else {
            return After::Stay;
        };
        match agent_pane(&*self.agents, &self.here) {
            Ok(_) => self.reopen(Overlay::Note(NoteBox {
                oid,
                text: String::new(),
            })),
            Err(said) => {
                self.message = said;
                After::Stay
            }
        }
    }

    pub(super) fn send_note(&mut self, note: NoteBox) -> After {
        let Some(object) = self.object(&note.oid).cloned() else {
            return After::Stay;
        };
        let label = self.config.handoff_label.clone();
        match hand_off(&*self.agents, &self.here, &object, note.text.trim(), &label) {
            HandOff::Refused(said) => {
                self.message = said;
                return self.reopen(Overlay::Note(note));
            }
            HandOff::Sent { agent, label_write } => {
                self.message = format!("sent to {}", agent.name);
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
