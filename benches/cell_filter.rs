// benches/cell_filter.rs
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use ratatui::{
    buffer::Buffer,
    layout,
    layout::{Constraint, Direction, Margin, Rect},
    prelude::Color,
};
use tachyonfx::{fx, CellFilter, Duration, Effect};

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
            core::hint::black_box(pos);
            core::hint::black_box(cell);
        }
    });

    if let Some(filter) = filter {
        effect = effect.with_filter(filter);
    }

    effect
}

#[allow(clippy::needless_pass_by_value)] // cloned repeatedly in iter_with_setup
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
                effect.process(core::hint::black_box(BENCH_DURATION), &mut buffer, area);
            },
        );
    });
}

pub fn cell_filter_overhead_benchmark(c: &mut Criterion) {
    let area = bench_area();
    let mut group = c.benchmark_group("cell_filter_overhead");
    group.throughput(Throughput::Elements(
        BENCH_WIDTH as u64 * BENCH_HEIGHT as u64,
    ));

    // Baseline - Raw buffer iteration with no effect framework overhead
    group.bench_function("raw_no_filter", |b| {
        b.iter_with_setup(
            || Buffer::empty(area),
            |buffer| {
                // This is the absolute baseline - just iterating through the buffer
                for y in 0..BENCH_HEIGHT {
                    for x in 0..BENCH_WIDTH {
                        core::hint::black_box(&buffer[(x, y)]);
                    }
                }
            },
        );
    });

    // Benchmark different filter configurations
    use CellFilter::*;
    let test_cases = [
        ("filter_plain", None),
        ("filter_all", Some(All)),
        (
            "filter_all_of_inner_color",
            Some(AllOf(vec![FgColor(Color::Red), Inner(Margin::new(1, 1))])),
        ),
        (
            "filter_allof_with_not",
            Some(AllOf(vec![
                Inner(Margin::new(2, 2)),
                Not(Box::new(Area(Rect::new(20, 20, 60, 60)))),
                AnyOf(vec![Outer(Margin::new(5, 5)), Inner(Margin::new(10, 10))]),
            ])),
        ),
        (
            "filter_complex_position",
            Some(NoneOf(vec![
                AllOf(vec![
                    Layout(
                        layout::Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)]),
                        0,
                    ),
                    Inner(Margin::new(3, 3)),
                ]),
                AnyOf(vec![
                    Area(Rect::new(0, 0, 30, 30)),
                    Area(Rect::new(70, 70, 30, 30)),
                    Not(Box::new(Outer(Margin::new(8, 8)))),
                ]),
                AllOf(vec![
                    Layout(
                        layout::Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)]),
                        1,
                    ),
                    Not(Box::new(Inner(Margin::new(15, 15)))),
                ]),
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
