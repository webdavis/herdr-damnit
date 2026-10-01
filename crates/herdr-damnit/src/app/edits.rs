use herdr_damnit_application::argv;
use herdr_damnit_domain::{Kind, Object};

use super::{After, App};
use crate::overlay::{Confirm, ConfirmPurpose, Overlay};

impl App {
    pub(super) fn complete(&mut self, force: bool) -> After {
        match self
            .selected_task()
            .map(|task| argv::done(&task.oid, force))
        {
            Some(write) => self.write(write),
            None => After::Stay,
        }
    }

    pub(super) fn cycle_priority(&mut self) -> After {
        let write = self
            .selected_task()
            .map(|task| argv::set_priority(&task.oid, task.priority().next()));
        match write {
            Some(write) => self.write(write),
            None => After::Stay,
        }
    }

    pub(super) fn ask_delete(&mut self) -> After {
        let Some(oid) = self.selected_oid().cloned() else {
            return After::Stay;
        };
        let subject = self.subject_of(&oid).unwrap_or_default();
        self.reopen(Overlay::Confirm(Confirm {
            question: format!("remove {} {subject}? d removes it", oid.short()),
            key: 'd',
            purpose: ConfirmPurpose::Delete(oid),
        }))
    }

    pub(super) fn confirmed(&mut self, purpose: ConfirmPurpose) -> After {
        match purpose {
            ConfirmPurpose::Delete(oid) => self.write(argv::remove(&oid)),
        }
    }

    pub(super) fn selected_object(&self) -> Option<&Object> {
        self.selected_oid().and_then(|oid| self.object(oid))
    }

    pub(super) fn selected_task(&self) -> Option<&Object> {
        self.selected_object()
            .filter(|object| object.kind == Kind::Task)
    }
}
