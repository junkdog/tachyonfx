// benches/cell_filter.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ratatui::{buffer::Buffer, layout::Rect};
use ratatui::layout::Margin;
use ratatui::prelude::Color;
use tachyonfx::{CellFilter, Duration, fx, Shader};

pub fn cell_filter_overhead_benchmark(c: &mut Criterion) {
    // Define a large buffer size for consistent measurement
    let width = 100;
    let height = 100;
    let area = Rect::new(0, 0, width, height);

    let mut group = c.benchmark_group("cell_filter_overhead");

    // Baseline - Empty effect function with no filter operations
    group.bench_function("raw_no_filter", |b| {
        b.iter_with_setup(
            || Buffer::empty(area),
            |buffer| {
                // This is the absolute baseline - just iterating through the buffer
                for y in 0..height {
                    for x in 0..width {
                        black_box(&buffer[(x, y)]);
                    }
                }
            }
        );
    });

    // Test the overhead of using CellFilter::All (should be minimal)
    group.bench_function("filter_plain", |b| {
        b.iter_with_setup(
            || {
                let buffer = Buffer::empty(area);
                let effect = fx::effect_fn((), 1, |_, _, cells| {
                    // Just iterate over the cells with black_box to prevent optimizations
                    for (pos, cell) in cells {
                        black_box(pos);
                        black_box(cell);
                    }
                });
                (buffer, effect)
            },
            |(mut buffer, mut effect)| {
                effect.process(black_box(Duration::from_millis(16)), &mut buffer, area);
            }
        );
    });

    // Test the overhead of using CellFilter::All (should be minimal)
    group.bench_function("filter_all", |b| {
        b.iter_with_setup(
            || {
                let buffer = Buffer::empty(area);
                let effect = fx::effect_fn((), 1, |_, _, cells| {
                    // Just iterate over the cells with black_box to prevent optimizations
                    for (pos, cell) in cells {
                        black_box(pos);
                        black_box(cell);
                    }
                }).with_filter(CellFilter::All);
                (buffer, effect)
            },
            |(mut buffer, mut effect)| {
                effect.process(black_box(Duration::from_millis(16)), &mut buffer, area);
            }
        );
    });

    // Test the overhead of using CellFilter::All (should be minimal)
    group.bench_function("filter_all_of_inner_color", |b| {
        b.iter_with_setup(
            || {
                let buffer = Buffer::empty(area);
                let effect = fx::effect_fn((), 1, |_, _, cells| {
                    // Just iterate over the cells with black_box to prevent optimizations
                    for (pos, cell) in cells {
                        black_box(pos);
                        black_box(cell);
                    }
                }).with_filter(CellFilter::AllOf(vec![
                    CellFilter::FgColor(Color::Red),
                    CellFilter::Inner(Margin::new(1, 1)),
                ]));
                (buffer, effect)
            },
            |(mut buffer, mut effect)| {
                effect.process(black_box(Duration::from_millis(16)), &mut buffer, area);
            }
        );
    });


    group.finish();
}

criterion_group!(benches, cell_filter_overhead_benchmark);
criterion_main!(benches);