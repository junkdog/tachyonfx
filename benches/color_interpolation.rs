// benches/color_interpolation.rs
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use ratatui::style::Color;
use tachyonfx::{ColorSpace, LruCache, ToRgbComponents};

// Constants for benchmark parameters
const ANIMATION_FRAMES: usize = 100;
const INTERPOLATION_ALPHA: f32 = 0.5;
const COLOR_OFFSET: u8 = 10;

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

/// Calculate target color by adding offset to RGB components
fn calculate_target_color(theme_color: Color) -> Color {
    let (r, g, b) = theme_color.to_rgb();
    Color::Rgb(
        r.saturating_add(COLOR_OFFSET),
        g.saturating_add(COLOR_OFFSET),
        b.saturating_add(COLOR_OFFSET),
    )
}

/// Run animation loop with the provided interpolation function
fn run_animation_loop<F>(theme_colors: &[Color], mut interpolate_fn: F)
where
    F: FnMut(Color, Color),
{
    for _ in 0..ANIMATION_FRAMES {
        for &theme_color in theme_colors {
            let target = calculate_target_color(theme_color);
            interpolate_fn(theme_color, target);
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
            run_animation_loop(&theme_colors, |theme_color, target| {
                std::hint::black_box(ColorSpace::Hsl.lerp(
                    &theme_color,
                    &target,
                    INTERPOLATION_ALPHA,
                ));
            });
        });
    });

    // Benchmark with cache size 8 for HSL conversion
    group.bench_with_input(
        BenchmarkId::new("cached_hsl_size_8", "ui-pattern"),
        &(),
        |b, _| {
            b.iter_with_setup(LruCache::<Color, (f32, f32, f32), 8>::new, |mut cache| {
                run_animation_loop(&theme_colors, |theme_color, target| {
                    std::hint::black_box(cache.lerp(
                        &theme_color,
                        &target,
                        ColorSpace::Hsl,
                        INTERPOLATION_ALPHA,
                    ));
                });
            })
        },
    );

    // Benchmark with cache size 16 for HSL conversion
    group.bench_with_input(
        BenchmarkId::new("cached_hsl_size_16", "ui-pattern"),
        &(),
        |b, _| {
            b.iter_with_setup(LruCache::<Color, (f32, f32, f32), 16>::new, |mut cache| {
                run_animation_loop(&theme_colors, |theme_color, target| {
                    std::hint::black_box(cache.lerp(
                        &theme_color,
                        &target,
                        ColorSpace::Hsl,
                        INTERPOLATION_ALPHA,
                    ));
                });
            })
        },
    );

    // Cache the entire lerp operation result
    group.bench_with_input(
        BenchmarkId::new("cached_full_lerp_size_8", "ui-pattern"),
        &(),
        |b, _| {
            b.iter_with_setup(LruCache::<LerpKey, Color, 8>::new, |mut cache| {
                run_animation_loop(&theme_colors, |theme_color, target| {
                    let key = LerpKey::new(theme_color, target, INTERPOLATION_ALPHA);
                    let result = cache.memoize(&key, |_| {
                        ColorSpace::Hsl.lerp(&theme_color, &target, INTERPOLATION_ALPHA)
                    });
                    std::hint::black_box(result);
                });
            })
        },
    );

    // Cache the entire lerp operation result with a larger cache
    group.bench_with_input(
        BenchmarkId::new("cached_full_lerp_size_16", "ui-pattern"),
        &(),
        |b, _| {
            b.iter_with_setup(LruCache::<LerpKey, Color, 16>::new, |mut cache| {
                run_animation_loop(&theme_colors, |theme_color, target| {
                    let key = LerpKey::new(theme_color, target, INTERPOLATION_ALPHA);
                    let result = cache.memoize(&key, |_| {
                        ColorSpace::Hsl.lerp(&theme_color, &target, INTERPOLATION_ALPHA)
                    });
                    std::hint::black_box(result);
                });
            })
        },
    );

    group.finish();
}

// Register both benchmarks with Criterion
criterion_group!(benches, ui_like_color_pattern_benchmark);
criterion_main!(benches);
