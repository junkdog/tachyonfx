use criterion::{criterion_group, criterion_main, Criterion, Throughput};

// ── HSL round-trip (current approach) ────────────────────────────────

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let sum = max as u16 + min as u16;

    if delta == 0 {
        return (0.0, 0.0, sum as f32 * (50.0 / 255.0));
    }

    let l = sum as f32 * (50.0 / 255.0);

    let abs_diff = sum.abs_diff(255);
    let denom = (255 - abs_diff) as f32;

    let inv_delta = 1.0 / delta as f32;
    let inv_denom = 1.0 / denom;

    let s = delta as f32 * 100.0 * inv_denom;

    let hr = (g as f32 - b as f32) * inv_delta;
    let hg = (b as f32 - r as f32) * inv_delta + 2.0;
    let hb = (r as f32 - g as f32) * inv_delta + 4.0;

    let r_mask = ((r >= g) as u8 & (r >= b) as u8) as f32;
    let g_mask = (g >= b) as u8 as f32 * (1.0 - r_mask);
    let b_mask = 1.0 - r_mask - g_mask;

    let hr = hr + (g < b) as u8 as f32 * 6.0;
    let h = (r_mask * hr + g_mask * hg + b_mask * hb) * 60.0;

    (h, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let s = s / 100.0;
    let l = l / 100.0;

    if s == 0.0 {
        let gray = (l * 255.0 + 0.5) as u8;
        return (gray, gray, gray);
    }

    let h = (h % 360.0) / 60.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let m = l - c * 0.5;

    let sector = h as u32;
    let f = h - sector as f32;
    let h2 = (sector & 1) as f32 + f;
    let x = c * (1.0 - (h2 - 1.0).abs());

    let (r, g, b) = match sector {
        0 => (c + m, x + m, m),
        1 => (x + m, c + m, m),
        2 => (m, c + m, x + m),
        3 => (m, x + m, c + m),
        4 => (x + m, m, c + m),
        _ => (c + m, m, x + m),
    };

    (
        (r * 255.0 + 0.5) as u8,
        (g * 255.0 + 0.5) as u8,
        (b * 255.0 + 0.5) as u8,
    )
}

/// Adjust saturation via full HSL round-trip.
/// `factor`: 0.0 = fully desaturated, 1.0 = original, >1.0 = oversaturated.
fn adjust_saturation_hsl(r: u8, g: u8, b: u8, factor: f32) -> (u8, u8, u8) {
    let (h, s, l) = rgb_to_hsl(r, g, b);
    let s = (s * factor).clamp(0.0, 100.0);
    hsl_to_rgb(h, s, l)
}

// ── Lerp toward average gray ─────────────────────────────────────────

/// Adjust saturation by lerping toward the simple average (r+g+b)/3.
/// `factor`: 0.0 = fully desaturated, 1.0 = original, >1.0 = oversaturated.
fn adjust_saturation_avg(r: u8, g: u8, b: u8, factor: f32) -> (u8, u8, u8) {
    let avg = ((r as u32 + g as u32 + b as u32) / 3) as i32;
    let f = (factor * 256.0) as i32;

    // c' = avg + (c - avg) * factor
    let r = (avg + (((r as i32 - avg) * f) >> 8)).clamp(0, 255) as u8;
    let g = (avg + (((g as i32 - avg) * f) >> 8)).clamp(0, 255) as u8;
    let b = (avg + (((b as i32 - avg) * f) >> 8)).clamp(0, 255) as u8;
    (r, g, b)
}

// ── Lerp toward weighted luminance ───────────────────────────────────

/// Adjust saturation by lerping toward BT.601 weighted luminance.
/// `factor`: 0.0 = fully desaturated, 1.0 = original, >1.0 = oversaturated.
fn adjust_saturation_weighted(r: u8, g: u8, b: u8, factor: f32) -> (u8, u8, u8) {
    // BT.601: 77/256 = 0.299, 150/256 = 0.586, 29/256 = 0.113
    let lum = ((r as u32 * 77 + g as u32 * 150 + b as u32 * 29) >> 8) as i32;
    let f = (factor * 256.0) as i32;

    // c' = lum + (c - lum) * factor
    let r = (lum + (((r as i32 - lum) * f) >> 8)).clamp(0, 255) as u8;
    let g = (lum + (((g as i32 - lum) * f) >> 8)).clamp(0, 255) as u8;
    let b = (lum + (((b as i32 - lum) * f) >> 8)).clamp(0, 255) as u8;
    (r, g, b)
}

// ── Test color set ───────────────────────────────────────────────────

fn test_colors() -> Vec<(u8, u8, u8)> {
    let steps: &[u8] = &[0, 51, 102, 153, 204, 255];
    let mut colors = Vec::with_capacity(256);

    for &r in steps {
        for &g in steps {
            for &b in steps {
                colors.push((r, g, b));
            }
        }
    }

    let grays: &[u8] = &[
        8, 18, 28, 38, 48, 58, 68, 78, 88, 98, 108, 118, 128, 138, 148, 158, 168, 178, 188, 198,
    ];
    for &v in grays {
        colors.push((v, v, v));
    }

    let extras: &[(u8, u8, u8)] = &[
        (1, 0, 0),
        (0, 1, 0),
        (0, 0, 1),
        (254, 255, 255),
        (255, 254, 255),
        (255, 255, 254),
        (127, 128, 129),
        (129, 128, 127),
        (64, 65, 63),
        (191, 190, 192),
        (17, 85, 170),
        (170, 85, 17),
        (85, 170, 17),
        (85, 17, 170),
        (17, 170, 85),
        (170, 17, 85),
        (1, 1, 1),
        (254, 254, 254),
        (42, 142, 242),
        (242, 142, 42),
    ];
    colors.extend_from_slice(extras);
    colors.truncate(256);
    colors
}

// ── Benchmarks ───────────────────────────────────────────────────────

fn bench_desaturate(c: &mut Criterion) {
    let colors = test_colors();
    let factor = 0.3_f32; // strong desaturation

    let mut group = c.benchmark_group("desaturate_0.3");
    group.throughput(Throughput::Elements(colors.len() as u64));

    group.bench_function("hsl round-trip", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_saturation_hsl(r, g, bb, factor));
            }
        });
    });

    group.bench_function("avg gray lerp", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_saturation_avg(r, g, bb, factor));
            }
        });
    });

    group.bench_function("weighted luminance lerp", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_saturation_weighted(r, g, bb, factor));
            }
        });
    });

    group.finish();
}

fn bench_mild_desaturate(c: &mut Criterion) {
    let colors = test_colors();
    let factor = 0.7_f32;

    let mut group = c.benchmark_group("desaturate_0.7");
    group.throughput(Throughput::Elements(colors.len() as u64));

    group.bench_function("hsl round-trip", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_saturation_hsl(r, g, bb, factor));
            }
        });
    });

    group.bench_function("avg gray lerp", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_saturation_avg(r, g, bb, factor));
            }
        });
    });

    group.bench_function("weighted luminance lerp", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_saturation_weighted(r, g, bb, factor));
            }
        });
    });

    group.finish();
}

fn bench_oversaturate(c: &mut Criterion) {
    let colors = test_colors();
    let factor = 1.5_f32;

    let mut group = c.benchmark_group("oversaturate_1.5");
    group.throughput(Throughput::Elements(colors.len() as u64));

    group.bench_function("hsl round-trip", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_saturation_hsl(r, g, bb, factor));
            }
        });
    });

    group.bench_function("avg gray lerp", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_saturation_avg(r, g, bb, factor));
            }
        });
    });

    group.bench_function("weighted luminance lerp", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_saturation_weighted(r, g, bb, factor));
            }
        });
    });

    group.finish();
}

fn bench_full_desaturate(c: &mut Criterion) {
    let colors = test_colors();
    let factor = 0.0_f32;

    let mut group = c.benchmark_group("desaturate_0.0");
    group.throughput(Throughput::Elements(colors.len() as u64));

    group.bench_function("hsl round-trip", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_saturation_hsl(r, g, bb, factor));
            }
        });
    });

    group.bench_function("avg gray lerp", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_saturation_avg(r, g, bb, factor));
            }
        });
    });

    group.bench_function("weighted luminance lerp", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_saturation_weighted(r, g, bb, factor));
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_desaturate,
    bench_mild_desaturate,
    bench_oversaturate,
    bench_full_desaturate,
);
criterion_main!(benches);
