use ratatui_core::style::Color;

pub trait ToRgbComponents {
    fn to_rgb(&self) -> (u8, u8, u8);
}

impl ToRgbComponents for Color {
    fn to_rgb(&self) -> (u8, u8, u8) {
        match self {
            Color::Rgb(r, g, b) => (*r, *g, *b),
            Color::Reset => (0, 0, 0),
            Color::Black => (0, 0, 0),
            Color::Red => (128, 0, 0),
            Color::Green => (0, 128, 0),
            Color::Yellow => (128, 128, 0),
            Color::Blue => (0, 0, 128),
            Color::Magenta => (128, 0, 128),
            Color::Cyan => (0, 128, 128),
            Color::Gray => (128, 128, 128),
            Color::DarkGray => (96, 96, 96),
            Color::LightRed => (255, 0, 0),
            Color::LightGreen => (0, 255, 0),
            Color::LightYellow => (255, 255, 0),
            Color::LightBlue => (0, 0, 255),
            Color::LightMagenta => (255, 0, 255),
            Color::LightCyan => (0, 255, 255),
            Color::White => (192, 192, 192),
            Color::Indexed(code) => {
                let rgb = indexed_color_to_rgb(*code);
                let r = ((rgb >> 16) & 0xFF) as u8;
                let g = ((rgb >> 8) & 0xFF) as u8;
                let b = (rgb & 0xFF) as u8;
                (r, g, b)
            },
        }
    }
}

#[deprecated(since = "0.16.0", note = "not considered core/useful")]
pub trait AsIndexedColor {
    fn as_indexed_color(&self) -> Color;
}

#[allow(deprecated)]
impl AsIndexedColor for Color {
    fn as_indexed_color(&self) -> Color {
        let (r, g, b) = self.to_rgb();

        // let c = colorsys::Rgb::from([r as f64, g as f64, b as f64]);
        // let ansi256 = colorsys::Ansi256::from(c);
        Color::Indexed(rgb_to_indexed_color(r, g, b))
    }
}

fn rgb_to_indexed_color(r: u8, g: u8, b: u8) -> u8 {
    // grayscale colors (232-255)
    if r == g && g == b && (8..=238).contains(&r) {
        return 232 + (r - 8) / 10;
    }

    let quantize = |val: u8| -> u8 {
        if val < 155 {
            if val < 48 {
                0
            } else if val < 115 {
                1
            } else {
                2
            }
        } else if val < 195 {
            3
        } else if val < 235 {
            4
        } else {
            5
        }
    };

    let r_idx = quantize(r);
    let g_idx = quantize(g);
    let b_idx = quantize(b);

    16 + r_idx * 36 + g_idx * 6 + b_idx
}

/// Converts an indexed color (0-255) to an RGB value.
fn indexed_color_to_rgb(index: u8) -> u32 {
    match index {
        // Basic 16 colors (0-15)
        0..=15 => {
            const BASIC_COLORS: [u32; 16] = [
                0x000000, // 0: black
                0xCD0000, // 1: red
                0x00CD00, // 2: green
                0xCDCD00, // 3: yellow
                0x0000EE, // 4: blue
                0xCD00CD, // 5: magenta
                0x00CDCD, // 6: cyan
                0xE5E5E5, // 7: white
                0x7F7F7F, // 8: bright Black
                0xFF0000, // 9: bright Red
                0x00FF00, // 10: bright Green
                0xFFFF00, // 11: bright Yellow
                0x5C5CFF, // 12: bright Blue
                0xFF00FF, // 13: bright Magenta
                0x00FFFF, // 14: bright Cyan
                0xFFFFFF, // 15: bright White
            ];
            BASIC_COLORS[index as usize]
        },

        // 216-color cube (16-231)
        16..=231 => {
            let cube_index = index - 16;
            let r = cube_index / 36;
            let g = (cube_index % 36) / 6;
            let b = cube_index % 6;

            // Convert 0-5 range to 0-255 RGB
            // Values: 0 -> 0, 1 -> 95, 2 -> 135, 3 -> 175, 4 -> 215, 5 -> 255
            let to_rgb = |n: u8| -> u32 {
                if n == 0 {
                    0
                } else {
                    55 + 40 * n as u32
                }
            };

            (to_rgb(r) << 16) | (to_rgb(g) << 8) | to_rgb(b)
        },

        // 24 grayscale colors (232-255)
        232..=255 => {
            let gray_index = index - 232;
            // linear interpolation from 8 to 238
            let gray = (8 + gray_index * 10) as u32;
            (gray << 16) | (gray << 8) | gray
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexed_color_roundtrip() {
        // skip basic colors (0-15) since they are not quantized
        for index in 16..=255u8 {
            let rgb_value = indexed_color_to_rgb(index);
            let r = ((rgb_value >> 16) & 0xFF) as u8;
            let g = ((rgb_value >> 8) & 0xFF) as u8;
            let b = (rgb_value & 0xFF) as u8;
            let back_to_index = rgb_to_indexed_color(r, g, b);

            // For the 216-color cube (16-231) and grayscale (232-255),
            // we should get exact roundtrip, except when cube colors are grayscale
            // and get remapped to the dedicated grayscale ramp
            match index {
                16..=231 => {
                    // Cube colors should roundtrip unless they're grayscale
                    if r == g && g == b && (8..=238).contains(&r) {
                        // Grayscale cube colors should map to grayscale ramp
                        assert!((232..=255).contains(&back_to_index),
                            "Grayscale cube color {index} -> ({r}, {g}, {b}) should map to grayscale ramp, got {back_to_index}");
                    } else {
                        // Non-grayscale cube colors should roundtrip exactly
                        assert_eq!(
                            index, back_to_index,
                            "Roundtrip failed for index {index}: {index} -> ({r}, {g}, {b}) -> {back_to_index}"
                        );
                    }
                },
                232..=255 => {
                    // Grayscale ramp should always roundtrip exactly
                    assert_eq!(
                        index, back_to_index,
                        "Roundtrip failed for index {index}: {index} -> ({r}, {g}, {b}) -> {back_to_index}"
                    );
                },
                _ => continue,
            }
        }
    }
}
