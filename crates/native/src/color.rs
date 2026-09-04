//! Color mapping from `alacritty_terminal` cell colors to `egui::Color32`.
//!
//! This is the deterministic piece of the renderer: it converts an
//! `alacritty_terminal::vte::ansi::Color` (which can be a truecolor `Rgb`, an
//! indexed 0..=255 color, or one of the 16 named/ANSI colors) into the exact
//! `egui::Color32` used to paint a cell.
//!
//! The mapping is table-driven so it can be unit-tested without a display.

use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb};
use egui::Color32;

/// The 16 legacy ANSI colors (indexes 0..=15) used by the xterm-256color
/// palette. This matches the default alacritty color scheme.
const ANSI_16: [[u8; 3]; 16] = [
    // normal
    [0x18, 0x18, 0x18], // 0 black
    [0xac, 0x42, 0x42], // 1 red
    [0x90, 0xa9, 0x59], // 2 green
    [0xf4, 0xbf, 0x75], // 3 yellow
    [0x6a, 0x9f, 0xb5], // 4 blue
    [0xaa, 0x75, 0x9f], // 5 magenta
    [0x75, 0xb5, 0xaa], // 6 cyan
    [0xd8, 0xd8, 0xd8], // 7 white
    // bright
    [0x6b, 0x6b, 0x6b], // 8 bright black
    [0xc5, 0x55, 0x55], // 9 bright red
    [0xaa, 0xc4, 0x74], // 10 bright green
    [0xfe, 0xca, 0x88], // 11 bright yellow
    [0x82, 0xb8, 0xc8], // 12 bright blue
    [0xc2, 0x8c, 0xb8], // 13 bright magenta
    [0x93, 0xd3, 0xc3], // 14 bright cyan
    [0xf8, 0xf8, 0xf8], // 15 bright white
];

/// Default foreground/background used when a Named color is requested.
pub const DEFAULT_FOREGROUND: Color32 = Color32::from_rgb(0xd8, 0xd8, 0xd8);
pub const DEFAULT_BACKGROUND: Color32 = Color32::from_rgb(0x18, 0x18, 0x18);

/// Map an indexed ANSI 256 color (0..=255) to `Color32`.
///
/// Indexes 0..=15 use the legacy palette; 16..=231 are the 6x6x6 color cube;
/// 232..=255 are the 24-step grayscale ramp.
pub fn ansi256_color(index: u8) -> Color32 {
    match index {
        0..=15 => {
            let [r, g, b] = ANSI_16[index as usize];
            Color32::from_rgb(r, g, b)
        }
        16..=231 => {
            let i = index - 16;
            let r = cube_component(i / 36);
            let g = cube_component((i % 36) / 6);
            let b = cube_component(i % 6);
            Color32::from_rgb(r, g, b)
        }
        232..=255 => {
            let value = 8 + (index - 232) * 10;
            Color32::from_rgb(value, value, value)
        }
    }
}

/// Convert a color-cube component (0..=5) to its byte value.
///
/// The 6x6x6 cube uses values `0, 95, 135, 175, 215, 255` (xterm formula:
/// `if n == 0 { 0 } else { 55 + 40*n }`).
fn cube_component(n: u8) -> u8 {
    if n == 0 {
        0
    } else {
        55 + 40 * n
    }
}

/// Map a named (ANSI-16 / special) color to `Color32` using the default
/// alacritty palette.
pub fn named_color(color: NamedColor) -> Color32 {
    match color {
        NamedColor::Black => rgb(ANSI_16[0]),
        NamedColor::Red => rgb(ANSI_16[1]),
        NamedColor::Green => rgb(ANSI_16[2]),
        NamedColor::Yellow => rgb(ANSI_16[3]),
        NamedColor::Blue => rgb(ANSI_16[4]),
        NamedColor::Magenta => rgb(ANSI_16[5]),
        NamedColor::Cyan => rgb(ANSI_16[6]),
        NamedColor::White => rgb(ANSI_16[7]),
        NamedColor::BrightBlack => rgb(ANSI_16[8]),
        NamedColor::BrightRed => rgb(ANSI_16[9]),
        NamedColor::BrightGreen => rgb(ANSI_16[10]),
        NamedColor::BrightYellow => rgb(ANSI_16[11]),
        NamedColor::BrightBlue => rgb(ANSI_16[12]),
        NamedColor::BrightMagenta => rgb(ANSI_16[13]),
        NamedColor::BrightCyan => rgb(ANSI_16[14]),
        NamedColor::BrightWhite => rgb(ANSI_16[15]),
        NamedColor::Foreground | NamedColor::BrightForeground | NamedColor::DimForeground => {
            DEFAULT_FOREGROUND
        }
        NamedColor::Background => DEFAULT_BACKGROUND,
        // The remaining "dim" variants are approximated by darkening the
        // corresponding normal color. This mirrors the common terminal
        // behavior of rendering dim text at reduced intensity.
        NamedColor::DimBlack => darken(rgb(ANSI_16[0])),
        NamedColor::DimRed => darken(rgb(ANSI_16[1])),
        NamedColor::DimGreen => darken(rgb(ANSI_16[2])),
        NamedColor::DimYellow => darken(rgb(ANSI_16[3])),
        NamedColor::DimBlue => darken(rgb(ANSI_16[4])),
        NamedColor::DimMagenta => darken(rgb(ANSI_16[5])),
        NamedColor::DimCyan => darken(rgb(ANSI_16[6])),
        NamedColor::DimWhite => darken(rgb(ANSI_16[7])),
        _ => DEFAULT_FOREGROUND,
    }
}

/// Reduce the intensity of a color by ~50% (used for the "dim" SGR attribute).
fn darken(color: Color32) -> Color32 {
    Color32::from_rgb(color.r() / 2, color.g() / 2, color.b() / 2)
}

/// Convert any `alacritty_terminal` cell color into an `egui::Color32`.
///
/// Handles all three representations:
///   * `Color::Spec(Rgb)` — 24-bit truecolor, passed through.
///   * `Color::Indexed(u8)` — the 256-entry xterm palette.
///   * `Color::Named(NamedColor)` — the 16 ANSI colors + special slots.
pub fn ansi_color_to_color32(color: Color) -> Color32 {
    match color {
        Color::Spec(Rgb { r, g, b }) => Color32::from_rgb(r, g, b),
        Color::Indexed(index) => ansi256_color(index),
        Color::Named(name) => named_color(name),
    }
}

fn rgb([r, g, b]: [u8; 3]) -> Color32 {
    Color32::from_rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truecolor_passthrough() {
        let color = ansi_color_to_color32(Color::Spec(Rgb { r: 1, g: 2, b: 3 }));
        assert_eq!(color, Color32::from_rgb(1, 2, 3));
        let white = ansi_color_to_color32(Color::Spec(Rgb {
            r: 255,
            g: 255,
            b: 255,
        }));
        assert_eq!(white, Color32::WHITE);
    }

    #[test]
    fn ansi_16_indexed_colors() {
        assert_eq!(ansi256_color(0), Color32::from_rgb(0x18, 0x18, 0x18));
        assert_eq!(ansi256_color(1), Color32::from_rgb(0xac, 0x42, 0x42));
        assert_eq!(ansi256_color(7), Color32::from_rgb(0xd8, 0xd8, 0xd8));
        assert_eq!(ansi256_color(15), Color32::from_rgb(0xf8, 0xf8, 0xf8));
    }

    #[test]
    fn ansi_256_cube_boundaries() {
        // First cube color (16) = (0,0,0).
        assert_eq!(ansi256_color(16), Color32::from_rgb(0, 0, 0));
        // 21 = 16 + 5 -> (0,0,5) -> blue 255.
        assert_eq!(ansi256_color(21), Color32::from_rgb(0, 0, 255));
        // 231 = 16 + 215 -> (5,5,5) -> white.
        assert_eq!(ansi256_color(231), Color32::from_rgb(255, 255, 255));
    }

    #[test]
    fn ansi_256_grayscale_ramp() {
        assert_eq!(ansi256_color(232), Color32::from_rgb(8, 8, 8));
        assert_eq!(ansi256_color(255), Color32::from_rgb(238, 238, 238));
    }

    #[test]
    fn cube_component_formula() {
        assert_eq!(cube_component(0), 0);
        assert_eq!(cube_component(1), 95);
        assert_eq!(cube_component(5), 255);
    }

    #[test]
    fn named_colors_map_to_palette() {
        assert_eq!(
            ansi_color_to_color32(Color::Named(NamedColor::Red)),
            Color32::from_rgb(0xac, 0x42, 0x42)
        );
        assert_eq!(
            ansi_color_to_color32(Color::Named(NamedColor::Foreground)),
            DEFAULT_FOREGROUND
        );
        assert_eq!(
            ansi_color_to_color32(Color::Named(NamedColor::Background)),
            DEFAULT_BACKGROUND
        );
    }

    #[test]
    fn indexed_and_named_roundtrip_through_single_entry() {
        // Every ANSI-16 named color must equal its indexed counterpart.
        let names = [
            NamedColor::Black,
            NamedColor::Red,
            NamedColor::Green,
            NamedColor::Yellow,
            NamedColor::Blue,
            NamedColor::Magenta,
            NamedColor::Cyan,
            NamedColor::White,
            NamedColor::BrightBlack,
            NamedColor::BrightRed,
            NamedColor::BrightGreen,
            NamedColor::BrightYellow,
            NamedColor::BrightBlue,
            NamedColor::BrightMagenta,
            NamedColor::BrightCyan,
            NamedColor::BrightWhite,
        ];
        for (index, name) in names.into_iter().enumerate() {
            assert_eq!(
                ansi_color_to_color32(Color::Named(name)),
                ansi_color_to_color32(Color::Indexed(index as u8)),
                "mismatch for named color index {index}"
            );
        }
    }
}
