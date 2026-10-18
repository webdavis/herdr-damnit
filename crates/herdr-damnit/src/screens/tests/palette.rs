use ratatui::style::Color;

use herdr_damnit_domain::Slot;

#[test]
fn every_foreground_role_keeps_its_catppuccin_mocha_color() {
    let palette = crate::theme::resolve(Some("catppuccin"));

    for (slot, color) in [
        (Slot::Text, Color::Rgb(0xcd, 0xd6, 0xf4)),
        (Slot::Dim1, Color::Rgb(0x7f, 0x84, 0x9c)),
        (Slot::Red, Color::Rgb(0xf3, 0x8b, 0xa8)),
        (Slot::Green, Color::Rgb(0xa6, 0xe3, 0xa1)),
        (Slot::Yellow, Color::Rgb(0xf9, 0xe2, 0xaf)),
        (Slot::Orange, Color::Rgb(0xfa, 0xb3, 0x87)),
        (Slot::Purple, Color::Rgb(0xcb, 0xa6, 0xf7)),
        (Slot::Blue, Color::Rgb(0xb4, 0xbe, 0xfe)),
        (Slot::Cyan, Color::Rgb(0x94, 0xe2, 0xd5)),
    ] {
        assert_eq!(palette.color(slot), color, "{slot:?}");
    }
}
