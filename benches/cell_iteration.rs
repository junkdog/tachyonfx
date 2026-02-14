use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
};
use tachyonfx::CellIterator;

// Test configurations: (name, rect, buffer_size)
const BENCH_CONFIGS: &[(&str, Rect, Rect)] = &[
    // Full rectangle starting at (0,0)
    (
        "full_rect_50x30",
        Rect::new(0, 0, 50, 30),
        Rect::new(0, 0, 50, 30),
    ),
    // Small rectangle with moderate offset
    (
        "small_rect_10x8_offset_5x3",
        Rect::new(5, 3, 10, 8),
        Rect::new(0, 0, 20, 15),
    ),
    // Large offset rectangle (offset > dimensions)
    (
        "offset_rect_8x6_offset_15x10",
        Rect::new(15, 10, 8, 6),
        Rect::new(0, 0, 30, 20),
    ),
];

fn setup_buffer(buffer_size: Rect) -> Buffer {
    Buffer::empty(buffer_size)
}

fn manual_iteration(buffer: &mut Buffer, rect: Rect) {
    for y in rect.y..rect.bottom() {
        for x in rect.x..rect.right() {
            let pos = Position::new(x, y);
            if let Some(cell) = buffer.cell_mut(pos) {
                core::hint::black_box((pos, cell));
            }
        }
    }
}

fn iterator_iteration(buffer: &mut Buffer, rect: Rect) {
    let mut iter = CellIterator::new(buffer, rect, None);
    for (pos, cell) in &mut iter {
        core::hint::black_box((pos, cell));
    }
}

fn for_each_cell_iteration(buffer: &mut Buffer, rect: Rect) {
    let iter = CellIterator::new(buffer, rect, None);
    iter.for_each_cell(|pos, cell| {
        core::hint::black_box((pos, cell));
    });
}

pub fn cell_iteration_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("cell_iteration");

    for &(name, rect, buffer_size) in BENCH_CONFIGS {
        group.throughput(Throughput::Elements(rect.width as u64 * rect.height as u64));
        // Manual iteration (baseline)
        group.bench_with_input(
            BenchmarkId::new("manual", name),
            &(rect, buffer_size),
            |b, &(rect, buffer_size)| {
                b.iter_with_setup(
                    || setup_buffer(buffer_size),
                    |mut buffer| manual_iteration(&mut buffer, rect),
                );
            },
        );

        // Iterator-based iteration
        group.bench_with_input(
            BenchmarkId::new("iterator", name),
            &(rect, buffer_size),
            |b, &(rect, buffer_size)| {
                b.iter_with_setup(
                    || setup_buffer(buffer_size),
                    |mut buffer| iterator_iteration(&mut buffer, rect),
                );
            },
        );

        // for_each_cell iteration
        group.bench_with_input(
            BenchmarkId::new("for_each_cell", name),
            &(rect, buffer_size),
            |b, &(rect, buffer_size)| {
                b.iter_with_setup(
                    || setup_buffer(buffer_size),
                    |mut buffer| for_each_cell_iteration(&mut buffer, rect),
                );
            },
        );
    }

    group.finish();
}

criterion_group!(benches, cell_iteration_benchmark);
criterion_main!(benches);
