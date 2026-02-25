// benches/math_functions.rs
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use micromath::F32Ext;

pub fn sqrt_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("sqrt");
    group.throughput(Throughput::Elements(1024));

    let values: Vec<f32> = (0..1024)
        .map(|i| i as f32 / 1024.0 * 100.0)
        .collect();

    group.bench_function("std", |b| {
        b.iter(|| {
            for &v in &values {
                core::hint::black_box(v.sqrt());
            }
        });
    });

    group.bench_function("micromath", |b| {
        b.iter(|| {
            for &v in &values {
                core::hint::black_box(F32Ext::sqrt(v));
            }
        });
    });

    group.finish();
}

pub fn powf_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("powf");
    group.throughput(Throughput::Elements(1024));

    let values: Vec<(f32, f32)> = (0..1024)
        .map(|i| {
            let base = (i as f32 / 1024.0) * 10.0 + 0.1;
            let exp = (i as f32 / 512.0) - 1.0;
            (base, exp)
        })
        .collect();

    group.bench_function("std", |b| {
        b.iter(|| {
            for &(base, exp) in &values {
                core::hint::black_box(base.powf(exp));
            }
        });
    });

    group.bench_function("micromath", |b| {
        b.iter(|| {
            for &(base, exp) in &values {
                core::hint::black_box(F32Ext::powf(base, exp));
            }
        });
    });

    group.finish();
}

pub fn powi_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("powi");
    group.throughput(Throughput::Elements(1024));

    let values: Vec<(f32, i32)> = (0..1024)
        .map(|i| {
            let base = (i as f32 / 1024.0) * 10.0 + 0.1;
            let exp = (i % 7) - 3;
            (base, exp)
        })
        .collect();

    group.bench_function("std", |b| {
        b.iter(|| {
            for &(base, exp) in &values {
                core::hint::black_box(base.powi(exp));
            }
        });
    });

    group.bench_function("micromath", |b| {
        b.iter(|| {
            for &(base, exp) in &values {
                core::hint::black_box(F32Ext::powi(base, exp));
            }
        });
    });

    group.finish();
}

pub fn round_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("round");
    group.throughput(Throughput::Elements(1024));

    let values: Vec<f32> = (0..1024)
        .map(|i| (i as f32 / 100.0) - 5.12)
        .collect();

    group.bench_function("std", |b| {
        b.iter(|| {
            for &v in &values {
                core::hint::black_box(v.round());
            }
        });
    });

    group.bench_function("micromath", |b| {
        b.iter(|| {
            for &v in &values {
                core::hint::black_box(F32Ext::round(v));
            }
        });
    });

    group.finish();
}

pub fn floor_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("floor");
    group.throughput(Throughput::Elements(1024));

    let values: Vec<f32> = (0..1024)
        .map(|i| (i as f32 / 100.0) - 5.12)
        .collect();

    group.bench_function("std", |b| {
        b.iter(|| {
            for &v in &values {
                core::hint::black_box(v.floor());
            }
        });
    });

    group.bench_function("micromath", |b| {
        b.iter(|| {
            for &v in &values {
                core::hint::black_box(F32Ext::floor(v));
            }
        });
    });

    group.finish();
}

pub fn ceil_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("ceil");
    group.throughput(Throughput::Elements(1024));

    let values: Vec<f32> = (0..1024)
        .map(|i| (i as f32 / 100.0) - 5.12)
        .collect();

    group.bench_function("std", |b| {
        b.iter(|| {
            for &v in &values {
                core::hint::black_box(v.ceil());
            }
        });
    });

    group.bench_function("micromath", |b| {
        b.iter(|| {
            for &v in &values {
                core::hint::black_box(F32Ext::ceil(v));
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    sqrt_benchmark,
    powf_benchmark,
    powi_benchmark,
    round_benchmark,
    floor_benchmark,
    ceil_benchmark,
);
criterion_main!(benches);
