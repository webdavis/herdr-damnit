use herdr_damnit_application::{HandOff, JobKind, hand_off};

use super::{After, App};
use crate::overlay::{NoteBox, Overlay};

impl App {
    pub(super) fn open_note_box(&mut self) -> After {
        match self.selected_object().map(|object| object.oid.clone()) {
            Some(oid) => self.reopen(Overlay::Note(NoteBox {
                oid,
                text: String::new(),
            })),
            None => After::Stay,
        }
    }

    pub(super) fn send_note(&mut self, note: NoteBox) -> After {
        let Some(object) = self.object(&note.oid).cloned() else {
            return After::Stay;
        };
        let label = self.config.handoff_label.clone();
        match hand_off(&*self.herdr, &self.here, &object, note.text.trim(), &label) {
            HandOff::Refused(said) => self.message = said,
            HandOff::Sent { agent, label_write } => {
                self.message = format!("sent to {}", agent.name);
                if let Some(write) = label_write
                    && let Some(id) = self.submit(JobKind::Write, write)
                {
                    self.labelled = Some((id, agent.name));
                }
            }
        }
        After::Stay
    }
}
