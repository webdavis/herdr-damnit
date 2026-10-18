use std::collections::HashMap;

use herdr_damnit_domain::{Cursor, Date, Object, Oid, done_rows};

#[derive(PartialEq, Eq)]
enum DoneReads {
    NotYetAsked,
    KeptFresh,
}

pub struct DoneScreen {
    objects: Vec<Object>,
    completion_days: HashMap<Oid, Date>,
    cursor: Cursor,
    reads: DoneReads,
}

impl DoneScreen {
    pub(super) fn new() -> Self {
        Self {
            objects: Vec::new(),
            completion_days: HashMap::new(),
            cursor: Cursor::new(Vec::new()),
            reads: DoneReads::NotYetAsked,
        }
    }

    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    pub(super) fn cursor_mut(&mut self) -> &mut Cursor {
        &mut self.cursor
    }

    pub(super) fn kept_fresh(&self) -> bool {
        self.reads == DoneReads::KeptFresh
    }

    pub(super) fn start_keeping_fresh(&mut self) -> bool {
        std::mem::replace(&mut self.reads, DoneReads::KeptFresh) == DoneReads::NotYetAsked
    }

    pub(super) fn object(&self, oid: &Oid) -> Option<&Object> {
        self.objects.iter().find(|object| &object.oid == oid)
    }

    pub(super) fn take_objects(&mut self, objects: Vec<Object>) {
        self.objects = objects;
        self.redraw();
    }

    pub(super) fn take_completion_days(&mut self, days: HashMap<Oid, Date>) {
        self.completion_days = days;
        self.redraw();
    }

    fn redraw(&mut self) {
        self.cursor
            .replace(done_rows(&self.objects, &self.completion_days));
    }
}
