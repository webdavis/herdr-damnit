//! The three words the config file spells that herdr and the row marks take by name. Each is
//! its own serde enum, so an unrecognized value is a parse error naming the alternatives
//! rather than a raw error from `herdr` at action time.

use herdr_damnit_domain::IconSet;
use serde::Deserialize;

/// The pane placements `herdr plugin pane open --placement` accepts. An unrecognized value is a
/// config parse error naming these, rather than a raw error from `herdr` at action time.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Placement {
    Overlay,
    #[default]
    Split,
    Tab,
    Zoomed,
}

impl Placement {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Overlay => "overlay",
            Self::Split => "split",
            Self::Tab => "tab",
            Self::Zoomed => "zoomed",
        }
    }
}

/// The side of the calling pane this pane takes. herdr's own open splits rightward or downward
/// only, so those are the only sides there are.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    #[default]
    Right,
    Down,
}

impl Side {
    /// The direction both `herdr plugin pane open --direction` and `herdr pane resize
    /// --direction` take for this side.
    pub fn split_direction(self) -> &'static str {
        match self {
            Self::Right => "right",
            Self::Down => "down",
        }
    }
}

/// The mark sets, as words in the file. `IconSet` is a domain type with no serde derive, so the
/// config declares its own and maps it, which is also what makes an unknown word name the two
/// that resolve.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Icons {
    #[default]
    NerdFont,
    Ascii,
}

impl From<Icons> for IconSet {
    fn from(icons: Icons) -> Self {
        match icons {
            Icons::NerdFont => IconSet::NerdFont,
            Icons::Ascii => IconSet::Ascii,
        }
    }
}
