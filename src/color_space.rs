use ratatui_core::style::Color;

use crate::{color_ext::ToRgbComponents, math};

/// Defines the color space to use for color interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorSpace {
    /// Linear RGB interpolation (fastest but not perceptually uniform)
    Rgb,
    /// HSL interpolation (default - balance of performance and perceptual quality)
    #[default]
    Hsl,
    /// HSV interpolation (similar to HSL but different perceptual model)
    Hsv,
}

/// Converts HSL (Hue, Saturation, Lightness) values to a ratatui Color.
///
/// # Arguments
/// * `h` - Hue value in degrees (0-360)
/// * `s` - Saturation percentage (0-100)
/// * `l` - Lightness percentage (0-100)
///
/// # Returns
/// A ratatui Color in RGB format
pub fn color_from_hsl(h: f32, s: f32, l: f32) -> Color {
    let (r, g, b) = hsl_to_rgb(h, s, l);
    Color::Rgb(r, g, b)
}

/// Converts HSV (Hue, Saturation, Value) values to a ratatui Color.
///
/// # Arguments
/// * `h` - Hue value in degrees (0-360)
/// * `s` - Saturation percentage (0-100)
/// * `v` - Value/brightness percentage (0-100)
///
/// # Returns
/// A ratatui Color in RGB format
pub fn color_from_hsv(h: f32, s: f32, v: f32) -> Color {
    let (r, g, b) = hsv_to_rgb(h, s, v);
    Color::Rgb(r, g, b)
}

/// Converts a ratatui Color to HSV (Hue, Saturation, Value) components.
///
/// # Arguments
/// * `color` - The source Color to convert
///
/// # Returns
/// A tuple of (hue, saturation, value) where:
/// * hue is in degrees (0-360)
/// * saturation is a percentage (0-100)
/// * value is a percentage (0-100)
pub fn color_to_hsv(color: &Color) -> (f32, f32, f32) {
    let (r, g, b) = color.to_rgb();
    rgb_to_hsv(r, g, b)
}

/// Converts a ratatui Color to HSL (Hue, Saturation, Lightness) components.
///
/// # Arguments
/// * `color` - The source Color to convert
///
/// # Returns
/// A tuple of (hue, saturation, lightness) where:
/// * hue is in degrees (0-360)
/// * saturation is a percentage (0-100)
/// * lightness is a percentage (0-100)
pub fn color_to_hsl(color: &Color) -> (f32, f32, f32) {
    let (r, g, b) = color.to_rgb();
    rgb_to_hsl(r, g, b)
}

/// Darken: scale toward 0. Lighten: lerp toward 255.
/// `amount`: -1.0 (black) to +1.0 (white).
#[inline]
fn adjust_lightness_linear(r: u8, g: u8, b: u8, amount: f32) -> (u8, u8, u8) {
    if amount >= 0.0 {
        // lerp toward white: c' = c + (255 - c) * amount
        let a = (amount * 256.0) as u32;
        let r = r as u32 + (((255 - r as u32) * a) >> 8);
        let g = g as u32 + (((255 - g as u32) * a) >> 8);
        let b = b as u32 + (((255 - b as u32) * a) >> 8);
        (r as u8, g as u8, b as u8)
    } else {
        // scale toward black: c' = c * (1 + amount)
        let factor = ((1.0 + amount) * 256.0) as u32;
        let r = (r as u32 * factor) >> 8;
        let g = (g as u32 * factor) >> 8;
        let b = (b as u32 * factor) >> 8;
        (r as u8, g as u8, b as u8)
    }
}

/// Adjust saturation by lerping toward BT.601 weighted luminance.
/// `factor`: 0.0 = fully desaturated, 1.0 = original, >1.0 = oversaturated.
#[inline]
fn adjust_saturation_weighted(r: u8, g: u8, b: u8, factor: f32) -> (u8, u8, u8) {
    // BT.601: 77/256 = 0.299, 150/256 = 0.586, 29/256 = 0.113
    let lum = ((r as u32 * 77 + g as u32 * 150 + b as u32 * 29) >> 8) as i32;
    let f = (factor * 256.0) as i32;

    // c' = lum + (c - lum) * factor
    let r = (lum + (((r as i32 - lum) * f) >> 8)).clamp(0, 255) as u8;
    let g = (lum + (((g as i32 - lum) * f) >> 8)).clamp(0, 255) as u8;
    let b = (lum + (((b as i32 - lum) * f) >> 8)).clamp(0, 255) as u8;
    (r, g, b)
}

/// Lerp a value toward 0 (negative amount) or toward `max` (positive amount).
/// `amount`: -1.0 to +1.0. Result is clamped to 0..max.
#[inline]
fn lerp_toward_extreme(value: f32, amount: f32, max: f32) -> f32 {
    if amount >= 0.0 {
        value + (max - value) * amount
    } else {
        value + value * amount
    }
}

impl ColorSpace {
    /// Adjust saturation of a color.
    /// `factor`: 0.0 = fully desaturated, 1.0 = original, >1.0 = oversaturated.
    pub fn saturate(&self, color: &Color, factor: f32) -> Color {
        match self {
            ColorSpace::Rgb => {
                let (r, g, b) = color.to_rgb();
                let (r, g, b) = adjust_saturation_weighted(r, g, b, factor);
                Color::Rgb(r, g, b)
            },
            ColorSpace::Hsl => {
                let (r, g, b) = color.to_rgb();
                let (h, s, l) = rgb_to_hsl(r, g, b);
                let s = (s * factor).clamp(0.0, 100.0);
                let (r, g, b) = hsl_to_rgb(h, s, l);
                Color::Rgb(r, g, b)
            },
            ColorSpace::Hsv => {
                let (r, g, b) = color.to_rgb();
                let (h, s, v) = rgb_to_hsv(r, g, b);
                let s = (s * factor).clamp(0.0, 100.0);
                let (r, g, b) = hsv_to_rgb(h, s, v);
                Color::Rgb(r, g, b)
            },
        }
    }

    /// Adjust lightness/brightness of a color.
    /// `amount`: -1.0 (black) to +1.0 (white). Lerps toward the extreme.
    pub fn lighten(&self, color: &Color, amount: f32) -> Color {
        let amount = amount.clamp(-1.0, 1.0);
        match self {
            ColorSpace::Rgb => {
                let (r, g, b) = color.to_rgb();
                let (r, g, b) = adjust_lightness_linear(r, g, b, amount);
                Color::Rgb(r, g, b)
            },
            ColorSpace::Hsl => {
                let (r, g, b) = color.to_rgb();
                let (h, s, l) = rgb_to_hsl(r, g, b);
                let l = lerp_toward_extreme(l, amount, 100.0);
                let (r, g, b) = hsl_to_rgb(h, s, l);
                Color::Rgb(r, g, b)
            },
            ColorSpace::Hsv => {
                let (r, g, b) = color.to_rgb();
                let (h, s, v) = rgb_to_hsv(r, g, b);
                let v = lerp_toward_extreme(v, amount, 100.0);
                let (r, g, b) = hsv_to_rgb(h, s, v);
                Color::Rgb(r, g, b)
            },
        }
    }

    pub fn lerp(&self, from: &Color, to: &Color, alpha: f32) -> Color {
        use ColorSpace::*;

        let alpha = alpha.clamp(0.0, 1.0);
        if alpha == 0.0 {
            return *from;
        } else if alpha == 1.0 {
            return *to;
        }

        match self {
            Rgb => Self::lerp_rgb(from.to_rgb(), to.to_rgb(), alpha),
            Hsl => Self::lerp_hsl(color_to_hsl(from), color_to_hsl(to), alpha),
            Hsv => Self::lerp_hsv(color_to_hsv(from), color_to_hsv(to), alpha),
        }
    }

    fn lerp_rgb((r1, g1, b1): (u8, u8, u8), (r2, g2, b2): (u8, u8, u8), alpha: f32) -> Color {
        let alpha = (alpha * 0x1_0000 as f32) as u32;
        let inv_alpha = 0x1_0000 - alpha;

        let lerp =
            |c1: u8, c2: u8| -> u8 { ((c1 as u32 * inv_alpha + c2 as u32 * alpha) >> 16) as u8 };

        Color::Rgb(lerp(r1, r2), lerp(g1, g2), lerp(b1, b2))
    }

    fn lerp_hsv((h1, s1, v1): (f32, f32, f32), (h2, s2, v2): (f32, f32, f32), alpha: f32) -> Color {
        // Calculate hue difference, taking the shortest path
        let mut h_diff = h2 - h1;

        // Adjust to take the shortest path around the color wheel
        if h_diff > 180.0 {
            h_diff -= 360.0;
        } else if h_diff < -180.0 {
            h_diff += 360.0;
        }

        // Calculate the interpolated hue
        let mut h = h1 + h_diff * alpha;
        // Normalize to 0-360 range
        if h < 0.0 {
            h += 360.0;
        }
        if h >= 360.0 {
            h -= 360.0;
        }

        let s = s1 + (s2 - s1) * alpha;
        let v = v1 + (v2 - v1) * alpha;

        let (r, g, b) = hsv_to_rgb(h, s, v);
        Color::Rgb(r, g, b)
    }

    fn lerp_hsl((h1, s1, l1): (f32, f32, f32), (h2, s2, l2): (f32, f32, f32), alpha: f32) -> Color {
        // Calculate hue difference, taking the shortest path
        let mut h_diff = h2 - h1;

        // Adjust to take the shortest path around the color wheel
        if h_diff > 180.0 {
            h_diff -= 360.0;
        } else if h_diff < -180.0 {
            h_diff += 360.0;
        }

        // Calculate the interpolated hue
        let mut h = h1 + h_diff * alpha;
        // Normalize to 0-360 range
        if h < 0.0 {
            h += 360.0;
        }
        if h >= 360.0 {
            h -= 360.0;
        }

        let s = s1 + (s2 - s1) * alpha;
        let l = l1 + (l2 - l1) * alpha;

        let (r, g, b) = hsl_to_rgb(h, s, l);
        Color::Rgb(r, g, b)
    }
}

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min; // u8

    if delta == 0 {
        return (0.0, 0.0, max as f32 * (100.0 / 255.0));
    }

    let v = max as f32 * (100.0 / 255.0);

    // Two independent divisions — CPU pipelines these
    let inv_delta = 1.0 / delta as f32;
    let inv_max = 1.0 / max as f32;

    let s = delta as f32 * 100.0 * inv_max;

    // Branchless hue via arithmetic masks
    let hr = (g as f32 - b as f32) * inv_delta;
    let hg = (b as f32 - r as f32) * inv_delta + 2.0;
    let hb = (r as f32 - g as f32) * inv_delta + 4.0;

    let r_mask = ((r >= g) as u8 & (r >= b) as u8) as f32;
    let g_mask = (g >= b) as u8 as f32 * (1.0 - r_mask);
    let b_mask = 1.0 - r_mask - g_mask;

    let hr = hr + (g < b) as u8 as f32 * 6.0;
    let h = (r_mask * hr + g_mask * hg + b_mask * hb) * 60.0;

    (h, s, v)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let s = s / 100.0;
    let v = v / 100.0;
    let h = ((h % 360.0) + 360.0) % 360.0;

    if s <= 0.0 {
        return (
            math::round(v * 255.0) as u8,
            math::round(v * 255.0) as u8,
            math::round(v * 255.0) as u8,
        );
    }

    let h = h / 60.0;
    let i = math::floor(h) as i32;
    let f = h - i as f32;

    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };

    (
        math::round(r * 255.0) as u8,
        math::round(g * 255.0) as u8,
        math::round(b * 255.0) as u8,
    )
}

pub(crate) fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min; // u8
    let sum = max as u16 + min as u16; // 0..510

    if delta == 0 {
        return (0.0, 0.0, sum as f32 * (50.0 / 255.0));
    }

    let l = sum as f32 * (50.0 / 255.0);

    // Saturation denom computed in integer: 255 - |sum - 255|
    let abs_diff = sum.abs_diff(255);
    let denom = (255 - abs_diff) as f32;

    // Two independent divisions — CPU pipelines these
    let inv_delta = 1.0 / delta as f32;
    let inv_denom = 1.0 / denom;

    let s = delta as f32 * 100.0 * inv_denom;

    // Hue: channel diffs as integer → single multiply by precomputed inv_delta
    let hr = (g as f32 - b as f32) * inv_delta;
    let hg = (b as f32 - r as f32) * inv_delta + 2.0;
    let hb = (r as f32 - g as f32) * inv_delta + 4.0;

    // Branchless masks using exact u8 comparisons
    let r_mask = ((r >= g) as u8 & (r >= b) as u8) as f32;
    let g_mask = (g >= b) as u8 as f32 * (1.0 - r_mask);
    let b_mask = 1.0 - r_mask - g_mask;

    let hr = hr + (g < b) as u8 as f32 * 6.0;
    let h = (r_mask * hr + g_mask * hg + b_mask * hb) * 60.0;

    (h, s, l)
}

pub(crate) fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let s = s / 100.0;
    let l = l / 100.0;

    if s == 0.0 {
        let gray = math::round(l * 255.0) as u8;
        return (gray, gray, gray);
    }

    let h = ((h % 360.0) + 360.0) % 360.0 / 60.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let m = l - c * 0.5;

    let sector = h as u32;
    let f = h - sector as f32;
    let h2 = (sector & 1) as f32 + f;
    let x = c * (1.0 - (h2 - 1.0).abs());

    let (r, g, b) = match sector {
        0 => (c + m, x + m, m),
        1 => (x + m, c + m, m),
        2 => (m, c + m, x + m),
        3 => (m, x + m, c + m),
        4 => (x + m, m, c + m),
        _ => (c + m, m, x + m),
    };

    (
        math::round(r * 255.0) as u8,
        math::round(g * 255.0) as u8,
        math::round(b * 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to assert approximate equality for floats
    fn assert_approx_eq(a: f32, b: f32, epsilon: f32) {
        assert!(
            (a - b).abs() < epsilon,
            "Expected {a} to be approximately equal to {b}"
        );
    }

    // Helper function to assert approximate equality for RGB values
    fn assert_rgb_eq(a: (u8, u8, u8), b: (u8, u8, u8)) {
        let a = Color::Rgb(a.0, a.1, a.2);
        let b = Color::Rgb(b.0, b.1, b.2);
        assert_eq!(a, b);
    }

    #[test]
    fn test_rgb_to_hsl() {
        // Test primary colors
        let (h, s, l) = rgb_to_hsl(255, 0, 0); // Red
        assert_approx_eq(h, 0.0, 0.1);
        assert_approx_eq(s, 100.0, 0.1);
        assert_approx_eq(l, 50.0, 0.1);

        let (h, s, l) = rgb_to_hsl(0, 255, 0); // Green
        assert_approx_eq(h, 120.0, 0.1);
        assert_approx_eq(s, 100.0, 0.1);
        assert_approx_eq(l, 50.0, 0.1);

        let (h, s, l) = rgb_to_hsl(0, 0, 255); // Blue
        assert_approx_eq(h, 240.0, 0.1);
        assert_approx_eq(s, 100.0, 0.1);
        assert_approx_eq(l, 50.0, 0.1);

        // Test secondary colors
        let (h, s, l) = rgb_to_hsl(255, 255, 0); // Yellow
        assert_approx_eq(h, 60.0, 0.1);
        assert_approx_eq(s, 100.0, 0.1);
        assert_approx_eq(l, 50.0, 0.1);

        // Test black and white
        let (h, s, l) = rgb_to_hsl(0, 0, 0); // Black
        assert_approx_eq(h, 0.0, 0.1);
        assert_approx_eq(s, 0.0, 0.1);
        assert_approx_eq(l, 0.0, 0.1);

        let (h, s, l) = rgb_to_hsl(255, 255, 255); // White
        assert_approx_eq(h, 0.0, 0.1);
        assert_approx_eq(s, 0.0, 0.1);
        assert_approx_eq(l, 100.0, 0.1);

        // Test gray
        let (h, s, l) = rgb_to_hsl(128, 128, 128); // Gray
        assert_approx_eq(h, 0.0, 0.1);
        assert_approx_eq(s, 0.0, 0.1);
        assert_approx_eq(l, 50.0, 0.2);
    }

    #[test]
    fn test_hsl_to_rgb() {
        // Test primary colors
        let (r, g, b) = hsl_to_rgb(0.0, 100.0, 50.0); // Red
        assert_eq!(r, 255);
        assert_eq!(g, 0);
        assert_eq!(b, 0);

        let (r, g, b) = hsl_to_rgb(120.0, 100.0, 50.0); // Green
        assert_eq!(r, 0);
        assert_eq!(g, 255);
        assert_eq!(b, 0);

        let (r, g, b) = hsl_to_rgb(240.0, 100.0, 50.0); // Blue
        assert_eq!(r, 0);
        assert_eq!(g, 0);
        assert_eq!(b, 255);

        // Test black and white
        let (r, g, b) = hsl_to_rgb(0.0, 0.0, 0.0); // Black
        assert_eq!(r, 0);
        assert_eq!(g, 0);
        assert_eq!(b, 0);

        let (r, g, b) = hsl_to_rgb(0.0, 0.0, 100.0); // White
        assert_eq!(r, 255);
        assert_eq!(g, 255);
        assert_eq!(b, 255);

        // Test gray
        let (r, g, b) = hsl_to_rgb(0.0, 0.0, 50.0); // Gray
        assert_eq!(r, 128);
        assert_eq!(g, 128);
        assert_eq!(b, 128);
    }

    #[test]
    fn test_rgb_to_hsv() {
        // Test primary colors
        let (h, s, v) = rgb_to_hsv(255, 0, 0); // Red
        assert_approx_eq(h, 0.0, 0.1);
        assert_approx_eq(s, 100.0, 0.1);
        assert_approx_eq(v, 100.0, 0.1);

        let (h, s, v) = rgb_to_hsv(0, 255, 0); // Green
        assert_approx_eq(h, 120.0, 0.1);
        assert_approx_eq(s, 100.0, 0.1);
        assert_approx_eq(v, 100.0, 0.1);

        let (h, s, v) = rgb_to_hsv(0, 0, 255); // Blue
        assert_approx_eq(h, 240.0, 0.1);
        assert_approx_eq(s, 100.0, 0.1);
        assert_approx_eq(v, 100.0, 0.1);

        // Test black and white
        let (h, s, v) = rgb_to_hsv(0, 0, 0); // Black
        assert_approx_eq(h, 0.0, 0.1);
        assert_approx_eq(s, 0.0, 0.1);
        assert_approx_eq(v, 0.0, 0.1);

        let (h, s, v) = rgb_to_hsv(255, 255, 255); // White
        assert_approx_eq(h, 0.0, 0.1);
        assert_approx_eq(s, 0.0, 0.1);
        assert_approx_eq(v, 100.0, 0.1);

        // Test gray
        let (h, s, v) = rgb_to_hsv(128, 128, 128); // Gray
        assert_approx_eq(h, 0.0, 0.1);
        assert_approx_eq(s, 0.0, 0.1);
        assert_approx_eq(v, 50.2, 0.1); // Note: 128/255 ≈ 0.502
    }

    #[test]
    fn test_hsv_to_rgb() {
        // Test primary colors
        let (r, g, b) = hsv_to_rgb(0.0, 100.0, 100.0); // Red
        assert_eq!(r, 255);
        assert_eq!(g, 0);
        assert_eq!(b, 0);

        let (r, g, b) = hsv_to_rgb(120.0, 100.0, 100.0); // Green
        assert_eq!(r, 0);
        assert_eq!(g, 255);
        assert_eq!(b, 0);

        let (r, g, b) = hsv_to_rgb(240.0, 100.0, 100.0); // Blue
        assert_eq!(r, 0);
        assert_eq!(g, 0);
        assert_eq!(b, 255);

        // Test black and white
        let (r, g, b) = hsv_to_rgb(0.0, 0.0, 0.0); // Black
        assert_eq!(r, 0);
        assert_eq!(g, 0);
        assert_eq!(b, 0);

        let (r, g, b) = hsv_to_rgb(0.0, 0.0, 100.0); // White
        assert_eq!(r, 255);
        assert_eq!(g, 255);
        assert_eq!(b, 255);

        // Test gray
        let (r, g, b) = hsv_to_rgb(0.0, 0.0, 50.0); // Gray
        assert_eq!(r, 128);
        assert_eq!(g, 128);
        assert_eq!(b, 128);
    }

    #[test]
    fn test_round_trip_conversions() {
        // Test a range of RGB colors for round-trip conversion
        for r in [0, 64, 128, 192, 255].iter() {
            for g in [0, 64, 128, 192, 255].iter() {
                for b in [0, 64, 128, 192, 255].iter() {
                    let original = (*r, *g, *b);

                    // RGB -> HSL -> RGB
                    let (h, s, l) = rgb_to_hsl(*r, *g, *b);
                    let rgb_from_hsl = hsl_to_rgb(h, s, l);
                    assert_rgb_eq(original, rgb_from_hsl); // Allow 1 unit difference due to rounding

                    // RGB -> HSV -> RGB
                    let (h, s, v) = rgb_to_hsv(*r, *g, *b);
                    let rgb_from_hsv = hsv_to_rgb(h, s, v);
                    assert_rgb_eq(original, rgb_from_hsv); // Allow 1 unit difference due
                                                           // to rounding
                }
            }
        }
    }

    #[test]
    fn test_interpolate_rgb() {
        let from = Color::Rgb(0, 0, 0); // Black
        let to = Color::Rgb(255, 255, 255); // White

        // Test 0%, 25%, 50%, 75%, 100% interpolation
        let result = ColorSpace::Rgb.lerp(&from, &to, 0.0);
        assert_eq!(result, Color::Rgb(0, 0, 0));

        let result = ColorSpace::Rgb.lerp(&from, &to, 0.25);
        assert_eq!(result, Color::Rgb(63, 63, 63));

        let result = ColorSpace::Rgb.lerp(&from, &to, 0.5);
        assert_eq!(result, Color::Rgb(127, 127, 127));

        let result = ColorSpace::Rgb.lerp(&from, &to, 0.75);
        assert_eq!(result, Color::Rgb(191, 191, 191));

        let result = ColorSpace::Rgb.lerp(&from, &to, 1.0);
        assert_eq!(result, Color::Rgb(255, 255, 255));

        // Test with uneven colors
        let from = Color::Rgb(100, 150, 200);
        let to = Color::Rgb(200, 100, 50);

        let result = ColorSpace::Rgb.lerp(&from, &to, 0.5);
        assert_eq!(result, Color::Rgb(150, 125, 125));
    }

    #[test]
    fn test_interpolate_hsl() {
        // Test interpolating between red and blue
        // At 50%, we should get purple (HSL interpolation goes the shortest way around the color
        // wheel)
        let from = Color::Rgb(255, 0, 0); // Red
        let to = Color::Rgb(0, 0, 255); // Blue

        let result = ColorSpace::Hsl.lerp(&from, &to, 0.5);
        assert_rgb_eq(result.to_rgb(), (255, 0, 255)); // Purple-ish

        // Test interpolating across the color wheel (red to cyan)
        let from = Color::Rgb(255, 0, 0); // Red (0°)
        let to = Color::Rgb(0, 255, 255); // Cyan (180°)

        // At 50%, we should get near green (HSL interpolation)
        let result = ColorSpace::Hsl.lerp(&from, &to, 0.5);
        let (h, _, _) = rgb_to_hsl(result.to_rgb().0, result.to_rgb().1, result.to_rgb().2);
        assert_approx_eq(h, 90.0, 5.0); // Near yellow-green

        // Test interpolating between fully saturated and desaturated
        let from = Color::Rgb(255, 0, 0); // Red (100% saturation)
        let to = Color::Rgb(128, 128, 128); // Gray (0% saturation)

        let result = ColorSpace::Hsl.lerp(&from, &to, 0.5);
        let (_, s, _) = rgb_to_hsl(result.to_rgb().0, result.to_rgb().1, result.to_rgb().2);
        assert_approx_eq(s, 50.0, 5.0); // 50% saturation
    }

    #[test]
    fn test_interpolate_hsv() {
        // Test interpolating between red and blue
        let from = Color::Rgb(255, 0, 0); // Red (0°)
        let to = Color::Rgb(0, 0, 255); // Blue (240°)

        // At 50%, we should get magenta (300°) which is halfway on the shortest path
        let result = ColorSpace::Hsv.lerp(&from, &to, 0.5);

        // Check that we get a color at 300° (magenta)
        let (h, s, v) = rgb_to_hsv(result.to_rgb().0, result.to_rgb().1, result.to_rgb().2);
        assert_approx_eq(h, 300.0, 5.0); // Should be near 300° (magenta)
        assert_approx_eq(s, 100.0, 0.1); // Should maintain 100% saturation
        assert_approx_eq(v, 100.0, 0.1); // Should maintain 100% value

        // The resulting color should be magenta-ish
        assert_rgb_eq(result.to_rgb(), (255, 0, 255));
    }

    #[test]
    fn test_edge_cases() {
        // Test edge cases where different color spaces might behave differently

        // Complementary colors (red to cyan)
        let from = Color::Rgb(255, 0, 0);
        let to = Color::Rgb(0, 255, 255);

        let rgb_mid = ColorSpace::Rgb.lerp(&from, &to, 0.5);
        let hsl_mid = ColorSpace::Hsl.lerp(&from, &to, 0.5);

        // RGB interpolation gives gray (127, 127, 127)
        // HSL interpolation gives yellowy-green
        assert_eq!(rgb_mid.to_rgb().0, rgb_mid.to_rgb().1);
        assert_eq!(rgb_mid.to_rgb().1, rgb_mid.to_rgb().2);

        let (hsl_h, _, _) = rgb_to_hsl(hsl_mid.to_rgb().0, hsl_mid.to_rgb().1, hsl_mid.to_rgb().2);
        assert_approx_eq(hsl_h, 90.0, 5.0);
    }

    #[test]
    fn test_lighten_all_color_spaces() {
        let spaces = [ColorSpace::Rgb, ColorSpace::Hsl, ColorSpace::Hsv];
        let red = Color::Rgb(200, 50, 50);
        let gray = Color::Rgb(128, 128, 128);

        for cs in spaces {
            // amount=0 returns original
            assert_eq!(
                cs.lighten(&red, 0.0).to_rgb(),
                red.to_rgb(),
                "{cs:?}: amount=0 should return original"
            );

            // amount=1.0: RGB/HSL produce white, HSV maxes brightness (stays saturated)
            let (r, g, b) = cs.lighten(&red, 1.0).to_rgb();
            match cs {
                ColorSpace::Rgb | ColorSpace::Hsl => {
                    assert!(
                        r >= 254 && g >= 254 && b >= 254,
                        "{cs:?}: amount=1.0 should produce white, got ({r}, {g}, {b})"
                    );
                },
                ColorSpace::Hsv => {
                    assert_eq!(r, 255, "{cs:?}: amount=1.0 should max out dominant channel");
                },
            }

            // amount=-1.0 produces black
            let (r, g, b) = cs.lighten(&red, -1.0).to_rgb();
            assert!(
                r <= 1 && g <= 1 && b <= 1,
                "{cs:?}: amount=-1.0 should produce black, got ({r}, {g}, {b})"
            );

            // positive amount increases perceived brightness
            let orig = gray.to_rgb();
            let lighter = cs.lighten(&gray, 0.5).to_rgb();
            assert!(
                lighter.0 > orig.0 && lighter.1 > orig.1 && lighter.2 > orig.2,
                "{cs:?}: lighten(0.5) should increase all channels for gray"
            );

            // negative amount decreases perceived brightness
            let darker = cs.lighten(&gray, -0.5).to_rgb();
            assert!(
                darker.0 < orig.0 && darker.1 < orig.1 && darker.2 < orig.2,
                "{cs:?}: lighten(-0.5) should decrease all channels for gray"
            );
        }
    }

    #[test]
    fn test_saturate_all_color_spaces() {
        let spaces = [ColorSpace::Rgb, ColorSpace::Hsl, ColorSpace::Hsv];
        let teal = Color::Rgb(50, 180, 160);

        for cs in spaces {
            // factor=1.0 returns original
            assert_eq!(
                cs.saturate(&teal, 1.0).to_rgb(),
                teal.to_rgb(),
                "{cs:?}: factor=1.0 should return original"
            );

            // factor=0.0 produces grayscale
            let (r, g, b) = cs.saturate(&teal, 0.0).to_rgb();
            let max_diff = (r as i32 - g as i32)
                .abs()
                .max((g as i32 - b as i32).abs())
                .max((r as i32 - b as i32).abs());
            assert!(
                max_diff <= 1,
                "{cs:?}: factor=0.0 should produce grayscale, got ({r}, {g}, {b})"
            );

            // factor=0.0 preserves approximate luminance (not too dark/bright)
            // HSL/HSV use different gray-point definitions than BT.601,
            // so allow wider tolerance for those spaces
            let gray_lum = (r as u32 * 77 + g as u32 * 150 + b as u32 * 29) >> 8;
            let orig = teal.to_rgb();
            let orig_lum = (orig.0 as u32 * 77 + orig.1 as u32 * 150 + orig.2 as u32 * 29) >> 8;
            let tolerance = match cs {
                ColorSpace::Rgb => 5,  // BT.601 weights — tight
                ColorSpace::Hsl => 30, // (max+min)/2 gray point
                ColorSpace::Hsv => 50, // max(r,g,b) gray point — furthest from BT.601
            };
            assert!((gray_lum as i32 - orig_lum as i32).unsigned_abs() < tolerance,
                "{cs:?}: desaturated luminance ({gray_lum}) should be close to original ({orig_lum})");

            // factor < 1.0 moves channels closer together
            let desat = cs.saturate(&teal, 0.5).to_rgb();
            let orig_spread = orig.0.abs_diff(orig.1) as u32
                + orig.1.abs_diff(orig.2) as u32
                + orig.0.abs_diff(orig.2) as u32;
            let desat_spread = desat.0.abs_diff(desat.1) as u32
                + desat.1.abs_diff(desat.2) as u32
                + desat.0.abs_diff(desat.2) as u32;
            assert!(
                desat_spread < orig_spread,
                "{cs:?}: factor=0.5 should reduce channel spread ({desat_spread} < {orig_spread})"
            );

            // factor > 1.0 pushes channels further apart
            let oversat = cs.saturate(&teal, 1.5).to_rgb();
            let oversat_spread = oversat.0.abs_diff(oversat.1) as u32
                + oversat.1.abs_diff(oversat.2) as u32
                + oversat.0.abs_diff(oversat.2) as u32;
            assert!(oversat_spread > orig_spread,
                "{cs:?}: factor=1.5 should increase channel spread ({oversat_spread} > {orig_spread})");
        }
    }

    #[test]
    fn test_lighten_clamps_out_of_range() {
        let color = Color::Rgb(100, 150, 200);
        for cs in [ColorSpace::Rgb, ColorSpace::Hsl, ColorSpace::Hsv] {
            assert_eq!(
                cs.lighten(&color, 2.0).to_rgb(),
                cs.lighten(&color, 1.0).to_rgb(),
                "{cs:?}: amount > 1.0 should clamp to 1.0"
            );
            assert_eq!(
                cs.lighten(&color, -5.0).to_rgb(),
                cs.lighten(&color, -1.0).to_rgb(),
                "{cs:?}: amount < -1.0 should clamp to -1.0"
            );
        }
    }

    #[test]
    fn test_hsl_to_rgb_negative_hue() {
        // A negative hue should wrap around the color wheel.
        // -30° is equivalent to 330° (magenta-pink).
        let from_negative = hsl_to_rgb(-30.0, 100.0, 50.0);
        let from_positive = hsl_to_rgb(330.0, 100.0, 50.0);
        assert_eq!(
            from_negative, from_positive,
            "hsl_to_rgb(-30°) should equal hsl_to_rgb(330°)"
        );
    }

    #[test]
    fn test_hsv_to_rgb_negative_hue() {
        // A negative hue should wrap around the color wheel.
        // -90° is equivalent to 270° (blue-violet).
        let from_negative = hsv_to_rgb(-90.0, 100.0, 100.0);
        let from_positive = hsv_to_rgb(270.0, 100.0, 100.0);
        assert_eq!(
            from_negative, from_positive,
            "hsv_to_rgb(-90°) should equal hsv_to_rgb(270°)"
        );
    }

    #[test]
    fn test_lighten_saturate_gray_invariance() {
        // Gray has no saturation — saturate should be a no-op
        let gray = Color::Rgb(128, 128, 128);
        for cs in [ColorSpace::Rgb, ColorSpace::Hsl, ColorSpace::Hsv] {
            for factor in [0.0_f32, 0.5, 1.0, 1.5, 2.0] {
                let result = cs.saturate(&gray, factor).to_rgb();
                let (r, g, b) = result;
                let max_diff = (r as i32 - 128)
                    .abs()
                    .max((g as i32 - 128).abs())
                    .max((b as i32 - 128).abs());
                assert!(max_diff <= 1,
                    "{cs:?}: saturate({factor}) on gray should be ~(128,128,128), got ({r},{g},{b})");
            }
        }
    }
}
