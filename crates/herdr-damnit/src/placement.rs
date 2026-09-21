//! Where the pane lands: which side of the calling pane it takes, and how much of the tab it
//! gets. `herdr plugin pane open --direction` splits rightward or downward only and takes no
//! ratio, so a side maps straight onto that direction and a width is a resize afterward.

use serde::Deserialize;

/// The side of the calling pane the Todoist pane takes. herdr's own open only splits rightward
/// or downward, so those are the only sides there are.
#[derive(Debug, Default, Deserialize, PartialEq, Eq, Clone, Copy)]
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

/// The calling pane's share of the tab once the Todoist pane has taken `width`: whatever is
/// left. `herdr pane resize` moves the calling pane to this ratio to get there.
pub fn leading_share(width: f32) -> f32 {
    1.0 - width
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_side_maps_onto_the_direction_herdr_splits_in() {
        assert_eq!(Side::Right.split_direction(), "right");
        assert_eq!(Side::Down.split_direction(), "down");
    }

    #[test]
    fn the_calling_panes_share_is_the_rest_of_the_tab() {
        assert_eq!(leading_share(0.3), 0.7);
        assert_eq!(leading_share(0.7), 0.3);
    }
}
