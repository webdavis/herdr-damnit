//! Where the pane lands: which side of the pane the action ran in it takes, and how much of the
//! tab it gets. `herdr plugin pane open` splits right or down only and accepts no ratio, so a
//! left or up side, or any width, is one `herdr pane move` after the open.

use serde::Deserialize;

/// The side of the calling pane the Todoist pane takes.
#[derive(Debug, Default, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    #[default]
    Right,
    Left,
    Down,
    Up,
}

impl Side {
    /// The split `herdr` itself offers, which is rightward or downward only.
    pub fn split_direction(self) -> &'static str {
        match self {
            Self::Right | Self::Left => "right",
            Self::Down | Self::Up => "down",
        }
    }

    /// Whether the Todoist pane is the leading pane of the split, the left one or the top one.
    fn pane_leads(self) -> bool {
        matches!(self, Self::Left | Self::Up)
    }
}

/// One `herdr pane move`: the pane that ends up second is the one that moves, and `ratio` is the
/// leading pane's share of the tab.
#[derive(Debug, PartialEq)]
pub struct Arrangement {
    pub source: String,
    pub target: String,
    pub direction: &'static str,
    pub ratio: Option<f32>,
}

/// The move that puts `pane` on `side` of `neighbor` at `width`, or nothing when the open already
/// placed it: a rightward or downward side with no width asked for is what the open does.
pub fn arrange(side: Side, width: Option<f32>, pane: &str, neighbor: &str) -> Option<Arrangement> {
    if neighbor.is_empty() || (!side.pane_leads() && width.is_none()) {
        return None;
    }
    let (source, target, ratio) = if side.pane_leads() {
        (neighbor, pane, width)
    } else {
        (pane, neighbor, width.map(|width| 1.0 - width))
    };
    Some(Arrangement {
        source: source.to_string(),
        target: target.to_string(),
        direction: side.split_direction(),
        ratio,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rightward_pane_at_the_default_width_is_placed_by_the_open_itself() {
        assert_eq!(arrange(Side::Right, None, "w:p2", "w:p1"), None);
        assert_eq!(arrange(Side::Down, None, "w:p2", "w:p1"), None);
    }

    #[test]
    fn a_width_moves_the_pane_behind_its_neighbor_and_leaves_the_neighbor_the_rest() {
        let arrangement = arrange(Side::Right, Some(0.3), "w:p2", "w:p1").expect("a move");

        assert_eq!(arrangement.source, "w:p2");
        assert_eq!(arrangement.target, "w:p1");
        assert_eq!(arrangement.direction, "right");
        assert_eq!(arrangement.ratio, Some(0.7));
    }

    #[test]
    fn a_leading_side_moves_the_neighbor_instead_and_keeps_the_width_as_the_ratio() {
        let arrangement = arrange(Side::Left, Some(0.3), "w:p2", "w:p1").expect("a move");

        assert_eq!(arrangement.source, "w:p1");
        assert_eq!(arrangement.target, "w:p2");
        assert_eq!(arrangement.direction, "right");
        assert_eq!(arrangement.ratio, Some(0.3));

        let downward = arrange(Side::Up, None, "w:p2", "w:p1").expect("a move");
        assert_eq!(downward.direction, "down");
        assert_eq!(downward.ratio, None);
    }

    #[test]
    fn a_pane_with_no_neighbor_is_left_where_the_open_put_it() {
        assert_eq!(arrange(Side::Left, Some(0.3), "w:p2", ""), None);
    }
}
