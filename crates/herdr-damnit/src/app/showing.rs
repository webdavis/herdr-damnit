use herdr_damnit_domain::Object;

use super::Screen;
use crate::overlay::Overlay;

pub struct Showing {
    screen: Screen,
    detail: Option<Object>,
    overlay: Option<Overlay>,
}

impl Showing {
    pub(super) fn new() -> Self {
        Self {
            screen: Screen::List,
            detail: None,
            overlay: None,
        }
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }

    pub fn detail(&self) -> Option<&Object> {
        self.detail.as_ref()
    }

    pub fn overlay(&self) -> Option<&Overlay> {
        self.overlay.as_ref()
    }

    pub(super) fn label<'a>(&self, view: &'a str) -> &'a str {
        match (&self.detail, self.screen.label()) {
            (Some(_), _) => "detail",
            (None, Some(label)) => label,
            (None, None) => view,
        }
    }

    pub(super) fn is_layered(&self) -> bool {
        self.overlay.is_some() || self.detail.is_some()
    }

    pub(super) fn show(&mut self, screen: Screen) {
        self.screen = screen;
        self.detail = None;
    }

    pub(super) fn open_detail(&mut self, object: Object) {
        self.detail = Some(object);
    }

    pub(super) fn close_detail(&mut self) {
        self.detail = None;
    }

    pub(super) fn open(&mut self, overlay: Overlay) {
        self.overlay = Some(overlay);
    }

    pub(super) fn take_overlay(&mut self) -> Option<Overlay> {
        self.overlay.take()
    }
}
