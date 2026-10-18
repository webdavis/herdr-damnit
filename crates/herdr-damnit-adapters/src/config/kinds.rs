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

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    #[default]
    Right,
    Down,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    #[default]
    CatppuccinMocha,
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
