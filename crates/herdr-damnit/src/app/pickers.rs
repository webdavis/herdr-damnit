use herdr_damnit_application::argv;
use herdr_damnit_domain::Oid;

use super::{After, App};
use crate::overlay::{Overlay, Picker, PickerEntry};

impl App {
    pub(super) fn open_label_picker(&mut self) -> After {
        let Some(object) = self.selected_object() else {
            return After::Stay;
        };
        let carried = &object.labels;
        let entries = self
            .list
            .labels_with(carried)
            .into_iter()
            .map(|name| PickerEntry {
                text: name.clone(),
                marked: carried.contains(name),
            });
        let picker = picker("labels", entries.collect(), 0);
        let oid = object.oid.clone();
        self.reopen(Overlay::Label(oid, picker))
    }

    pub(super) fn take_label(&mut self, oid: Oid, mut picker: Picker) -> After {
        if let Some(entry) = picker.entries.get_mut(picker.selected) {
            let write = match entry.marked {
                true => argv::unlabel(&oid, &entry.text),
                false => argv::label(&oid, &entry.text),
            };
            entry.marked = !entry.marked;
            self.write(write);
        }
        self.reopen(Overlay::Label(oid, picker))
    }

    pub(super) fn open_path_picker(&mut self) -> After {
        let Some(object) = self.selected_object() else {
            return After::Stay;
        };
        let paths = self.list.paths();
        let own = object.path.trim_end_matches('/');
        let selected = paths.iter().position(|path| path == own).unwrap_or(0);
        let entries = paths.into_iter().map(|text| PickerEntry {
            text,
            marked: false,
        });
        let oid = object.oid.clone();
        self.reopen(Overlay::Path(
            oid,
            picker("move to", entries.collect(), selected),
        ))
    }

    pub(super) fn take_path(&mut self, oid: Oid, picker: Picker) -> After {
        match picker.entries.get(picker.selected) {
            Some(entry) => self.write(argv::move_to(&oid, &entry.text)),
            None => After::Stay,
        }
    }
}

fn picker(title: &str, entries: Vec<PickerEntry>, selected: usize) -> Picker {
    Picker {
        title: title.to_string(),
        entries,
        selected,
    }
}
