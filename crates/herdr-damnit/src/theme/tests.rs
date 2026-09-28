use super::*;
use anchors::hex;

#[test]
fn the_default_palette_is_reviewrs_own_mocha() {
    let palette = resolve(None);

    assert_eq!(palette.text, Color::Rgb(0xcd, 0xd6, 0xf4));
    assert_eq!(palette.red, Color::Rgb(0xf3, 0x8b, 0xa8));
    assert_eq!(palette.orange, Color::Rgb(0xfa, 0xb3, 0x87));
    assert_eq!(palette.purple, Color::Rgb(0xcb, 0xa6, 0xf7));
    assert_eq!(palette.blue, Color::Rgb(0xb4, 0xbe, 0xfe));
    assert_eq!(palette.dim1, Color::Rgb(0x7f, 0x84, 0x9c));
    assert_eq!(palette.cyan, Color::Rgb(0x94, 0xe2, 0xd5));
}

const RULED_SEPARATION: f64 = 30.0;

#[test]
fn every_theme_paints_a_cyan_no_mark_is_mistaken_for() {
    for name in NAMES {
        let palette = resolve(Some(name));

        assert!(
            hue::rgb_distance(palette.cyan, palette.blue) >= RULED_SEPARATION,
            "{name}: cyan sits on its blue"
        );
        assert!(
            hue::rgb_distance(palette.cyan, palette.green) >= RULED_SEPARATION,
            "{name}: cyan sits on its green"
        );
    }
}

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

#[test]
fn a_cyan_is_rotated_below_the_ruled_separation_and_left_alone_at_it() {
    let cyan = Color::Rgb(0x80, 0xc0, 0xc0);
    let far = Color::Rgb(0x20, 0x20, 0x20);
    let inside = Color::Rgb(0x80, 0xdb, 0xc0);
    let outside = Color::Rgb(0x80, 0xe1, 0xc0);

    assert_eq!(
        hue::rgb_distance(cyan, inside).round(),
        RULED_SEPARATION - 3.0
    );
    assert_eq!(
        hue::rgb_distance(cyan, outside).round(),
        RULED_SEPARATION + 3.0
    );
    assert_eq!(
        cyan_separated_from(cyan, far, inside),
        hue::rotate_hue(cyan, CYAN_ROTATION_DEGREES),
        "a green inside the separation is left"
    );
    assert_eq!(
        cyan_separated_from(cyan, far, outside),
        cyan,
        "a green outside the separation is lived with"
    );
}

#[test]
fn a_cyan_too_close_to_both_neighbours_leaves_the_nearer_one() {
    let cyan = Color::Rgb(0x80, 0xc0, 0xc0);
    let near = Color::Rgb(0x84, 0xc4, 0xc4);
    let far = Color::Rgb(0x76, 0xb6, 0xb6);

    assert_eq!(
        cyan_separated_from(cyan, far, near),
        hue::rotate_hue(cyan, CYAN_ROTATION_DEGREES),
        "a nearer green is left toward blue"
    );
    assert_eq!(
        cyan_separated_from(cyan, near, far),
        hue::rotate_hue(cyan, -CYAN_ROTATION_DEGREES),
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
