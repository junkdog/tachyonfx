use core::fmt::Debug;

use ratatui_core::style::Color;

use crate::lru_cache::LruCache;

/// A specialized stack-based cache for color transformation operations that handles
/// `Color::Reset` with appropriate fallback colors.
///
/// This cache wraps two [`LruCache`] instances (one for foreground, one for background)
/// and automatically maps `Color::Reset` to semantically appropriate fallback colors:
/// - Foreground: `Color::Reset` → `Color::White` (typical terminal default)
/// - Background: `Color::Reset` → `Color::Black` (typical terminal default)
///
/// This ensures that color transformation operations work correctly when dealing with
/// cells that have reset colors, while maintaining cache efficiency.
///
/// ## Context Parameter
///
/// The `Context` generic parameter allows you to provide additional discriminating
/// information for cache entries. The cache key consists of the source color plus
/// the context, ensuring that different transformations are cached separately.
///
/// ### When to use different context types:
///
/// - **`ColorCache<Color, N>`**: Use when the transformation depends on a target color.
///   Different target colors should produce different results for the same source color.
///   Example: fading from Red to Blue vs Red to Green.
///
/// - **`ColorCache<(), N>`**: Use when the transformation depends only on the source
///   color. The same source color always produces the same result regardless of other
///   factors. Example: HSL shifts that are applied uniformly.
///
/// - **`ColorCache<(Color, u8), N>`**: Use for complex transformations that depend on
///   multiple parameters. Example: alpha-aware blending operations where alpha is
///   converted to u8 for stable caching (e.g., `(alpha * 255.0).round() as u8`).
///
/// # Examples
///
/// ```rust
/// use tachyonfx::{ColorCache, ColorSpace};
/// use ratatui::prelude::Color;
///
/// // Example 1: Target-dependent transformation (fading to specific colors)
/// let mut fade_cache = ColorCache::<Color, 8>::new();
/// let target_color = Color::Cyan;
///
/// let result = fade_cache.memoize_fg(Color::Red, target_color, |source| {
///     ColorSpace::Rgb.lerp(source, &target_color, 0.5)
/// });
/// // Cache key: (Red, Cyan) - different from (Red, Blue)
///
/// // Example 2: Source-only transformation (uniform HSL shift)
/// let mut hsl_cache = ColorCache::<(), 8>::new();
///
/// let result = hsl_cache.memoize_fg(Color::Red, (), |source| {
///     // Apply consistent HSL transformation
///     ColorSpace::Hsl.lerp(source, &Color::Yellow, 0.3)
/// });
/// // Cache key: (Red, ()) - same for all Red inputs
///
/// // Example 3: Alpha-aware transformation (using u8 for stable caching)
/// let mut alpha_cache = ColorCache::<u8, 16>::new();
/// let alpha = 0.75f32;
/// let alpha_key = (alpha.clamp(0.0, 1.0) * 255.0) as u8; // Convert f32 to u8
///
/// let result = alpha_cache.memoize_fg(Color::Blue, alpha_key, |source| {
///     ColorSpace::Rgb.lerp(source, &Color::White, alpha)
/// });
/// // Cache key: (Blue, 191) - stable u8 representation of 0.75
/// ```
pub struct ColorCache<Context, const N: usize>
where
    Context: Debug + PartialEq + Copy + Eq + Default,
{
    fg_cache: LruCache<CacheKey<Context>, Color, N>,
    bg_cache: LruCache<CacheKey<Context>, Color, N>,
}

impl<Context, const N: usize> ColorCache<Context, N>
where
    Context: Debug + PartialEq + Copy + Eq + Default,
{
    /// Creates a new `ColorCache` with empty foreground and background caches.
    pub fn new() -> Self {
        Self {
            fg_cache: LruCache::new(),
            bg_cache: LruCache::new(),
        }
    }

    /// Memoizes a foreground color computation.
    ///
    /// If the input key is `Color::Reset`, it will be treated as `Color::White`
    /// for both caching and computation purposes.
    ///
    /// # Arguments
    ///
    /// * `from` - The source color to compute from
    /// * `context` - Context data for cache key differentiation
    /// * `f` - Function that computes the result color from the effective key
    ///
    /// # Returns
    ///
    /// The computed color, either from cache or newly computed
    pub fn memoize_fg<F>(&mut self, from: Color, context: Context, f: F) -> Color
    where
        F: FnOnce(&Color) -> Color,
    {
        let from = if from == Color::Reset { Color::White } else { from };
        let key = CacheKey::new(from, context);

        self.fg_cache.memoize(&key, |key| f(&key.from))
    }

    /// Memoizes a background color computation.
    ///
    /// If the input key is `Color::Reset`, it will be treated as `Color::Black`
    /// for both caching and computation purposes.
    ///
    /// # Arguments
    ///
    /// * `from` - The source color to compute from
    /// * `context` - Context data for cache key differentiation
    /// * `f` - Function that computes the result color from the effective key
    ///
    /// # Returns
    ///
    /// The computed color, either from cache or newly computed
    pub fn memoize_bg<F>(&mut self, from: Color, context: Context, f: F) -> Color
    where
        F: FnOnce(&Color) -> Color,
    {
        let from = if from == Color::Reset { Color::Black } else { from };
        let key = CacheKey::new(from, context);

        self.bg_cache.memoize(&key, |key| f(&key.from))
    }

    /// Returns the number of cache hits for foreground color operations.
    pub fn fg_cache_hits(&self) -> u32 {
        self.fg_cache.cache_hits()
    }

    /// Returns the number of cache misses for foreground color operations.
    pub fn fg_cache_misses(&self) -> u32 {
        self.fg_cache.cache_misses()
    }

    /// Returns the number of cache hits for background color operations.
    pub fn bg_cache_hits(&self) -> u32 {
        self.bg_cache.cache_hits()
    }

    /// Returns the number of cache misses for background color operations.
    pub fn bg_cache_misses(&self) -> u32 {
        self.bg_cache.cache_misses()
    }
}

impl<Context, const N: usize> Default for ColorCache<Context, N>
where
    Context: Debug + Copy + PartialEq + Eq + Default,
{
    fn default() -> Self {
        Self::new()
    }
}

/// A composite key for caching color operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct CacheKey<T> {
    from: Color,
    context: T,
}

impl<T> CacheKey<T> {
    fn new(from: Color, context: T) -> Self {
        Self { from, context }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColorSpace;

    #[test]
    fn test_fg_color_reset_mapping() {
        let mut cache = ColorCache::<Color, 4>::new();
        let target = Color::Cyan;

        // First call should compute the value
        let result1 = cache.memoize_fg(Color::Reset, target, |c| {
            // Should receive Color::White instead of Color::Reset
            assert_eq!(*c, Color::White);
            ColorSpace::Rgb.lerp(c, &target, 0.5)
        });

        // Second call should hit cache
        let result2 = cache.memoize_fg(Color::Reset, target, |_c| {
            panic!("Should not be called - should hit cache");
        });

        assert_eq!(result1, result2);
        assert_eq!(cache.fg_cache_hits(), 1);
        assert_eq!(cache.fg_cache_misses(), 1);
    }

    #[test]
    fn test_bg_color_reset_mapping() {
        let mut cache = ColorCache::<Color, 4>::new();
        let target = Color::Cyan;

        // First call should compute the value
        let result1 = cache.memoize_bg(Color::Reset, target, |c| {
            // Should receive Color::Black instead of Color::Reset
            assert_eq!(*c, Color::Black);
            ColorSpace::Rgb.lerp(c, &target, 0.5)
        });

        // Second call should hit cache
        let result2 = cache.memoize_bg(Color::Reset, target, |_c| {
            panic!("Should not be called - should hit cache");
        });

        assert_eq!(result1, result2);
        assert_eq!(cache.bg_cache_hits(), 1);
        assert_eq!(cache.bg_cache_misses(), 1);
    }

    #[test]
    fn test_non_reset_colors_passthrough() {
        let mut cache = ColorCache::<Color, 4>::new();
        let source = Color::Red;
        let target = Color::Blue;

        let result = cache.memoize_fg(source, target, |c| {
            // Should receive the original color
            assert_eq!(*c, source);
            ColorSpace::Rgb.lerp(c, &target, 0.5)
        });

        // Should be purple-ish (halfway between red and blue)
        // Color::Red is (128, 0, 0) and Color::Blue is (0, 0, 128)
        // So 50% interpolation should be (64, 0, 64)
        assert_eq!(result, Color::Rgb(64, 0, 64));
    }

    #[test]
    fn test_separate_fg_bg_caches() {
        let mut cache = ColorCache::<Color, 4>::new();
        let target = Color::White;

        // These should be cached separately
        let fg_result = cache.memoize_fg(Color::Reset, target, |c| {
            ColorSpace::Rgb.lerp(c, &target, 0.5)
        });

        let bg_result = cache.memoize_bg(Color::Reset, target, |c| {
            ColorSpace::Rgb.lerp(c, &target, 0.5)
        });

        // Results should be different because fg uses White and bg uses Black as fallback
        assert_ne!(fg_result, bg_result);

        // Each cache should have one miss
        assert_eq!(cache.fg_cache_misses(), 1);
        assert_eq!(cache.bg_cache_misses(), 1);
    }
}
