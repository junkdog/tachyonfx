use ratatui::style::Color;
use crate::color_ext::ToRgbComponents;
use crate::lru_cache::LruCache;

/// Defines the color space to use for color interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
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

impl<const N: usize> LruCache<Color, (f32, f32, f32), N> {
    pub fn lerp(
        &mut self,
        from: &Color,
        to: &Color,
        color_space: ColorSpace,
        alpha: f32
    ) -> Color {
        use ColorSpace::*;

        let (a, b) = match color_space {
            Rgb => return ColorSpace::lerp_rgb(from.to_rgb(), to.to_rgb(), alpha),
            Hsl => (self.memoize(from, color_to_hsl), self.memoize(to, color_to_hsl)),
            Hsv => (self.memoize(from, color_to_hsv), self.memoize(to, color_to_hsv)),
        };

        match color_space {
            Hsl => ColorSpace::lerp_hsl(a, b, alpha),
            Hsv => ColorSpace::lerp_hsv(a, b, alpha),
            Rgb => unreachable!("Handled above"),
        }
    }
}

impl ColorSpace {
    pub fn lerp(&self, from: &Color, to: &Color, alpha: f32) -> Color {
        use ColorSpace::*;

        match self {
            Rgb => Self::lerp_rgb(from.to_rgb(), to.to_rgb(), alpha),
            Hsl => Self::lerp_hsl(color_to_hsl(from), color_to_hsl(to), alpha),
            Hsv => Self::lerp_hsv(color_to_hsv(from), color_to_hsv(to), alpha),
        }
    }

    fn lerp_rgb(
        (r1, g1, b1): (u8, u8, u8),
        (r2, g2, b2): (u8, u8, u8),
        alpha: f32
    ) -> Color {
        let alpha = (alpha * 0x1_0000 as f32) as u32;
        let inv_alpha = 0x1_0000 - alpha;

        let r = ((r1 as u32 * inv_alpha + r2 as u32 * alpha) >> 16) as u8;
        let g = ((g1 as u32 * inv_alpha + g2 as u32 * alpha) >> 16) as u8;
        let b = ((b1 as u32 * inv_alpha + b2 as u32 * alpha) >> 16) as u8;

        Color::Rgb(r, g, b)
    }

    fn lerp_hsv(
        (h1, s1, v1): (f32, f32, f32),
        (h2, s2, v2): (f32, f32, f32),
        alpha: f32
    ) -> Color {
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
        if h < 0.0 { h += 360.0; }
        if h >= 360.0 { h -= 360.0; }

        let s = s1 + (s2 - s1) * alpha;
        let v = v1 + (v2 - v1) * alpha;

        let (r, g, b) = hsv_to_rgb(h, s, v);
        Color::Rgb(r, g, b)
    }

    fn lerp_hsl(
        (h1, s1, l1): (f32, f32, f32),
        (h2, s2, l2): (f32, f32, f32),
        alpha: f32
    ) -> Color {
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
        if h < 0.0 { h += 360.0; }
        if h >= 360.0 { h -= 360.0; }

        let s = s1 + (s2 - s1) * alpha;
        let l = l1 + (l2 - l1) * alpha;

        let (r, g, b) = hsl_to_rgb(h, s, l);
        Color::Rgb(r, g, b)
    }
}

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    // Hue calculation
    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };

    let h = if h < 0.0 { h + 360.0 } else { h };

    // Saturation calculation
    let s = if max == 0.0 { 0.0 } else { delta / max };

    // Value calculation
    let v = max;

    (h, s * 100.0, v * 100.0)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let s = s / 100.0;
    let v = v / 100.0;
    let h = h % 360.0;

    if s <= 0.0 {
        return ((v * 255.0).round() as u8,
            (v * 255.0).round() as u8,
            (v * 255.0).round() as u8);
    }

    let h = h / 60.0;
    let i = h.floor() as i32;
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

    ((r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8)
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    // Lightness calculation
    let l = (max + min) / 2.0;

    // If delta is 0, the color is a shade of gray
    if delta == 0.0 {
        return (0.0, 0.0, l * 100.0);
    }

    // Saturation calculation
    let s = if l <= 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };

    // Hue calculation
    let h = if max == r {
        (g - b) / delta + (if g < b { 6.0 } else { 0.0 })
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };

    (h * 60.0, s * 100.0, l * 100.0)
}

pub(crate) fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let h = h % 360.0;
    let s = s / 100.0;
    let l = l / 100.0;

    // If saturation is 0, color is a shade of gray
    if s == 0.0 {
        let gray = (l * 255.0).round() as u8;
        return (gray, gray, gray);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };

    let p = 2.0 * l - q;

    let to_rgb_component = |t: f32| -> u8 {
        let t = if t < 0.0 {
            t + 1.0
        } else if t > 1.0 {
            t - 1.0
        } else {
            t
        };

        let value = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };

        (value * 255.0).round() as u8
    };

    let h = h / 360.0;

    let r = to_rgb_component(h + 1.0/3.0);
    let g = to_rgb_component(h);
    let b = to_rgb_component(h - 1.0/3.0);

    (r, g, b)
}


#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to assert approximate equality for floats
    fn assert_approx_eq(a: f32, b: f32, epsilon: f32) {
        assert!((a - b).abs() < epsilon, "Expected {} to be approximately equal to {}", a, b);
    }

    // Helper function to assert approximate equality for RGB values
    fn assert_rgb_eq(a: (u8, u8, u8), b: (u8, u8, u8),) {
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
                    assert_rgb_eq(original, rgb_from_hsv); // Allow 1 unit difference due to rounding
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
        // At 50%, we should get purple (HSL interpolation goes the shortest way around the color wheel)
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
}