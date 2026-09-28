use herdr_damnit_domain::IconSet;
use serde::Deserialize;

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

    pub fn attaches_to_a_pane_rather_than_the_workspace(self) -> bool {
        matches!(self, Self::Split | Self::Zoomed)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    #[default]
    Right,
    Down,
}

impl Side {
    pub fn split_direction(self) -> &'static str {
        match self {
            Self::Right => "right",
            Self::Down => "down",
        }
    }
}

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
