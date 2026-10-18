use ratatui::Frame;
use ratatui::layout::Rect;

use herdr_damnit_domain::{Object, Oid, Stage, counted};

use super::{detail, refusal, rows};
use crate::app::{DoneScreen, ListScreen, Overlay, StatusScreen};
use crate::legend::Mode;

pub enum Body<'a> {
    Refusal(&'a str),
    Detail(&'a Object, &'a dyn Fn(&Oid) -> Option<&'a str>),
    List(&'a ListScreen, &'a Stage),
    Status(&'a StatusScreen),
    Done(&'a DoneScreen),
}

impl Body<'_> {
    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect) {
        match self {
            Self::Refusal(said) => refusal::draw(frame, area, said),
            Self::Detail(object, subject_of) => detail::draw(frame, area, object, *subject_of),
            Self::List(list, _) => rows::draw(frame, area, list.cursor(), "nothing in this view"),
            Self::Status(status) => rows::draw(frame, area, status.cursor(), ""),
            Self::Done(done) => rows::draw(frame, area, done.cursor(), "nothing completed yet"),
        }
    }

    pub fn counts(&self) -> String {
        match self {
            Self::Refusal(_) | Self::Detail(..) => String::new(),
            Self::List(list, stage) => counted(&[
                (list.cursor().object_count() as u64, "open"),
                (stage.staged_count() as u64, "staged"),
                (stage.unpushed_commits(), "unpushed"),
            ]),
            Self::Status(status) => status.stage().summary(),
            Self::Done(done) => format!("{} done", done.cursor().object_count()),
        }
    }

    pub fn cursor(&self) -> Option<usize> {
        match self {
            Self::Refusal(_) | Self::Detail(..) => None,
            Self::List(list, _) => Some(list.cursor().selected()),
            Self::Status(status) => status
                .cursor()
                .selected_oid()
                .map(|_| status.cursor().selected()),
            Self::Done(done) => Some(done.cursor().selected()),
        }
    }

    pub fn mode(&self, overlay: Option<&Overlay>) -> Mode {
        match (self, overlay) {
            (Self::Refusal(_), _) => Mode::Refusal,
            (_, Some(Overlay::View(_) | Overlay::Label(..) | Overlay::Path(..))) => Mode::Picker,
            (_, Some(Overlay::Line(_))) => Mode::LineBox,
            (_, Some(Overlay::Note(_))) => Mode::NoteBox,
            (_, Some(Overlay::Confirm(confirm))) => Mode::Confirm(confirm.key),
            (Self::Detail(..), None) => Mode::Detail,
            (Self::List(..), None) => Mode::List,
            (Self::Status(_), None) => Mode::Status,
            (Self::Done(_), None) => Mode::Done,
        }
    }
}
