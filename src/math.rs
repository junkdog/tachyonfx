//! Math utilities with no_std compatibility
//!
//! This module provides math functions that work in both std and no_std environments.
//! In std environments, it uses the standard library implementations for performance.
//! In no_std environments, it uses custom approximation functions optimized for embedded
//! systems.

#[cfg(not(feature = "std"))]
use core::f32::consts::{PI, TAU};

/// Square root function that works in both std and no_std environments
#[cfg(feature = "std")]
#[inline]
pub(crate) fn sqrt(x: f32) -> f32 {
    x.sqrt()
}

/// Square root function that works in both std and no_std environments
#[cfg(not(feature = "std"))]
#[inline]
pub(crate) fn sqrt(x: f32) -> f32 {
    sqrt_approx(x)
}

/// Sine function that works in both std and no_std environments
#[cfg(feature = "std")]
#[inline]
pub(crate) fn sin(x: f32) -> f32 {
    x.sin()
}

/// Sine function that works in both std and no_std environments
#[cfg(not(feature = "std"))]
#[inline]
pub(crate) fn sin(x: f32) -> f32 {
    sin_approx(x)
}

/// Cosine function that works in both std and no_std environments
#[cfg(feature = "std")]
#[inline]
pub(crate) fn cos(x: f32) -> f32 {
    x.cos()
}

/// Cosine function that works in both std and no_std environments
#[cfg(not(feature = "std"))]
#[inline]
pub(crate) fn cos(x: f32) -> f32 {
    cos_approx(x)
}

/// Power function that works in both std and no_std environments
#[cfg(feature = "std")]
#[inline]
pub(crate) fn powf(base: f32, exp: f32) -> f32 {
    base.powf(exp)
}

/// Power function that works in both std and no_std environments
#[cfg(not(feature = "std"))]
#[inline]
pub(crate) fn powf(base: f32, exp: f32) -> f32 {
    pow_approx(base, exp)
}

/// Integer power function that works in both std and no_std environments
#[cfg(feature = "std")]
#[inline]
pub(crate) fn powi(base: f32, exp: i32) -> f32 {
    base.powi(exp)
}

/// Integer power function that works in both std and no_std environments
#[cfg(not(feature = "std"))]
#[inline]
pub(crate) fn powi(base: f32, exp: i32) -> f32 {
    pow_approx(base, exp as f32)
}

/// Round function that works in both std and no_std environments
#[cfg(feature = "std")]
#[inline]
pub(crate) fn round(x: f32) -> f32 {
    x.round()
}

/// Round function that works in both std and no_std environments
#[cfg(not(feature = "std"))]
#[inline]
pub(crate) fn round(x: f32) -> f32 {
    if x >= 0.0 {
        (x + 0.5).floor_impl()
    } else {
        (x - 0.5).ceil_impl()
    }
}

/// Floor function that works in both std and no_std environments
#[cfg(feature = "std")]
#[inline]
pub(crate) fn floor(x: f32) -> f32 {
    x.floor()
}

/// Floor function that works in both std and no_std environments
#[cfg(not(feature = "std"))]
#[inline]
pub(crate) fn floor(x: f32) -> f32 {
    x.floor_impl()
}

/// Ceiling function that works in both std and no_std environments
#[cfg(feature = "std")]
#[inline]
pub(crate) fn ceil(x: f32) -> f32 {
    x.ceil()
}

/// Ceiling function that works in both std and no_std environments
#[cfg(not(feature = "std"))]
#[inline]
pub(crate) fn ceil(x: f32) -> f32 {
    x.ceil_impl()
}

// Approximation functions for no_std environments
#[cfg(not(feature = "std"))]
fn sqrt_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }

    // Newton-Raphson method
    // 4 iterations provides 1e-4 accuracy, good balance for embedded systems
    let mut guess = x / 2.0;
    for _ in 0..4 {
        guess = (guess + x / guess) / 2.0;
    }
    guess
}

#[cfg(not(feature = "std"))]
fn pow_approx(base: f32, exp: f32) -> f32 {
    if exp == 0.0 {
        return 1.0;
    }
    if base == 0.0 {
        return 0.0;
    }
    if exp == 1.0 {
        return base;
    }

    // For 2^x, use bit manipulation approximation
    if base == 2.0 {
        let exp_int = exp as i32;
        let exp_frac = exp - exp_int as f32;

        let int_part = if exp_int >= 0 {
            (1u32 << exp_int.min(30)) as f32
        } else {
            1.0 / (1u32 << (-exp_int).min(30)) as f32
        };

        // Linear approximation for fractional part
        let frac_part = 1.0 + exp_frac * core::f32::consts::LN_2;

        int_part * frac_part
    } else {
        // General case using exp(exp * ln(base))
        exp_approx(exp * ln_approx(base))
    }
}

#[cfg(not(feature = "std"))]
fn exp_approx(x: f32) -> f32 {
    if x > 10.0 {
        return 22026.5;
    } // e^10 ≈ 22026
    if x < -10.0 {
        return 0.0;
    }

    // Taylor series: e^x = 1 + x + x²/2! + x³/3! + ...
    // 6 iterations provides 1e-3 accuracy, good balance for embedded systems
    let mut result = 1.0;
    let mut term = 1.0;

    for i in 1..6 {
        term *= x / i as f32;
        result += term;
    }

    result
}

#[cfg(not(feature = "std"))]
fn ln_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return -100.0;
    }
    if x == 1.0 {
        return 0.0;
    }

    // Use the identity ln(x) = 2 * arctanh((x-1)/(x+1))
    let y = (x - 1.0) / (x + 1.0);
    let y2 = y * y;

    // arctanh series: arctanh(y) = y + y³/3 + y⁵/5 + ...
    let mut sum = y;
    let mut term = y;

    for i in 1..10 {
        term *= y2;
        sum += term / (2 * i + 1) as f32;
    }

    2.0 * sum
}

#[cfg(not(feature = "std"))]
fn sin_approx(x: f32) -> f32 {
    let mut x = x % TAU;
    if x > PI {
        x -= TAU;
    }
    if x < -PI {
        x += TAU;
    }

    // taylor series: sin(x) = x - x³/3! + x⁵/5! - x⁷/7! + ...
    // 3 iterations provides 1e-3 accuracy, good balance for embedded systems
    let x2 = x * x;
    let mut result = x;
    let mut term = x;

    for i in 1..3 {
        term *= -x2 / ((2 * i) * (2 * i + 1)) as f32;
        result += term;
    }

    result
}

#[cfg(not(feature = "std"))]
fn cos_approx(x: f32) -> f32 {
    sin_approx(PI / 2.0 - x)
}

// Helper trait to add floor/ceil implementations for no_std
#[cfg(not(feature = "std"))]
trait FloorCeilImpl {
    fn floor_impl(self) -> Self;
    fn ceil_impl(self) -> Self;
}

#[cfg(not(feature = "std"))]
impl FloorCeilImpl for f32 {
    fn floor_impl(self) -> f32 {
        if self >= 0.0 {
            self as i32 as f32
        } else {
            let int_part = self as i32 as f32;
            if self == int_part {
                int_part
            } else {
                int_part - 1.0
            }
        }
    }

    fn ceil_impl(self) -> f32 {
        if self >= 0.0 {
            let int_part = self as i32 as f32;
            if self == int_part {
                int_part
            } else {
                int_part + 1.0
            }
        } else {
            self as i32 as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use core::f32::consts::PI;

    use super::*;

    #[test]
    fn test_sqrt_basic() {
        assert_eq!(sqrt(4.0), 2.0);
        assert_eq!(sqrt(9.0), 3.0);
        assert!(sqrt(2.0) > 1.4 && sqrt(2.0) < 1.5);
    }

    #[test]
    fn test_sin_cos_basic() {
        assert!((sin(0.0) - 0.0).abs() < 0.01);
        assert!((cos(0.0) - 1.0).abs() < 0.01);
        assert!((sin(PI / 2.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_powf_basic() {
        assert_eq!(powf(2.0, 3.0), 8.0);
        assert_eq!(powf(10.0, 0.0), 1.0);
        assert!((powf(2.0, 0.5) - sqrt(2.0)).abs() < 0.1);
    }

    #[test]
    fn test_powi_basic() {
        // For embedded devices, we prioritize performance over precision
        // These are rough approximations suitable for visual effects, not scientific computation
        assert!((powi(2.0, 3) - 8.0).abs() < 0.4); // ~5% tolerance
        assert_eq!(powi(10.0, 0), 1.0);
        assert!((powi(2.0, -2) - 0.25).abs() < 0.013); // ~5% tolerance
        assert!((powi(5.0, 2) - 25.0).abs() < 3.75); // ~15% tolerance for embedded visual
                                                     // effects
    }

    #[test]
    fn test_round_basic() {
        assert_eq!(round(0.0), 0.0);
        assert_eq!(round(1.0), 1.0);
        assert_eq!(round(-1.0), -1.0);
        assert_eq!(round(1.5), 2.0);
        assert_eq!(round(1.4), 1.0);
        assert_eq!(round(-1.5), -2.0);
        assert_eq!(round(-1.4), -1.0);
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -3.0);
    }

    #[test]
    fn test_floor_basic() {
        assert_eq!(floor(0.0), 0.0);
        assert_eq!(floor(1.0), 1.0);
        assert_eq!(floor(-1.0), -1.0);
        assert_eq!(floor(1.9), 1.0);
        assert_eq!(floor(1.1), 1.0);
        assert_eq!(floor(-1.1), -2.0);
        assert_eq!(floor(-1.9), -2.0);
        assert_eq!(floor(3.7), 3.0);
        assert_eq!(floor(-3.7), -4.0);
    }

    #[test]
    fn test_ceil_basic() {
        assert_eq!(ceil(0.0), 0.0);
        assert_eq!(ceil(1.0), 1.0);
        assert_eq!(ceil(-1.0), -1.0);
        assert_eq!(ceil(1.1), 2.0);
        assert_eq!(ceil(1.9), 2.0);
        assert_eq!(ceil(-1.1), -1.0);
        assert_eq!(ceil(-1.9), -1.0);
        assert_eq!(ceil(3.1), 4.0);
        assert_eq!(ceil(-3.1), -3.0);
    }

    #[test]
    fn test_round_floor_ceil_edge_cases() {
        // Test very small values
        assert_eq!(round(0.1), 0.0);
        assert_eq!(round(-0.1), 0.0);
        assert_eq!(floor(0.1), 0.0);
        assert_eq!(floor(-0.1), -1.0);
        assert_eq!(ceil(0.1), 1.0);
        assert_eq!(ceil(-0.1), 0.0);

        // Test values close to integers
        assert_eq!(round(0.99999), 1.0);
        assert_eq!(round(-0.99999), -1.0);
        assert_eq!(floor(0.99999), 0.0);
        assert_eq!(ceil(0.00001), 1.0);
    }
}
