//! Colour geometry over the palette: how far apart two painted marks are, and how to carry one
//! around the hue circle without changing how bright or how vivid it is.

use ratatui::style::Color;

/// The straight-line distance between two colours in RGB.
pub fn distance(left: Color, right: Color) -> f64 {
    let (lr, lg, lb) = channels(left);
    let (rr, rg, rb) = channels(right);
    let square = |a: u8, b: u8| (f64::from(a) - f64::from(b)).powi(2);
    (square(lr, rr) + square(lg, rg) + square(lb, rb)).sqrt()
}

/// `color` carried `degrees` around the hue circle at the saturation and lightness it already has.
/// Hue rises toward blue and falls toward green, so the sign chooses the direction.
pub fn rotate(color: Color, degrees: f64) -> Color {
    let (hue, saturation, lightness) = to_hsl(color);
    from_hsl((hue + degrees).rem_euclid(360.0), saturation, lightness)
}

pub fn channels(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (0, 0, 0),
    }
}

fn to_hsl(color: Color) -> (f64, f64, f64) {
    let (r, g, b) = channels(color);
    let (r, g, b) = (
        f64::from(r) / 255.0,
        f64::from(g) / 255.0,
        f64::from(b) / 255.0,
    );
    let high = r.max(g).max(b);
    let low = r.min(g).min(b);
    let span = high - low;
    let lightness = (high + low) / 2.0;
    if span == 0.0 {
        return (0.0, 0.0, lightness);
    }
    let saturation = span / (1.0 - (2.0 * lightness - 1.0).abs());
    let hue = if high == r {
        60.0 * (((g - b) / span) % 6.0)
    } else if high == g {
        60.0 * ((b - r) / span + 2.0)
    } else {
        60.0 * ((r - g) / span + 4.0)
    };
    (hue.rem_euclid(360.0), saturation, lightness)
}

fn from_hsl(hue: f64, saturation: f64, lightness: f64) -> Color {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sextant = hue / 60.0;
    let second = chroma * (1.0 - (sextant % 2.0 - 1.0).abs());
    let (r, g, b) = match sextant as u8 {
        0 => (chroma, second, 0.0),
        1 => (second, chroma, 0.0),
        2 => (0.0, chroma, second),
        3 => (0.0, second, chroma),
        4 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };
    let base = lightness - chroma / 2.0;
    let byte = |value: f64| ((value + base) * 255.0).round().clamp(0.0, 255.0) as u8;
    Color::Rgb(byte(r), byte(g), byte(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOAM: Color = Color::Rgb(0x9c, 0xcf, 0xd8);

    #[test]
    fn the_distance_between_a_colour_and_itself_is_nothing() {
        assert_eq!(distance(FOAM, FOAM), 0.0);
        assert!((distance(Color::Rgb(0, 0, 0), Color::Rgb(0, 0, 255)) - 255.0).abs() < 0.001);
    }

    #[test]
    fn a_rotation_of_nothing_returns_the_colour_it_was_given() {
        assert_eq!(rotate(FOAM, 0.0), FOAM);
    }

    #[test]
    fn a_rotation_keeps_the_saturation_and_the_lightness_it_started_with() {
        let (_, saturation, lightness) = to_hsl(FOAM);
        let (_, rotated_saturation, rotated_lightness) = to_hsl(rotate(FOAM, 30.0));

        assert!(
            (saturation - rotated_saturation).abs() < 0.01,
            "saturation moved"
        );
        assert!(
            (lightness - rotated_lightness).abs() < 0.01,
            "lightness moved"
        );
    }

    #[test]
    fn a_positive_rotation_goes_toward_blue_and_a_negative_one_toward_green() {
        let (start, _, _) = to_hsl(FOAM);
        let (toward_blue, _, _) = to_hsl(rotate(FOAM, 30.0));
        let (toward_green, _, _) = to_hsl(rotate(FOAM, -30.0));

        assert!((toward_blue - (start + 30.0)).abs() < 1.0, "{toward_blue}");
        assert!(
            (toward_green - (start - 30.0)).abs() < 1.0,
            "{toward_green}"
        );
    }

    #[test]
    fn a_grey_has_no_hue_to_rotate_and_comes_back_unchanged() {
        let grey = Color::Rgb(0x80, 0x80, 0x80);
        assert_eq!(rotate(grey, 120.0), grey);
    }
}
