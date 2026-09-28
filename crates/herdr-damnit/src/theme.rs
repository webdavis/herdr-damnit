#![allow(
    clippy::unreadable_literal,
    reason = "a color table, where a six-digit 0xRRGGBB literal reads better as one value"
)]

use ratatui::style::Color;

pub use herdr_damnit_domain::Slot;

mod anchors;
mod hue;

const CYAN_SEPARATION_FROM_BLUE_AND_GREEN: f64 = 30.0;

const CYAN_ROTATION_DEGREES: f64 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Appearance {
    Dark,
    Light,
}

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

pub fn resolve(name: Option<&str>) -> Palette {
    name.and_then(build).unwrap_or_else(catppuccin_mocha)
}

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

fn build(name: &str) -> Option<Palette> {
    use Appearance::{Dark, Light};
    use anchors::{
        CATPPUCCIN_LATTE, DRACULA, FRAPPE, GITHUB_LIGHT, GRUVBOX, GRUVBOX_LIGHT, MACCHIATO,
        MONOKAI, NORD, ONE_DARK, ONE_LIGHT, ROSE_PINE, ROSE_PINE_DAWN, SOLARIZED, SOLARIZED_LIGHT,
        TOKYO_NIGHT, TOKYO_NIGHT_DAY,
    };
    Some(match name {
        "catppuccin" => catppuccin_mocha(),
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

fn catppuccin_mocha() -> Palette {
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
        cyan: cyan_separated_from(Color::Rgb(0x94, 0xe2, 0xd5), blue, green),
    }
}

fn derive(a: anchors::Anchors, appearance: Appearance) -> Palette {
    let contrast_pole = match appearance {
        Appearance::Dark => WHITE,
        Appearance::Light => BLACK,
    };
    Palette {
        dim1: blend(a.base, contrast_pole, DIM_STEP_TOWARD_CONTRAST_POLE),
        text: a.text,
        red: a.red,
        green: a.green,
        yellow: a.yellow,
        orange: a.orange,
        purple: a.purple,
        blue: a.blue,
        cyan: cyan_separated_from(a.cyan, a.blue, a.green),
    }
}

fn cyan_separated_from(cyan: Color, blue: Color, green: Color) -> Color {
    let to_blue = hue::rgb_distance(cyan, blue);
    let to_green = hue::rgb_distance(cyan, green);
    let toward_blue = match (
        to_blue < CYAN_SEPARATION_FROM_BLUE_AND_GREEN,
        to_green < CYAN_SEPARATION_FROM_BLUE_AND_GREEN,
    ) {
        (false, false) => return cyan,
        (true, true) => to_green < to_blue,
        (true, false) => false,
        (false, true) => true,
    };
    match toward_blue {
        true => hue::rotate_hue(cyan, CYAN_ROTATION_DEGREES),
        false => hue::rotate_hue(cyan, -CYAN_ROTATION_DEGREES),
    }
}

const DIM_STEP_TOWARD_CONTRAST_POLE: f64 = 0.34;

const WHITE: Color = Color::Rgb(0xff, 0xff, 0xff);
const BLACK: Color = Color::Rgb(0x00, 0x00, 0x00);

fn blend(from: Color, to: Color, fraction: f64) -> Color {
    let (fr, fg, fb) = hue::channels(from);
    let (tr, tg, tb) = hue::channels(to);
    let mix = |lhs: u8, rhs: u8| {
        (f64::from(lhs) * (1.0 - fraction) + f64::from(rhs) * fraction).round() as u8
    };
    Color::Rgb(mix(fr, tr), mix(fg, tg), mix(fb, tb))
}

#[cfg(test)]
mod tests;
