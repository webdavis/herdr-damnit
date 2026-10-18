use herdr_damnit_domain::Slot;
use ratatui::style::{Color, Style};

pub const BASE: Color = Color::Rgb(0x1e, 0x1e, 0x2e);

pub const fn color(slot: Slot) -> Color {
    match slot {
        Slot::Text => Color::Rgb(0xcd, 0xd6, 0xf4),
        Slot::Dim1 => Color::Rgb(0x7f, 0x84, 0x9c),
        Slot::Red => Color::Rgb(0xf3, 0x8b, 0xa8),
        Slot::Green => Color::Rgb(0xa6, 0xe3, 0xa1),
        Slot::Yellow => Color::Rgb(0xf9, 0xe2, 0xaf),
        Slot::Orange => Color::Rgb(0xfa, 0xb3, 0x87),
        Slot::Purple => Color::Rgb(0xcb, 0xa6, 0xf7),
        Slot::Blue => Color::Rgb(0xb4, 0xbe, 0xfe),
        Slot::Cyan => Color::Rgb(0x94, 0xe2, 0xd5),
    }
}

pub fn ground() -> Style {
    Style::new().fg(color(Slot::Text)).bg(BASE)
}

pub fn bar() -> Style {
    Style::new().bg(Color::Rgb(0x36, 0x36, 0x46))
}
