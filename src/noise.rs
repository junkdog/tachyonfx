//! Value noise and fractal Brownian motion.
//!
//! [`crate::wave`] provides periodic waveforms; this module provides aperiodic
//! ones. The distinction matters for ambient effects: a background driven by a
//! sine visibly repeats, and the eye picks the period out quickly. Noise does
//! not repeat, which is what makes an effect read as organic.
//!
//! Everything here is deterministic pseudo-noise seeded by position, not
//! randomness. The same coordinates always yield the same value, so a field is
//! reproducible, testable, and stable as the terminal is resized.
//!
//! ```
//! use tachyonfx::noise;
//!
//! // A value in 0.0..=1.0 for any point in the plane.
//! let v = noise::fbm2(3.5, 1.25, 3);
//! assert!((0.0..=1.0).contains(&v));
//! ```

use crate::math;

/// Robert Jenkins' 32-bit integer hash, mapped to `0.0..=1.0`.
///
/// Wrapping arithmetic throughout: this is a bit-mixing function, so overflow
/// is the mechanism rather than an error.
fn hash_noise(n: i64) -> f32 {
    let mut n = (n as u64 as u32).wrapping_add(0x1656_67b1);
    n = (n ^ (n >> 16)).wrapping_mul(0x045d_9f3b);
    n = (n ^ (n >> 16)).wrapping_mul(0x045d_9f3b);
    n ^= n >> 16;
    (n >> 16) as f32 / u16::MAX as f32
}

/// Hermite smoothstep, easing interpolation between lattice points so the field
/// has no visible grid.
fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// 1D value noise in `0.0..=1.0`.
#[must_use]
pub fn noise1(x: f32) -> f32 {
    let xi = math::floor(x) as i64;
    let xf = x - xi as f32;
    let a = hash_noise(xi);
    let b = hash_noise(xi + 1);
    a + smooth(xf) * (b - a)
}

/// 2D value noise in `0.0..=1.0`.
#[must_use]
pub fn noise2(x: f32, y: f32) -> f32 {
    // Prime stride keeps rows from hashing into each other.
    const STRIDE: i64 = 7919;

    let (xi, yi) = (math::floor(x) as i64, math::floor(y) as i64);
    let (xf, yf) = (x - xi as f32, y - yi as f32);

    let a = hash_noise(xi + yi * STRIDE);
    let b = hash_noise(xi + 1 + yi * STRIDE);
    let c = hash_noise(xi + (yi + 1) * STRIDE);
    let d = hash_noise(xi + 1 + (yi + 1) * STRIDE);

    let (u, v) = (smooth(xf), smooth(yf));
    let top = a + u * (b - a);
    let bot = c + u * (d - c);
    top + v * (bot - top)
}

/// Fractal Brownian motion: layered noise at rising frequency and falling
/// amplitude, in `0.0..=1.0`.
///
/// More octaves means more fine detail, and linearly more work — which for a
/// per-cell effect is paid every cell, every frame. Three is usually plenty.
///
/// Returns the neutral midpoint for a zero octave count, so a degenerate
/// parameter yields a flat field rather than a panic.
#[must_use]
pub fn fbm2(x: f32, y: f32, octaves: u32) -> f32 {
    const LACUNARITY: f32 = 2.0;
    const GAIN: f32 = 0.5;

    if octaves == 0 {
        return 0.5;
    }

    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut total = 0.0;

    for _ in 0..octaves {
        value += amplitude * noise2(x * frequency, y * frequency);
        total += amplitude;
        amplitude *= GAIN;
        frequency *= LACUNARITY;
    }

    value / total
}

/// Asymmetric breathing curve over one unit of `t`, in `0.0..=1.0`.
///
/// A slow cubic rise over the first 60%, then a quicker quadratic fall.
/// Deliberately not a sine: the asymmetry is what makes it read as breathing
/// rather than oscillating.
#[must_use]
pub fn breathe(t: f32) -> f32 {
    let t = math::rem_euclid(t, 1.0);
    if t < 0.6 {
        let p = t / 0.6;
        p * p * p
    } else {
        let p = (t - 0.6) / 0.4;
        1.0 - p * p
    }
}

/// Stochastic brightness factor in `1-intensity ..= 1`, for a phosphor-style
/// flicker.
///
/// Squared to bias towards bright, so it reads as occasional dips rather than
/// constant noise. Distinct `seed` values give independent flicker paths.
#[must_use]
pub fn flicker(t: f32, seed: f32, intensity: f32) -> f32 {
    let n = noise2(t, seed);
    1.0 - intensity * n * n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_bounded_and_deterministic() {
        for i in 0..200 {
            let (x, y) = (i as f32 * 0.37, i as f32 * 0.11);
            let v = noise2(x, y);
            assert!((0.0..=1.0).contains(&v), "noise2({x}, {y}) = {v}");
            assert_eq!(v, noise2(x, y), "noise must be deterministic");

            let v = noise1(x);
            assert!((0.0..=1.0).contains(&v), "noise1({x}) = {v}");
        }
    }

    #[test]
    fn noise_is_continuous() {
        // Neighbouring samples must not jump: a discontinuous field looks like
        // static rather than a smooth gradient.
        let mut previous = noise2(0.0, 0.0);
        for i in 1..500 {
            let v = noise2(i as f32 * 0.01, 0.0);
            assert!(
                (v - previous).abs() < 0.2,
                "discontinuity at step {i}: {previous} -> {v}"
            );
            previous = v;
        }
    }

    #[test]
    fn noise_actually_varies() {
        // A constant field would satisfy the bounds and continuity checks above
        // while being useless.
        let samples: alloc::vec::Vec<f32> =
            (0..100).map(|i| noise2(i as f32 * 0.7, 3.0)).collect();
        let min = samples.iter().copied().fold(f32::MAX, f32::min);
        let max = samples.iter().copied().fold(f32::MIN, f32::max);
        assert!(max - min > 0.3, "field is too flat: {min}..{max}");
    }

    #[test]
    fn fbm_is_bounded_and_handles_zero_octaves() {
        assert_eq!(fbm2(1.0, 2.0, 0), 0.5);
        for octaves in 1..=5 {
            for i in 0..100 {
                let v = fbm2(i as f32 * 0.3, i as f32 * 0.2, octaves);
                assert!((0.0..=1.0).contains(&v), "fbm2 out of range: {v}");
            }
        }
    }

    #[test]
    fn breathe_rises_then_falls_and_stays_bounded() {
        assert_eq!(breathe(0.0), 0.0);
        assert!((breathe(0.6) - 1.0).abs() < 1e-6, "peak should be at 0.6");
        assert!(breathe(0.3) < breathe(0.5), "should rise before the peak");
        assert!(breathe(0.8) < breathe(0.7), "should fall after the peak");

        for i in 0..=100 {
            let v = breathe(i as f32 / 100.0);
            assert!((0.0..=1.0).contains(&v), "breathe out of range: {v}");
        }
    }

    #[test]
    fn breathe_is_periodic() {
        for i in 0..50 {
            let t = i as f32 / 50.0;
            assert!(
                (breathe(t) - breathe(t + 3.0)).abs() < 1e-5,
                "breathe must repeat every unit"
            );
        }
    }

    #[test]
    fn flicker_stays_within_its_intensity_band() {
        for i in 0..200 {
            let v = flicker(i as f32 * 0.05, 1.0, 0.3);
            assert!((0.7..=1.0).contains(&v), "flicker escaped its band: {v}");
        }
        // Zero intensity must be a true no-op, so the effect can be disabled by
        // parameter without a special case.
        assert_eq!(flicker(1.23, 4.0, 0.0), 1.0);
    }
}
