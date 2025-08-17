use std::fmt::Debug;

use ratatui::prelude::Color;

use crate::lru_cache::LruCache;

/// A specialized cache for color interpolation operations that handles `Color::Reset`
/// with appropriate fallback colors.
///
/// This cache wraps two [`LruCache`] instances (one for foreground, one for background)
/// and automatically maps `Color::Reset` to semantically appropriate fallback colors:
/// - Foreground: `Color::Reset` → `Color::White` (typical terminal default)
/// - Background: `Color::Reset` → `Color::Black` (typical terminal default)
///
/// This ensures that color interpolation operations work correctly when dealing with
/// cells that have reset colors, while maintaining cache efficiency.
///
/// # Example
///
/// ```rust
/// use tachyonfx::{ColorCache, ColorSpace};
/// use ratatui::prelude::Color;
///
/// let mut cache = ColorCache::<Color, 8>::new();
/// let target_color = Color::Cyan;
///
/// // This will treat Color::Reset as Color::White for foreground interpolation
/// let result = cache.memoize_fg(Color::Reset, target_color, |c| {
///     ColorSpace::Rgb.lerp(c, &target_color, 0.5)
/// });
///
/// // This will treat Color::Reset as Color::Black for background interpolation
/// let result = cache.memoize_bg(Color::Reset, target_color, |c| {
///     ColorSpace::Rgb.lerp(c, &target_color, 0.5)
/// });
/// ```
pub struct ColorCache<Context, const N: usize>
where
    Context: Debug + Clone + Copy + PartialEq + Eq + Default,
{
    fg_cache: LruCache<CacheKey<Context>, Color, N>,
    bg_cache: LruCache<CacheKey<Context>, Color, N>,
}

impl<Context, const N: usize> ColorCache<Context, N>
where
    Context: Debug + Clone + Copy + PartialEq + Eq + Default,
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
    // pub fn memoize_fg<F, ID>(&mut self, from: Color, to: Color, alpha: f32, f: F) -> Color
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
    // pub fn memoize_bg<F>(&mut self, from: Color, to: Color, alpha: f32, f: F) -> Color
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
    Context: Debug + Clone + Copy + PartialEq + Eq + Default,
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
