use herdr_damnit_application::{Side, argv};
use herdr_damnit_domain::{DAM_RESTORE, Kind, Object, Oid};

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
            ConfirmPurpose::Discard(oid) => self.write(argv::restore(&oid)),
            ConfirmPurpose::QuitMidJob => After::Quit,
        }
    }

    /// The one key gated on a dam version: a confirm followed by a refusal is the worst shape a
    /// destructive key can have, so without `restore` the key is simply not bound.
    pub(super) fn ask_discard(&mut self) -> After {
        if !self.version.is_some_and(|found| found >= DAM_RESTORE) {
            return After::Stay;
        }
        let Some(change) = self
            .selected_oid()
            .and_then(|oid| self.stage.unstaged.iter().find(|change| &change.oid == oid))
        else {
            return After::Stay;
        };
        let lost = match change.fields.is_empty() {
            true => "its working change".to_string(),
            false => change.fields.join(", "),
        };
        let question = format!(
            "discard {} {} ({lost})? ! discards it",
            change.oid.short(),
            change.subject
        );
        let oid = change.oid.clone();
        self.reopen(Overlay::Confirm(Confirm {
            question,
            key: '!',
            purpose: ConfirmPurpose::Discard(oid),
        }))
    }

    pub(super) fn quit(&mut self) -> After {
        let Some(sync) = self.jobs.exclusive() else {
            return After::Quit;
        };
        self.reopen(Overlay::Confirm(Confirm {
            question: format!(
                "a {} is running; q again quits and lets it finish",
                sync.verb()
            ),
            key: 'q',
            purpose: ConfirmPurpose::QuitMidJob,
        }))
    }

    pub(super) fn resolve(&mut self, side: Side) -> After {
        let conflicted = self.selected_oid().filter(|oid| {
            self.stage
                .conflicts
                .iter()
                .any(|conflict| &conflict.oid == *oid)
        });
        match conflicted.map(|oid| argv::resolve(oid, side)) {
            Some(write) => self.write(write),
            None => After::Stay,
        }
    }

    pub(super) fn edit_in_editor(&self) -> After {
        match self.selected_oid() {
            Some(oid) => After::Editor(argv::edit_in_editor(oid)),
            None => After::Stay,
        }
    }

    pub fn subject_of(&self, oid: &Oid) -> Option<&str> {
        self.object(oid).map(|object| object.subject.as_str())
    }

    pub(super) fn object(&self, oid: &Oid) -> Option<&Object> {
        self.objects
            .iter()
            .chain(&self.done_objects)
            .find(|object| &object.oid == oid)
    }

    pub(super) fn selected_object(&self) -> Option<&Object> {
        self.selected_oid().and_then(|oid| self.object(oid))
    }

    pub(super) fn selected_task(&self) -> Option<&Object> {
        self.selected_object()
            .filter(|object| object.kind == Kind::Task)
    }
}
