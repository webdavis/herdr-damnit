//! The pane's colors: the theme names herdr and reviewr use, resolved to the same palette slots
//! those two resolve them to, so a color named `blue` here is the color named `blue` there and the
//! two panes read as one workspace. A theme is a few anchor colors; the dim step is derived from
//! them by the same rule reviewr derives its own. The pane background stays the terminal's: only
//! foregrounds are painted.

// This file is a color table; 6-digit `0xRRGGBB` literals read better grouped as one value.
#![allow(clippy::unreadable_literal)]

use ratatui::style::Color;

pub use herdr_damnit_domain::Slot;

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
    Palette {
        dim1: Color::Rgb(0x7f, 0x84, 0x9c),
        text: Color::Rgb(0xcd, 0xd6, 0xf4),
        red: Color::Rgb(0xf3, 0x8b, 0xa8),
        green: Color::Rgb(0xa6, 0xe3, 0xa1),
        yellow: Color::Rgb(0xf9, 0xe2, 0xaf),
        orange: Color::Rgb(0xfa, 0xb3, 0x87),
        purple: Color::Rgb(0xcb, 0xa6, 0xf7),
        blue: Color::Rgb(0xb4, 0xbe, 0xfe),
        cyan: Color::Rgb(0x94, 0xe2, 0xd5),
    }
}

/// The anchor colors a theme names; the dim step is computed from these.
#[derive(Clone, Copy, Debug)]
struct Anchors {
    base: Color,
    text: Color,
    red: Color,
    green: Color,
    yellow: Color,
    orange: Color,
    purple: Color,
    blue: Color,
    cyan: Color,
}

const CATPPUCCIN_LATTE: Anchors = anchors(
    0xeff1f5, 0x4c4f69, 0xd20f39, 0x40a02b, 0xdf8e1d, 0xfe640b, 0x8839ef, 0x7287fd, 0x179299,
);
const DRACULA: Anchors = anchors(
    0x282a36, 0xf8f8f2, 0xff5555, 0x50fa7b, 0xf1fa8c, 0xffb86c, 0xbd93f9, 0x8be9fd, 0x8be9fd,
);
const NORD: Anchors = anchors(
    0x2e3440, 0xd8dee9, 0xbf616a, 0xa3be8c, 0xebcb8b, 0xd08770, 0xb48ead, 0x81a1c1, 0x88c0d0,
);
const GRUVBOX: Anchors = anchors(
    0x282828, 0xebdbb2, 0xfb4934, 0xb8bb26, 0xfabd2f, 0xfe8019, 0xd3869b, 0x83a598, 0x8ec07c,
);
const GRUVBOX_LIGHT: Anchors = anchors(
    0xfbf1c7, 0x3c3836, 0x9d0006, 0x79740e, 0xb57614, 0xaf3a03, 0x8f3f71, 0x076678, 0x427b58,
);
const ONE_DARK: Anchors = anchors(
    0x282c34, 0xabb2bf, 0xe06c75, 0x98c379, 0xe5c07b, 0xd19a66, 0xc678dd, 0x61afef, 0x56b6c2,
);
const ONE_LIGHT: Anchors = anchors(
    0xfafafa, 0x383a42, 0xe45649, 0x50a14f, 0xc18401, 0x986801, 0xa626a4, 0x4078f2, 0x0184bc,
);
const SOLARIZED: Anchors = anchors(
    0x002b36, 0x93a1a1, 0xdc322f, 0x859900, 0xb58900, 0xcb4b16, 0x6c71c4, 0x268bd2, 0x2aa198,
);
const SOLARIZED_LIGHT: Anchors = anchors(
    0xfdf6e3, 0x586e75, 0xdc322f, 0x859900, 0xb58900, 0xcb4b16, 0x6c71c4, 0x268bd2, 0x2aa198,
);
const FRAPPE: Anchors = anchors(
    0x303446, 0xc6d0f5, 0xe78284, 0xa6d189, 0xe5c890, 0xef9f76, 0xca9ee6, 0xbabbf1, 0x81c8be,
);
const MACCHIATO: Anchors = anchors(
    0x24273a, 0xcad3f5, 0xed8796, 0xa6da95, 0xeed49f, 0xf5a97f, 0xc6a0f6, 0xb7bdf8, 0x8bd5ca,
);
const GITHUB_LIGHT: Anchors = anchors(
    0xffffff, 0x1f2328, 0xcf222e, 0x1a7f37, 0x9a6700, 0xbc4c00, 0x8250df, 0x0969da, 0x1b7c83,
);
const MONOKAI: Anchors = anchors(
    0x272822, 0xf8f8f2, 0xf92672, 0xa6e22e, 0xe6db74, 0xfd971f, 0xae81ff, 0x66d9ef, 0x66d9ef,
);
const TOKYO_NIGHT: Anchors = anchors(
    0x1a1b26, 0xc0caf5, 0xf7768e, 0x9ece6a, 0xe0af68, 0xff9e64, 0xbb9af7, 0x7aa2f7, 0x7dcfff,
);
const TOKYO_NIGHT_DAY: Anchors = anchors(
    0xe1e2e7, 0x3760bf, 0xf52a65, 0x587539, 0x8c6c3e, 0xb15c00, 0x9854f1, 0x2e7de9, 0x007197,
);
const ROSE_PINE: Anchors = anchors(
    0x191724, 0xe0def4, 0xeb6f92, 0x9ccfd8, 0xf6c177, 0xebbcba, 0xc4a7e7, 0x31748f, 0x9ccfd8,
);
const ROSE_PINE_DAWN: Anchors = anchors(
    0xfaf4ed, 0x575279, 0xb4637a, 0x56949f, 0xea9d34, 0xd7827e, 0x907aa9, 0x286983, 0x56949f,
);

/// Build `Anchors` from `0xRRGGBB` literals: base, text, then the seven accents.
#[allow(clippy::too_many_arguments)]
const fn anchors(
    base: u32,
    text: u32,
    red: u32,
    green: u32,
    yellow: u32,
    orange: u32,
    purple: u32,
    blue: u32,
    cyan: u32,
) -> Anchors {
    Anchors {
        base: hex(base),
        text: hex(text),
        red: hex(red),
        green: hex(green),
        yellow: hex(yellow),
        orange: hex(orange),
        purple: hex(purple),
        blue: hex(blue),
        cyan: hex(cyan),
    }
}

const fn hex(rgb: u32) -> Color {
    Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// A full palette from anchors: `dim1` steps the base toward the contrast pole, lighter for a
/// dark theme and darker for a light one, at the same fraction reviewr steps it.
fn derive(a: Anchors, appearance: Appearance) -> Palette {
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
        cyan: a.cyan,
    }
}

const WHITE: Color = Color::Rgb(0xff, 0xff, 0xff);
const BLACK: Color = Color::Rgb(0x00, 0x00, 0x00);

/// Linear per-channel blend: `t` of the way from `from` to `to`.
fn blend(from: Color, to: Color, t: f64) -> Color {
    let (fr, fg, fb) = channels(from);
    let (tr, tg, tb) = channels(to);
    let mix = |lhs: u8, rhs: u8| (f64::from(lhs) * (1.0 - t) + f64::from(rhs) * t).round() as u8;
    Color::Rgb(mix(fr, tr), mix(fg, tg), mix(fb, tb))
}

fn channels(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (0, 0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Each theme's cyan is the value its own palette publishes, so a mark painted cyan is the
    /// cyan a user of that theme already reads as cyan everywhere else.
    #[test]
    fn every_theme_paints_the_cyan_its_own_palette_publishes() {
        let published = [
            ("catppuccin", 0x94e2d5),
            ("catppuccin-latte", 0x179299),
            ("catppuccin-frappe", 0x81c8be),
            ("catppuccin-macchiato", 0x8bd5ca),
            ("dracula", 0x8be9fd),
            ("github-light", 0x1b7c83),
            ("gruvbox", 0x8ec07c),
            ("gruvbox-light", 0x427b58),
            ("monokai", 0x66d9ef),
            ("nord", 0x88c0d0),
            ("one-dark", 0x56b6c2),
            ("one-light", 0x0184bc),
            ("rose-pine", 0x9ccfd8),
            ("rose-pine-dawn", 0x56949f),
            ("solarized", 0x2aa198),
            ("solarized-light", 0x2aa198),
            ("tokyo-night", 0x7dcfff),
            ("tokyo-night-day", 0x007197),
        ];

        assert_eq!(
            published.len(),
            NAMES.len(),
            "a theme carries no pinned cyan"
        );
        for (name, cyan) in published {
            assert_eq!(resolve(Some(name)).cyan, hex(cyan), "{name}");
        }
    }

    #[test]
    fn a_derived_theme_keeps_its_anchors_and_steps_dim1_from_the_base() {
        // gruvbox: the anchor reviewr lists, and the dim1 step its derivation produces.
        let palette = resolve(Some("gruvbox"));

        assert_eq!(palette.red, Color::Rgb(0xfb, 0x49, 0x34));
        assert_eq!(palette.dim1, Color::Rgb(0x71, 0x71, 0x71));
    }

    #[test]
    fn every_named_theme_resolves_and_an_unknown_one_is_not_known() {
        for name in NAMES {
            assert!(is_known(name), "{name} should resolve");
        }
        assert!(!is_known("nope"));
        assert_eq!(resolve(Some("nope")), resolve(None));
    }

    #[test]
    fn a_light_theme_steps_dim1_darker_than_its_base() {
        let palette = resolve(Some("github-light"));

        assert_eq!(palette.dim1, Color::Rgb(0xa8, 0xa8, 0xa8));
    }

    #[test]
    fn a_slot_names_the_color_it_paints() {
        let palette = resolve(None);

        assert_eq!(palette.color(Slot::Red), palette.red);
        assert_eq!(palette.color(Slot::Dim1), palette.dim1);
        assert_eq!(palette.color(Slot::Text), palette.text);
        assert_eq!(palette.color(Slot::Cyan), palette.cyan);
    }
}
