// benches/color_conversion.rs
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use ratatui::style::Color;
use tachyonfx::{color_from_hsl, color_to_hsl, ToRgbComponents};

pub fn color_conversion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("color_conversion");
    group.throughput(Throughput::Elements(16));

    // Create a diverse set of colors to test conversions
    // Include primary, secondary, various brightness and saturation levels
    let colors = [
        Color::Rgb(255, 0, 0),     // Red
        Color::Rgb(0, 255, 0),     // Green
        Color::Rgb(0, 0, 255),     // Blue
        Color::Rgb(255, 255, 0),   // Yellow
        Color::Rgb(255, 0, 255),   // Magenta
        Color::Rgb(0, 255, 255),   // Cyan
        Color::Rgb(255, 255, 255), // White
        Color::Rgb(0, 0, 0),       // Black
        Color::Rgb(128, 128, 128), // Gray
        Color::Rgb(255, 128, 0),   // Orange
        Color::Rgb(128, 0, 128),   // Purple
        Color::Rgb(64, 128, 64),   // Muted green
        Color::Rgb(192, 192, 255), // Pale blue
        Color::Rgb(55, 43, 30),    // Brown
        Color::Rgb(181, 101, 29),  // Ochre
        Color::Rgb(112, 41, 99),   // Plum
    ];

    // Benchmark 1: Color to HSL using tachyonfx
    group.bench_function("tachyonfx_color_to_hsl", |b| {
        b.iter(|| {
            for &color in &colors {
                core::hint::black_box(color_to_hsl(&color));
            }
        });
    });

    // Benchmark 2: HSL to Color using tachyonfx
    group.bench_function("tachyonfx_hsl_to_color", |b| {
        b.iter(|| {
            for &color in &colors {
                let (h, s, l) = color_to_hsl(&color);
                core::hint::black_box(color_from_hsl(h, s, l));
            }
        });
    });

    // Benchmark 3: Color to HSL using colorsys
    group.bench_function("colorsys_color_to_hsl", |b| {
        b.iter(|| {
            for &color in &colors {
                let (r, g, b) = color.to_rgb();
                let rgb = colorsys::Rgb::from([r as f64, g as f64, b as f64]);
                let hsl: colorsys::Hsl = rgb.into();
                core::hint::black_box((hsl.hue(), hsl.saturation(), hsl.lightness()));
            }
        });
    });

    // Benchmark 4: HSL to Color using colorsys
    group.bench_function("colorsys_hsl_to_color", |b| {
        b.iter(|| {
            for &color in &colors {
                let (r, g, b) = color.to_rgb();
                let rgb = colorsys::Rgb::from([r as f64, g as f64, b as f64]);
                let hsl: colorsys::Hsl = rgb.into();

                // Convert back to RGB
                let rgb_back: colorsys::Rgb = hsl.into();
                core::hint::black_box(Color::Rgb(
                    rgb_back.red().round() as u8,
                    rgb_back.green().round() as u8,
                    rgb_back.blue().round() as u8,
                ));
            }
        });
    });

    // Benchmark 5: Round-trip Color→HSL→Color using tachyonfx
    group.bench_function("tachyonfx_round_trip", |b| {
        b.iter(|| {
            for &color in &colors {
                let (h, s, l) = color_to_hsl(&color);
                core::hint::black_box(color_from_hsl(h, s, l));
            }
        });
    });

    // Benchmark 6: Round-trip Color→HSL→Color using colorsys
    group.bench_function("colorsys_round_trip", |b| {
        b.iter(|| {
            for &color in &colors {
                let (r, g, b) = color.to_rgb();
                let rgb = colorsys::Rgb::from([r as f64, g as f64, b as f64]);
                let hsl: colorsys::Hsl = rgb.into();
                let rgb_back: colorsys::Rgb = hsl.into();
                core::hint::black_box(Color::Rgb(
                    rgb_back.red().round() as u8,
                    rgb_back.green().round() as u8,
                    rgb_back.blue().round() as u8,
                ));
            }
        });
    });

    group.finish();

    // Batch conversion benchmark in a separate group with its own throughput
    let mut group = c.benchmark_group("color_conversion_batch");
    let batch_size = 100;
    group.throughput(Throughput::Elements(batch_size * 16));

    group.bench_function("tachyonfx_batch_100", |b| {
        b.iter(|| {
            for _ in 0..batch_size {
                for &color in &colors {
                    let (h, s, l) = color_to_hsl(&color);
                    core::hint::black_box(color_from_hsl(h, s, l));
                }
            }
        });
    });

    // Benchmark 8: Batch conversions (100 items) - colorsys
    group.bench_function("colorsys_batch_100", |b| {
        b.iter(|| {
            for _ in 0..batch_size {
                for &color in &colors {
                    let (r, g, b) = color.to_rgb();
                    let rgb = colorsys::Rgb::from([r as f64, g as f64, b as f64]);
                    let hsl: colorsys::Hsl = rgb.into();
                    let rgb_back: colorsys::Rgb = hsl.into();
                    core::hint::black_box(Color::Rgb(
                        rgb_back.red().round() as u8,
                        rgb_back.green().round() as u8,
                        rgb_back.blue().round() as u8,
                    ));
                }
            }
        });
    });

    group.finish();
}

criterion_group!(benches, color_conversion_benchmark);
criterion_main!(benches);
