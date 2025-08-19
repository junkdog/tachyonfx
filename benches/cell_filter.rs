// benches/cell_filter.rs
use criterion::{criterion_group, criterion_main, Criterion};
use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    prelude::Color,
};
use tachyonfx::{fx, CellFilter, Duration, Effect, Shader};

// Constants for consistent measurements
const BENCH_WIDTH: u16 = 100;
const BENCH_HEIGHT: u16 = 100;
const BENCH_DURATION: Duration = Duration::from_millis(16);

fn bench_area() -> Rect {
    Rect::new(0, 0, BENCH_WIDTH, BENCH_HEIGHT)
}

fn create_noop_effect(filter: Option<CellFilter>) -> Effect {
    let mut effect = fx::effect_fn((), 1, |_, _, cells| {
        // Just iterate over the cells with black_box to prevent optimizations
        for (pos, cell) in cells {
            std::hint::black_box(pos);
            std::hint::black_box(cell);
        }
    });

    if let Some(filter) = filter {
        effect = effect.with_filter(filter);
    }

    effect
}

fn bench_effect_with_filter(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    name: &str,
    filter: Option<CellFilter>,
) {
    let area = bench_area();

    group.bench_function(name, |b| {
        b.iter_with_setup(
            || (Buffer::empty(area), create_noop_effect(filter.clone())),
            |(mut buffer, mut effect)| {
                effect.process(std::hint::black_box(BENCH_DURATION), &mut buffer, area);
            },
        );
    });
}

pub fn cell_filter_overhead_benchmark(c: &mut Criterion) {
    let area = bench_area();
    let mut group = c.benchmark_group("cell_filter_overhead");

    // Baseline - Raw buffer iteration with no effect framework overhead
    group.bench_function("raw_no_filter", |b| {
        b.iter_with_setup(
            || Buffer::empty(area),
            |buffer| {
                // This is the absolute baseline - just iterating through the buffer
                for y in 0..BENCH_HEIGHT {
                    for x in 0..BENCH_WIDTH {
                        std::hint::black_box(&buffer[(x, y)]);
                    }
                }
            },
        );
    });

    // Benchmark different filter configurations
    let test_cases = [
        ("filter_plain", None),
        ("filter_all", Some(CellFilter::All)),
        (
            "filter_all_of_inner_color",
            Some(CellFilter::AllOf(vec![
                CellFilter::FgColor(Color::Red),
                CellFilter::Inner(Margin::new(1, 1)),
            ])),
        ),
    ];

    for (name, filter) in test_cases {
        bench_effect_with_filter(&mut group, name, filter);
    }

    group.finish();
}

criterion_group!(benches, cell_filter_overhead_benchmark);
criterion_main!(benches);
