use herdr_damnit_application::Clock;
use herdr_damnit_domain::{Cursor, IconSet, Object, Oid, RowStyle, Stage, Views, list_rows};

pub struct ListScreen {
    views: Views,
    objects: Vec<Object>,
    cursor: Cursor,
    clock: Box<dyn Clock>,
}

impl ListScreen {
    pub(super) fn new(views: Views, clock: Box<dyn Clock>) -> Self {
        Self {
            views,
            objects: Vec::new(),
            cursor: Cursor::new(Vec::new()),
            clock,
        }
    }

    pub fn views(&self) -> &Views {
        &self.views
    }

    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    pub(super) fn cursor_mut(&mut self) -> &mut Cursor {
        &mut self.cursor
    }

    pub(super) fn take_views(&mut self, mut views: Views) {
        views.select_named(&self.views.current().name);
        self.views = views;
    }

    pub(super) fn select_view(&mut self, index: usize) -> bool {
        self.views.select(index)
    }

    pub(super) fn object(&self, oid: &Oid) -> Option<&Object> {
        self.objects.iter().find(|object| &object.oid == oid)
    }

    pub(super) fn take(&mut self, objects: Vec<Object>, stage: &Stage, icons: IconSet) {
        self.objects = objects;
        self.redraw(stage, icons);
    }

    pub(super) fn redraw(&mut self, stage: &Stage, icons: IconSet) {
        let style = RowStyle {
            icons,
            today: self.clock.today(),
        };
        self.cursor.replace(list_rows(&self.objects, stage, style));
    }

    pub(super) fn labels_with<'a>(&'a self, carried: &'a [String]) -> Vec<&'a String> {
        let mut names: Vec<&String> = self
            .objects
            .iter()
            .flat_map(|object| &object.labels)
            .chain(carried)
            .collect();
        names.sort();
        names.dedup();
        names
    }

    pub(super) fn paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self
            .objects
            .iter()
            .flat_map(|object| path_and_its_parents(&object.path))
            .collect();
        paths.sort();
        paths.dedup();
        paths
    }
}

fn path_and_its_parents(path: &str) -> Vec<String> {
    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    (1..=segments.len())
        .map(|depth| segments[..depth].join("/"))
        .collect()
}
