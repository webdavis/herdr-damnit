//! The colours each theme publishes, as the pane's eight anchors plus the cyan slot. Every value
//! is the theme's own: taken from its published palette, under whichever of cyan, teal or aqua that
//! theme calls it.

// This file is a color table; 6-digit `0xRRGGBB` literals read better grouped as one value.
#![allow(clippy::unreadable_literal)]

use ratatui::style::Color;

/// The anchor colors a theme names; the dim step is computed from these.
#[derive(Clone, Copy, Debug)]
pub struct Anchors {
    pub base: Color,
    pub text: Color,
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub orange: Color,
    pub purple: Color,
    pub blue: Color,
    pub cyan: Color,
}

pub const CATPPUCCIN_LATTE: Anchors = anchors(
    0xeff1f5, 0x4c4f69, 0xd20f39, 0x40a02b, 0xdf8e1d, 0xfe640b, 0x8839ef, 0x7287fd, 0x179299,
);
pub const DRACULA: Anchors = anchors(
    0x282a36, 0xf8f8f2, 0xff5555, 0x50fa7b, 0xf1fa8c, 0xffb86c, 0xbd93f9, 0x8be9fd, 0x8be9fd,
);
pub const NORD: Anchors = anchors(
    0x2e3440, 0xd8dee9, 0xbf616a, 0xa3be8c, 0xebcb8b, 0xd08770, 0xb48ead, 0x81a1c1, 0x88c0d0,
);
pub const GRUVBOX: Anchors = anchors(
    0x282828, 0xebdbb2, 0xfb4934, 0xb8bb26, 0xfabd2f, 0xfe8019, 0xd3869b, 0x83a598, 0x8ec07c,
);
pub const GRUVBOX_LIGHT: Anchors = anchors(
    0xfbf1c7, 0x3c3836, 0x9d0006, 0x79740e, 0xb57614, 0xaf3a03, 0x8f3f71, 0x076678, 0x427b58,
);
pub const ONE_DARK: Anchors = anchors(
    0x282c34, 0xabb2bf, 0xe06c75, 0x98c379, 0xe5c07b, 0xd19a66, 0xc678dd, 0x61afef, 0x56b6c2,
);
pub const ONE_LIGHT: Anchors = anchors(
    0xfafafa, 0x383a42, 0xe45649, 0x50a14f, 0xc18401, 0x986801, 0xa626a4, 0x4078f2, 0x0184bc,
);
pub const SOLARIZED: Anchors = anchors(
    0x002b36, 0x93a1a1, 0xdc322f, 0x859900, 0xb58900, 0xcb4b16, 0x6c71c4, 0x268bd2, 0x2aa198,
);
pub const SOLARIZED_LIGHT: Anchors = anchors(
    0xfdf6e3, 0x586e75, 0xdc322f, 0x859900, 0xb58900, 0xcb4b16, 0x6c71c4, 0x268bd2, 0x2aa198,
);
pub const FRAPPE: Anchors = anchors(
    0x303446, 0xc6d0f5, 0xe78284, 0xa6d189, 0xe5c890, 0xef9f76, 0xca9ee6, 0xbabbf1, 0x81c8be,
);
pub const MACCHIATO: Anchors = anchors(
    0x24273a, 0xcad3f5, 0xed8796, 0xa6da95, 0xeed49f, 0xf5a97f, 0xc6a0f6, 0xb7bdf8, 0x8bd5ca,
);
pub const GITHUB_LIGHT: Anchors = anchors(
    0xffffff, 0x1f2328, 0xcf222e, 0x1a7f37, 0x9a6700, 0xbc4c00, 0x8250df, 0x0969da, 0x1b7c83,
);
pub const MONOKAI: Anchors = anchors(
    0x272822, 0xf8f8f2, 0xf92672, 0xa6e22e, 0xe6db74, 0xfd971f, 0xae81ff, 0x66d9ef, 0x66d9ef,
);
pub const TOKYO_NIGHT: Anchors = anchors(
    0x1a1b26, 0xc0caf5, 0xf7768e, 0x9ece6a, 0xe0af68, 0xff9e64, 0xbb9af7, 0x7aa2f7, 0x7dcfff,
);
pub const TOKYO_NIGHT_DAY: Anchors = anchors(
    0xe1e2e7, 0x3760bf, 0xf52a65, 0x587539, 0x8c6c3e, 0xb15c00, 0x9854f1, 0x2e7de9, 0x007197,
);
pub const ROSE_PINE: Anchors = anchors(
    0x191724, 0xe0def4, 0xeb6f92, 0x9ccfd8, 0xf6c177, 0xebbcba, 0xc4a7e7, 0x31748f, 0x9ccfd8,
);
pub const ROSE_PINE_DAWN: Anchors = anchors(
    0xfaf4ed, 0x575279, 0xb4637a, 0x56949f, 0xea9d34, 0xd7827e, 0x907aa9, 0x286983, 0x56949f,
);

/// Build `Anchors` from `0xRRGGBB` literals: base, text, then the seven accents.
#[allow(clippy::too_many_arguments)]
pub const fn anchors(
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

pub const fn hex(rgb: u32) -> Color {
    Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}
