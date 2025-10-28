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
    use micromath::F32Ext;
    x.sqrt()
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
    use micromath::F32Ext;
    x.sin()
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
    use micromath::F32Ext;
    x.cos()
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
    use micromath::F32Ext;
    x.round()
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
    use micromath::F32Ext;
    x.floor()
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
    use micromath::F32Ext;
    x.ceil()
}
