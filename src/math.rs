//! Math utilities with no_std compatibility
//!
//! This module provides math functions that work in both std and no_std environments.
//! Functions where micromath outperforms std (sqrt, round, floor, ceil) always use
//! micromath. Functions where std is faster (powf) use cfg-gated implementations.

use core::f32::consts::TAU;

/// Parabolic sine approximation. Input is in radians.
#[inline(always)]
pub fn parabolic_sin(t: f32) -> f32 {
    wave_sin(t * (1.0 / TAU))
}

/// Parabolic cosine approximation. Input is in radians.
#[inline(always)]
pub fn parabolic_cos(t: f32) -> f32 {
    wave_sin(t * (1.0 / TAU) + 0.25)
}

/// Fast, branchless sine approximation using parabolic segments.
///
/// Input `t` is in normalized cycles where `1.0` equals one full period.
/// Values beyond `1.0` wrap naturally, so `t` can increase continuously
/// to produce repeating oscillations (e.g., `2.5` is equivalent to `0.5`).
///
/// Output ranges from `-1.0` to `1.0`. The shape closely follows a true
/// sine wave but with slightly flattened peaks.
#[inline(always)]
pub fn wave_sin(t: f32) -> f32 {
    let f = micromath::F32Ext::fract(t);
    let x = if f < 0.0 { f + 1.0 } else { f };
    let phase: f32 = 1.0 - 2.0 * x;
    4.0 * phase * (1.0 - micromath::F32Ext::abs(phase))
}

/// Fast, branchless cosine approximation using parabolic segments.
///
/// Input `t` is in normalized cycles where `1.0` equals one full period.
/// Values beyond `1.0` wrap naturally. Output ranges from `-1.0` to `1.0`.
#[inline(always)]
pub fn wave_cos(t: f32) -> f32 {
    wave_sin(t + 0.25)
}

/// Square root function using micromath (faster than std)
#[inline]
pub(crate) fn sqrt(x: f32) -> f32 {
    micromath::F32Ext::sqrt(x)
}

/// Sine function that works in both std and no_std environments
#[inline]
pub(crate) fn sin(x: f32) -> f32 {
    parabolic_sin(x)
}

/// Cosine function that works in both std and no_std environments
#[inline]
pub(crate) fn cos(x: f32) -> f32 {
    parabolic_cos(x)
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
    use micromath::F32Ext;
    base.powf(exp)
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
    use micromath::F32Ext;
    base.powi(exp)
}

/// Round function using micromath (faster than std)
#[inline]
pub(crate) fn round(x: f32) -> f32 {
    micromath::F32Ext::round(x)
}

/// Floor function using micromath (faster than std)
#[inline]
pub(crate) fn floor(x: f32) -> f32 {
    micromath::F32Ext::floor(x)
}

/// Ceiling function using micromath (faster than std)
#[inline]
pub(crate) fn ceil(x: f32) -> f32 {
    micromath::F32Ext::ceil(x)
}
