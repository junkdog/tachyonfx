use criterion::{criterion_group, criterion_main, Criterion, Throughput};

// ── HSL round-trip (current approach) ────────────────────────────────

/// Current rgb_to_hsl (integer pipeline, copied from color_space.rs).
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

/// Current hsl_to_rgb (direct sector, copied from color_space.rs).
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

/// Adjust lightness via full HSL round-trip.
/// `amount`: -1.0 (full darken) to +1.0 (full lighten).
fn adjust_lightness_hsl(r: u8, g: u8, b: u8, amount: f32) -> (u8, u8, u8) {
    let (h, s, l) = rgb_to_hsl(r, g, b);
    let l = if amount >= 0.0 {
        l + (100.0 - l) * amount // lerp toward 100
    } else {
        l + l * amount // lerp toward 0
    };
    let l = l.clamp(0.0, 100.0);
    hsl_to_rgb(h, s, l)
}

// ── Linear RGB scaling ───────────────────────────────────────────────

/// Darken: scale toward 0. Lighten: lerp toward 255.
/// `amount`: -1.0 (black) to +1.0 (white).
fn adjust_lightness_linear(r: u8, g: u8, b: u8, amount: f32) -> (u8, u8, u8) {
    if amount >= 0.0 {
        // lerp toward white: c' = c + (255 - c) * amount
        let a = (amount * 256.0) as u32;
        let r = r as u32 + (((255 - r as u32) * a) >> 8);
        let g = g as u32 + (((255 - g as u32) * a) >> 8);
        let b = b as u32 + (((255 - b as u32) * a) >> 8);
        (r as u8, g as u8, b as u8)
    } else {
        // scale toward black: c' = c * (1 + amount)
        let factor = ((1.0 + amount) * 256.0) as u32;
        let r = (r as u32 * factor) >> 8;
        let g = (g as u32 * factor) >> 8;
        let b = (b as u32 * factor) >> 8;
        (r as u8, g as u8, b as u8)
    }
}

// ── Weighted luminance + uniform offset ──────────────────────────────

/// Compute approximate perceived luminance (BT.601 weights, integer).
/// Returns 0..255.
#[inline(always)]
fn luminance_i(r: u8, g: u8, b: u8) -> u32 {
    // 77/256 = 0.299, 150/256 = 0.586, 29/256 = 0.113
    (r as u32 * 77 + g as u32 * 150 + b as u32 * 29) >> 8
}

/// Adjust lightness by computing a uniform delta from the luminance error.
/// `amount`: -1.0 (black) to +1.0 (white).
fn adjust_lightness_weighted(r: u8, g: u8, b: u8, amount: f32) -> (u8, u8, u8) {
    let lum = luminance_i(r, g, b);
    let target = if amount >= 0.0 {
        lum + ((255 - lum) as f32 * amount) as u32
    } else {
        ((lum as f32) * (1.0 + amount)) as u32
    };
    let delta = target as i32 - lum as i32;

    let r = (r as i32 + delta).clamp(0, 255) as u8;
    let g = (g as i32 + delta).clamp(0, 255) as u8;
    let b = (b as i32 + delta).clamp(0, 255) as u8;
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

fn bench_lighten(c: &mut Criterion) {
    let colors = test_colors();
    let amount = 0.3_f32; // moderate lighten

    let mut group = c.benchmark_group("lighten_+0.3");
    group.throughput(Throughput::Elements(colors.len() as u64));

    group.bench_function("hsl round-trip", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_lightness_hsl(r, g, bb, amount));
            }
        });
    });

    group.bench_function("linear rgb", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_lightness_linear(r, g, bb, amount));
            }
        });
    });

    group.bench_function("weighted luminance", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_lightness_weighted(r, g, bb, amount));
            }
        });
    });

    group.finish();
}

fn bench_darken(c: &mut Criterion) {
    let colors = test_colors();
    let amount = -0.3_f32; // moderate darken

    let mut group = c.benchmark_group("darken_-0.3");
    group.throughput(Throughput::Elements(colors.len() as u64));

    group.bench_function("hsl round-trip", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_lightness_hsl(r, g, bb, amount));
            }
        });
    });

    group.bench_function("linear rgb", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_lightness_linear(r, g, bb, amount));
            }
        });
    });

    group.bench_function("weighted luminance", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_lightness_weighted(r, g, bb, amount));
            }
        });
    });

    group.finish();
}

fn bench_extreme_lighten(c: &mut Criterion) {
    let colors = test_colors();
    let amount = 0.8_f32;

    let mut group = c.benchmark_group("lighten_+0.8");
    group.throughput(Throughput::Elements(colors.len() as u64));

    group.bench_function("hsl round-trip", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_lightness_hsl(r, g, bb, amount));
            }
        });
    });

    group.bench_function("linear rgb", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_lightness_linear(r, g, bb, amount));
            }
        });
    });

    group.bench_function("weighted luminance", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_lightness_weighted(r, g, bb, amount));
            }
        });
    });

    group.finish();
}

fn bench_extreme_darken(c: &mut Criterion) {
    let colors = test_colors();
    let amount = -0.8_f32;

    let mut group = c.benchmark_group("darken_-0.8");
    group.throughput(Throughput::Elements(colors.len() as u64));

    group.bench_function("hsl round-trip", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_lightness_hsl(r, g, bb, amount));
            }
        });
    });

    group.bench_function("linear rgb", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_lightness_linear(r, g, bb, amount));
            }
        });
    });

    group.bench_function("weighted luminance", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(adjust_lightness_weighted(r, g, bb, amount));
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_lighten,
    bench_darken,
    bench_extreme_lighten,
    bench_extreme_darken,
);
criterion_main!(benches);
