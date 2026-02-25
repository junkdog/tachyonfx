// benches/parabolic_sin.rs
use core::f32::consts::TAU;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use micromath::F32Ext;
use tachyonfx::parabolic_sin;

pub fn parabolic_sin_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("sin_approximation");
    group.throughput(Throughput::Elements(1024));

    // sample points: four full periods in radians
    let radians: Vec<f32> = (0..1024)
        .map(|i| i as f32 / 1024.0 * 4.0 * TAU)
        .collect();

    group.bench_function("parabolic_sin", |b| {
        b.iter(|| {
            for &t in &radians {
                core::hint::black_box(parabolic_sin(t));
            }
        });
    });

    group.bench_function("f32_sin", |b| {
        b.iter(|| {
            for &t in &radians {
                core::hint::black_box(t.sin());
            }
        });
    });

    group.bench_function("micromath_sin", |b| {
        b.iter(|| {
            for &t in &radians {
                core::hint::black_box(F32Ext::sin(t));
            }
        });
    });

    group.finish();
}

criterion_group!(benches, parabolic_sin_benchmark);
criterion_main!(benches);
