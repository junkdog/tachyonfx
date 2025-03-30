// benches/color_interpolation.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use ratatui::style::Color;
use tachyonfx::{ColorSpace, LruCache, ToRgbComponents};

/// A composite key for caching complete lerp operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct LerpKey {
    from: Color,
    to: Color,
    // We'll use a u8 to represent alpha with 0-255 precision
    // This avoids floating point equality issues in cache lookups
    alpha_byte: u8,
}

impl LerpKey {
    fn new(from: Color, to: Color, alpha: f32) -> Self {
        Self {
            from,
            to,
            // Convert 0.0-1.0 to 0-255
            alpha_byte: (alpha * 255.0).round() as u8,
        }
    }
}

pub fn ui_like_color_pattern_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("ui_pattern");

    // Create a set of theme colors and highlight colors
    let theme_colors = [
        Color::Rgb(30, 30, 30),
        Color::Rgb(220, 220, 220),
        Color::Rgb(40, 40, 40),
        Color::Rgb(180, 180, 180),
    ];

    // Direct interpolation benchmark
    group.bench_with_input(BenchmarkId::new("direct", "ui-pattern"), &(), |b, _| {
        b.iter(|| {
            // Simulate 100 frames of animation
            for _ in 0..100 {
                // Theme colors used in every frame
                for &theme_color in &theme_colors {
                    // Each theme color fades to a slightly different shade
                    let target = Color::Rgb(
                        theme_color.to_rgb().0.saturating_add(10),
                        theme_color.to_rgb().1.saturating_add(10),
                        theme_color.to_rgb().2.saturating_add(10),
                    );

                    // Animation progress
                    let alpha = 0.5;
                    black_box(ColorSpace::Hsl.lerp(&theme_color, &target, alpha));
                }
            }
        });
    });

    // Benchmark with cache size 8 for HSL conversion
    group.bench_with_input(BenchmarkId::new("cached_hsl_size_8", "ui-pattern"), &(), |b, _| {
        b.iter_with_setup(
            LruCache::<Color, (f32, f32, f32), 8>::new,
            |mut cache| {
                // Simulate 100 frames of animation
                for _ in 0..100 {
                    // Theme colors used in every frame
                    for &theme_color in &theme_colors {
                        // Each theme color fades to a slightly different shade
                        let target = Color::Rgb(
                            theme_color.to_rgb().0.saturating_add(10),
                            theme_color.to_rgb().1.saturating_add(10),
                            theme_color.to_rgb().2.saturating_add(10),
                        );

                        // Animation progress
                        let alpha = 0.5;
                        black_box(cache.lerp(&theme_color, &target, ColorSpace::Hsl, alpha));
                    }
                }
            },
        )
    });

    // Benchmark with cache size 16 for HSL conversion
    group.bench_with_input(BenchmarkId::new("cached_hsl_size_16", "ui-pattern"), &(), |b, _| {
        b.iter_with_setup(
            LruCache::<Color, (f32, f32, f32), 16>::new,
            |mut cache| {
                // Simulate 100 frames of animation
                for _ in 0..100 {
                    // Theme colors used in every frame
                    for &theme_color in &theme_colors {
                        // Each theme color fades to a slightly different shade
                        let target = Color::Rgb(
                            theme_color.to_rgb().0.saturating_add(10),
                            theme_color.to_rgb().1.saturating_add(10),
                            theme_color.to_rgb().2.saturating_add(10),
                        );

                        // Animation progress
                        let alpha = 0.5;
                        black_box(cache.lerp(&theme_color, &target, ColorSpace::Hsl, alpha));
                    }
                }
            },
        )
    });

    // Cache the entire lerp operation result
    group.bench_with_input(BenchmarkId::new("cached_full_lerp_size_8", "ui-pattern"), &(), |b, _| {
        b.iter_with_setup(
            LruCache::<LerpKey, Color, 8>::new,
            |mut cache| {
                // Simulate 100 frames of animation
                for _ in 0..100 {
                    // Theme colors used in every frame
                    for &theme_color in &theme_colors {
                        // Each theme color fades to a slightly different shade
                        let target = Color::Rgb(
                            theme_color.to_rgb().0.saturating_add(10),
                            theme_color.to_rgb().1.saturating_add(10),
                            theme_color.to_rgb().2.saturating_add(10),
                        );

                        // Animation progress
                        let alpha = 0.5;

                        // Create the cache key
                        let key = LerpKey::new(theme_color, target, alpha);

                        // Get or compute the interpolated color
                        let result = cache.memoize(&key, |_| {
                            ColorSpace::Hsl.lerp(&theme_color, &target, alpha)
                        });

                        black_box(result);
                    }
                }
            },
        )
    });

    // Cache the entire lerp operation result with a larger cache
    group.bench_with_input(BenchmarkId::new("cached_full_lerp_size_16", "ui-pattern"), &(), |b, _| {
        b.iter_with_setup(
            LruCache::<LerpKey, Color, 16>::new,
            |mut cache| {
                // Simulate 100 frames of animation
                for _ in 0..100 {
                    // Theme colors used in every frame
                    for &theme_color in &theme_colors {
                        // Each theme color fades to a slightly different shade
                        let target = Color::Rgb(
                            theme_color.to_rgb().0.saturating_add(10),
                            theme_color.to_rgb().1.saturating_add(10),
                            theme_color.to_rgb().2.saturating_add(10),
                        );

                        // Animation progress
                        let alpha = 0.5;

                        // Create the cache key
                        let key = LerpKey::new(theme_color, target, alpha);

                        // Get or compute the interpolated color
                        let result = cache.memoize(&key, |_| {
                            ColorSpace::Hsl.lerp(&theme_color, &target, alpha)
                        });

                        black_box(result);
                    }
                }
            },
        )
    });

    group.finish();
}

// Register both benchmarks with Criterion
criterion_group!(
    benches,
    ui_like_color_pattern_benchmark
);
criterion_main!(benches);