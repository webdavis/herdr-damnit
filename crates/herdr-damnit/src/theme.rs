//! The pane's colors: the theme names herdr and reviewr use, resolved to the same palette slots
//! those two resolve them to, so a color named `blue` here is the color named `blue` there and the
//! two panes read as one workspace. A theme is a few anchor colors; the dim step is derived from
//! them by the same rule reviewr derives its own. The pane background stays the terminal's: only
//! foregrounds are painted.

// This file is a color table; 6-digit `0xRRGGBB` literals read better grouped as one value.
#![allow(clippy::unreadable_literal)]

use ratatui::style::Color;

pub use herdr_damnit_domain::Slot;

mod anchors;
mod hue;

/// How far apart two painted marks must sit to read as two colours at a glyph's width. A row can
/// carry an unpushed mark beside an upcoming one or a staged one, so the cyan slot is held this far
/// from the blue and the green.
const SEPARATION: f64 = 30.0;

/// How far a cyan that repeats a neighbour is carried around the hue circle, away from it. One step
/// is what clears `SEPARATION` on every theme that needs it, which is what keeps the rule to a
/// single move.
const ROTATION: f64 = 30.0;

/// A theme's intrinsic cast, which sets the direction the dim step goes in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Appearance {
    Dark,
    Light,
}

/// The resolved colors the pane paints with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub dim1: Color,
    pub text: Color,
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub orange: Color,
    pub purple: Color,
    pub blue: Color,
    pub cyan: Color,
}

impl Palette {
    /// The color of a slot.
    pub fn color(&self, slot: Slot) -> Color {
        match slot {
            Slot::Text => self.text,
            Slot::Dim1 => self.dim1,
            Slot::Red => self.red,
            Slot::Green => self.green,
            Slot::Yellow => self.yellow,
            Slot::Orange => self.orange,
            Slot::Purple => self.purple,
            Slot::Blue => self.blue,
            Slot::Cyan => self.cyan,
        }
    }
}

/// Resolve a theme name to its palette. An unnamed theme is the default.
pub fn resolve(name: Option<&str>) -> Palette {
    name.and_then(build).unwrap_or_else(catppuccin)
}

/// Whether `name` selects a palette this pane knows, which the config checks before the pane runs.
pub fn is_known(name: &str) -> bool {
    build(name).is_some()
}

/// Every name this pane answers to, in the order the config error lists them.
pub const NAMES: &[&str] = &[
    "catppuccin",
    "catppuccin-latte",
    "catppuccin-frappe",
    "catppuccin-macchiato",
    "dracula",
    "github-light",
    "gruvbox",
    "gruvbox-light",
    "monokai",
    "nord",
    "one-dark",
    "one-light",
    "rose-pine",
    "rose-pine-dawn",
    "solarized",
    "solarized-light",
    "tokyo-night",
    "tokyo-night-day",
];

/// The palette for `name`. The names and their anchors are reviewr's, which are herdr's, so the
/// value copied from a herdr config resolves to the same colors in all three.
fn build(name: &str) -> Option<Palette> {
    use Appearance::{Dark, Light};
    use anchors::{
        CATPPUCCIN_LATTE, DRACULA, FRAPPE, GITHUB_LIGHT, GRUVBOX, GRUVBOX_LIGHT, MACCHIATO,
        MONOKAI, NORD, ONE_DARK, ONE_LIGHT, ROSE_PINE, ROSE_PINE_DAWN, SOLARIZED, SOLARIZED_LIGHT,
        TOKYO_NIGHT, TOKYO_NIGHT_DAY,
    };
    Some(match name {
        "catppuccin" => catppuccin(),
        "catppuccin-latte" => derive(CATPPUCCIN_LATTE, Light),
        "catppuccin-frappe" => derive(FRAPPE, Dark),
        "catppuccin-macchiato" => derive(MACCHIATO, Dark),
        "dracula" => derive(DRACULA, Dark),
        "github-light" => derive(GITHUB_LIGHT, Light),
        "gruvbox" => derive(GRUVBOX, Dark),
        "gruvbox-light" => derive(GRUVBOX_LIGHT, Light),
        "monokai" => derive(MONOKAI, Dark),
        "nord" => derive(NORD, Dark),
        "one-dark" => derive(ONE_DARK, Dark),
        "one-light" => derive(ONE_LIGHT, Light),
        "rose-pine" => derive(ROSE_PINE, Dark),
        "rose-pine-dawn" => derive(ROSE_PINE_DAWN, Light),
        "solarized" => derive(SOLARIZED, Dark),
        "solarized-light" => derive(SOLARIZED_LIGHT, Light),
        "tokyo-night" => derive(TOKYO_NIGHT, Dark),
        "tokyo-night-day" => derive(TOKYO_NIGHT_DAY, Light),
        _ => return None,
    })
}

/// Catppuccin Mocha, pinned to its canonical values, which is how reviewr carries it.
fn catppuccin() -> Palette {
    let green = Color::Rgb(0xa6, 0xe3, 0xa1);
    let blue = Color::Rgb(0xb4, 0xbe, 0xfe);
    Palette {
        dim1: Color::Rgb(0x7f, 0x84, 0x9c),
        text: Color::Rgb(0xcd, 0xd6, 0xf4),
        red: Color::Rgb(0xf3, 0x8b, 0xa8),
        green,
        yellow: Color::Rgb(0xf9, 0xe2, 0xaf),
        orange: Color::Rgb(0xfa, 0xb3, 0x87),
        purple: Color::Rgb(0xcb, 0xa6, 0xf7),
        blue,
        cyan: separated(Color::Rgb(0x94, 0xe2, 0xd5), blue, green),
    }
}

/// A full palette from anchors: `dim1` steps the base toward the contrast pole, lighter for a
/// dark theme and darker for a light one, at the same fraction reviewr steps it.
fn derive(a: anchors::Anchors, appearance: Appearance) -> Palette {
    let pole = match appearance {
        Appearance::Dark => WHITE,
        Appearance::Light => BLACK,
    };
    Palette {
        dim1: blend(a.base, pole, 0.34),
        text: a.text,
        red: a.red,
        green: a.green,
        yellow: a.yellow,
        orange: a.orange,
        purple: a.purple,
        blue: a.blue,
        cyan: separated(a.cyan, a.blue, a.green),
    }
}

/// The cyan a theme paints. A published cyan that stands apart from the blue and the green beside
/// it is painted as it is; one that repeats either is carried one `ROTATION` away from it, toward
/// green when it repeats the blue and toward blue when it repeats the green. A cyan too close to
/// both leaves the nearer.
fn separated(cyan: Color, blue: Color, green: Color) -> Color {
    let to_blue = hue::distance(cyan, blue);
    let to_green = hue::distance(cyan, green);
    let toward_blue = match (to_blue < SEPARATION, to_green < SEPARATION) {
        (false, false) => return cyan,
        (true, true) => to_green < to_blue,
        (true, false) => false,
        (false, true) => true,
    };
    match toward_blue {
        true => hue::rotate(cyan, ROTATION),
        false => hue::rotate(cyan, -ROTATION),
    }
}

const WHITE: Color = Color::Rgb(0xff, 0xff, 0xff);
const BLACK: Color = Color::Rgb(0x00, 0x00, 0x00);

/// Linear per-channel blend: `t` of the way from `from` to `to`.
fn blend(from: Color, to: Color, t: f64) -> Color {
    let (fr, fg, fb) = hue::channels(from);
    let (tr, tg, tb) = hue::channels(to);
    let mix = |lhs: u8, rhs: u8| (f64::from(lhs) * (1.0 - t) + f64::from(rhs) * t).round() as u8;
    Color::Rgb(mix(fr, tr), mix(fg, tg), mix(fb, tb))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchors::hex;

    #[test]
    fn the_default_palette_is_reviewrs_own_mocha() {
        // The slots reviewr pins as literals, which is what makes a name mean one color in both
        // panes: `crates/herdr-damnit/src/theme.rs` and reviewr's `src/theme.rs` agree here.
        let palette = resolve(None);

        assert_eq!(palette.text, Color::Rgb(0xcd, 0xd6, 0xf4));
        assert_eq!(palette.red, Color::Rgb(0xf3, 0x8b, 0xa8));
        assert_eq!(palette.orange, Color::Rgb(0xfa, 0xb3, 0x87));
        assert_eq!(palette.purple, Color::Rgb(0xcb, 0xa6, 0xf7));
        assert_eq!(palette.blue, Color::Rgb(0xb4, 0xbe, 0xfe));
        assert_eq!(palette.dim1, Color::Rgb(0x7f, 0x84, 0x9c));
        assert_eq!(palette.cyan, Color::Rgb(0x94, 0xe2, 0xd5));
    }

    /// The separation the rule is ruled to hold, written out rather than read from the constant
    /// the rule itself reads, so moving that constant in either direction fails here.
    const RULED_SEPARATION: f64 = 30.0;

    /// No two marks a row can carry at once may read as one colour, so every theme's cyan stands
    /// at least `RULED_SEPARATION` from the blue and the green beside it.
    #[test]
    fn every_theme_paints_a_cyan_no_mark_is_mistaken_for() {
        for name in NAMES {
            let palette = resolve(Some(name));

            assert!(
                hue::distance(palette.cyan, palette.blue) >= RULED_SEPARATION,
                "{name}: cyan sits on its blue"
            );
            assert!(
                hue::distance(palette.cyan, palette.green) >= RULED_SEPARATION,
                "{name}: cyan sits on its green"
            );
        }
    }

    /// A theme whose published cyan already stands apart is painted with it, untouched.
    #[test]
    fn a_published_cyan_that_stands_apart_is_the_one_painted() {
        for (name, cyan) in [
            ("catppuccin", 0x94e2d5),
            ("catppuccin-latte", 0x179299),
            ("catppuccin-frappe", 0x81c8be),
            ("catppuccin-macchiato", 0x8bd5ca),
            ("github-light", 0x1b7c83),
            ("gruvbox", 0x8ec07c),
            ("gruvbox-light", 0x427b58),
            ("nord", 0x88c0d0),
            ("one-dark", 0x56b6c2),
            ("one-light", 0x0184bc),
            ("solarized", 0x2aa198),
            ("solarized-light", 0x2aa198),
            ("tokyo-night", 0x7dcfff),
            ("tokyo-night-day", 0x007197),
        ] {
            assert_eq!(resolve(Some(name)).cyan, hex(cyan), "{name}");
        }
    }

    /// Four themes publish one colour where this pane paints two. Dracula and monokai name their
    /// cyan as the blue this pane draws upcoming marks in; both Rose Pine variants put foam in the
    /// green slot. Each takes its published cyan rotated a step away from the neighbour it repeats.
    #[test]
    fn a_published_cyan_that_repeats_a_neighbour_is_rotated_off_it() {
        for (name, published, painted) in [
            ("dracula", 0x8be9fd, 0x8bfdd8),
            ("monokai", 0x66d9ef, 0x66efc1),
            ("rose-pine", 0x9ccfd8, 0x9cb1d8),
            ("rose-pine-dawn", 0x56949f, 0x56709f),
        ] {
            let painted_cyan = resolve(Some(name)).cyan;

            assert_eq!(painted_cyan, hex(painted), "{name}");
            assert_ne!(
                painted_cyan,
                hex(published),
                "{name} kept a cyan it repeats"
            );
        }
    }

    /// The ruled separation, either side of it. No shipped theme sits in that band, so the number
    /// itself is held here rather than through a palette.
    #[test]
    fn a_cyan_is_rotated_below_the_ruled_separation_and_left_alone_at_it() {
        let cyan = Color::Rgb(0x80, 0xc0, 0xc0);
        let far = Color::Rgb(0x20, 0x20, 0x20);
        let inside = Color::Rgb(0x80, 0xdb, 0xc0);
        let outside = Color::Rgb(0x80, 0xe1, 0xc0);

        assert_eq!(hue::distance(cyan, inside).round(), RULED_SEPARATION - 3.0);
        assert_eq!(hue::distance(cyan, outside).round(), RULED_SEPARATION + 3.0);
        assert_eq!(
            separated(cyan, far, inside),
            hue::rotate(cyan, ROTATION),
            "a green inside the separation is left"
        );
        assert_eq!(
            separated(cyan, far, outside),
            cyan,
            "a green outside the separation is lived with"
        );
    }

    /// No theme ships a cyan that repeats both its neighbours, so the rule that picks which one to
    /// leave is exercised here rather than through a palette.
    #[test]
    fn a_cyan_too_close_to_both_neighbours_leaves_the_nearer_one() {
        let cyan = Color::Rgb(0x80, 0xc0, 0xc0);
        let near = Color::Rgb(0x84, 0xc4, 0xc4);
        let far = Color::Rgb(0x76, 0xb6, 0xb6);

        assert_eq!(
            separated(cyan, far, near),
            hue::rotate(cyan, ROTATION),
            "a nearer green is left toward blue"
        );
        assert_eq!(
            separated(cyan, near, far),
            hue::rotate(cyan, -ROTATION),
            "a nearer blue is left toward green"
        );
    }

    #[test]
    fn every_theme_carries_a_cyan_of_its_own() {
        assert_eq!(
            NAMES.len(),
            18,
            "a theme was added; give it a published cyan and a row in the tests above"
        );
    }
}
