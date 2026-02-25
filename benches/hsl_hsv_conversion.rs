use criterion::{criterion_group, criterion_main, Criterion, Throughput};

// ── v1: original implementations ─────────────────────────────────────

/// Original branching RGB→HSL (preserved for benchmarking).
fn rgb_to_hsl_v1(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let l = (max + min) / 2.0;

    if delta == 0.0 {
        return (0.0, 0.0, l * 100.0);
    }

    let s = if l <= 0.5 { delta / (max + min) } else { delta / (2.0 - max - min) };

    let h = if max == r {
        (g - b) / delta + (if g < b { 6.0 } else { 0.0 })
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };

    (h * 60.0, s * 100.0, l * 100.0)
}

/// Original closure-based HSL→RGB (preserved for benchmarking).
fn hsl_to_rgb_v1(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let h = h % 360.0;
    let s = s / 100.0;
    let l = l / 100.0;

    if s == 0.0 {
        let gray = (l * 255.0 + 0.5) as u8;
        return (gray, gray, gray);
    }

    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };

    let p = 2.0 * l - q;

    let to_rgb_component = |t: f32| -> u8 {
        let t = if t < 0.0 {
            t + 1.0
        } else if t > 1.0 {
            t - 1.0
        } else {
            t
        };

        let value = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };

        (value * 255.0 + 0.5) as u8
    };

    let h = h / 360.0;

    let r = to_rgb_component(h + 1.0 / 3.0);
    let g = to_rgb_component(h);
    let b = to_rgb_component(h - 1.0 / 3.0);

    (r, g, b)
}

// ── v2: branchless float ───────────────────────────────────────────────

/// Branchless RGB→HSL: single division, arithmetic masking for hue selection,
/// branchless saturation via abs().
fn rgb_to_hsl_v2(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rf = r as f32 * (1.0 / 255.0);
    let gf = g as f32 * (1.0 / 255.0);
    let bf = b as f32 * (1.0 / 255.0);

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;
    let sum = max + min;
    let l = sum * 0.5;

    if delta == 0.0 {
        return (0.0, 0.0, l * 100.0);
    }

    let s = delta / (1.0 - (sum - 1.0).abs());
    let inv_delta = 1.0 / delta;

    let hr = (gf - bf) * inv_delta;
    let hg = (bf - rf) * inv_delta + 2.0;
    let hb = (rf - gf) * inv_delta + 4.0;

    let r_mask = (max == rf) as u8 as f32;
    let g_mask = (max == gf) as u8 as f32 * (1.0 - r_mask);
    let b_mask = 1.0 - r_mask - g_mask;

    let hr = hr + (gf < bf) as u8 as f32 * 6.0;
    let h = (r_mask * hr + g_mask * hg + b_mask * hb) * 60.0;

    (h, s * 100.0, l * 100.0)
}

/// Direct sector HSL→RGB: compute chroma/x once, single 6-way match
/// instead of 3 closure calls with 4 branches each.
fn hsl_to_rgb_v2(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
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

// ── v3: integer pipeline ───────────────────────────────────────────────

/// Integer pipeline RGB→HSL (no LUT): integer max/min/delta, branchless
/// hue + saturation, two independent float divisions (pipelined by CPU).
fn rgb_to_hsl_v3(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min; // u8
    let sum = max as u16 + min as u16; // 0..510

    if delta == 0 {
        return (0.0, 0.0, sum as f32 * (50.0 / 255.0));
    }

    let l = sum as f32 * (50.0 / 255.0);

    // Saturation denom computed in integer: 255 - |sum - 255|
    let denom = (255 - sum.abs_diff(255)) as f32;

    // Two independent divisions — CPU pipelines these (throughput ~3 cy each)
    let inv_delta = 1.0 / delta as f32;
    let inv_denom = 1.0 / denom;

    let s = delta as f32 * 100.0 * inv_denom;

    // Hue: channel diffs as integer → single multiply by precomputed inv_delta
    let hr = (g as f32 - b as f32) * inv_delta;
    let hg = (b as f32 - r as f32) * inv_delta + 2.0;
    let hb = (r as f32 - g as f32) * inv_delta + 4.0;

    // Branchless masks using exact u8 comparisons
    let r_mask = ((r >= g) as u8 & (r >= b) as u8) as f32;
    let g_mask = (g >= b) as u8 as f32 * (1.0 - r_mask);
    let b_mask = 1.0 - r_mask - g_mask;

    let hr = hr + (g < b) as u8 as f32 * 6.0;
    let h = (r_mask * hr + g_mask * hg + b_mask * hb) * 60.0;

    (h, s, l)
}

/// Fixed-point HSL→RGB: 15-bit fixed-point arithmetic (Q0.15) for all
/// intermediate values. Only float needed for h→sector conversion.
const FP_ONE: u32 = 32768; // 1.0 in Q0.15

fn hsl_to_rgb_v3(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    // Convert s,l from percentage (0..100) to fixed-point (0..32768)
    let s_fp = (s * (FP_ONE as f32 / 100.0)) as u32;
    let l_fp = (l * (FP_ONE as f32 / 100.0)) as u32;

    if s_fp == 0 {
        let gray = ((l_fp * 255 + FP_ONE / 2) >> 15) as u8;
        return (gray, gray, gray);
    }

    // Chroma: c = (1 - |2L - 1|) * S  [all in Q0.15]
    let double_l = 2 * l_fp; // 0..65536
    let abs_term = double_l.abs_diff(FP_ONE);
    let c = ((FP_ONE - abs_term) * s_fp) >> 15; // 0..32768

    // m = L - C/2
    let m = l_fp - c / 2;

    // Sector and fraction from h (the one place we need float)
    let h_norm = (h % 360.0) / 60.0; // 0..6
    let sector = h_norm as u32;
    let f = ((h_norm - sector as f32) * FP_ONE as f32) as u32; // 0..32768

    // x = c * (1 - |h mod 2 - 1|)  [Q0.15]
    let h2 = (sector & 1) * FP_ONE + f; // 0..65536
    let abs_h2 = h2.abs_diff(FP_ONE);
    let x = (c * (FP_ONE - abs_h2)) >> 15;

    let (rf, gf, bf) = match sector {
        0 => (c + m, x + m, m),
        1 => (x + m, c + m, m),
        2 => (m, c + m, x + m),
        3 => (m, x + m, c + m),
        4 => (x + m, m, c + m),
        _ => (c + m, m, x + m),
    };

    // Q0.15 → u8:  (v * 255 + 16384) >> 15
    let to_u8 = |v: u32| ((v * 255 + FP_ONE / 2) >> 15) as u8;
    (to_u8(rf), to_u8(gf), to_u8(bf))
}

// ── Test color set ─────────────────────────────────────────────────────

/// Generates 256 colors by sampling a 6x6x6 RGB cube plus grays
/// and near-boundary values.
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

// ── Batch benchmarks (256 colors) ──────────────────────────────────────

fn bench_rgb_to_hsl(c: &mut Criterion) {
    let colors = test_colors();
    let mut group = c.benchmark_group("rgb_to_hsl");
    group.throughput(Throughput::Elements(colors.len() as u64));

    group.bench_function("v1 original", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(rgb_to_hsl_v1(r, g, bb));
            }
        });
    });

    group.bench_function("v2 branchless", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(rgb_to_hsl_v2(r, g, bb));
            }
        });
    });

    group.bench_function("v3 int pipeline", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(rgb_to_hsl_v3(r, g, bb));
            }
        });
    });

    group.finish();
}

fn bench_hsl_to_rgb(c: &mut Criterion) {
    let colors = test_colors();
    let hsl_values: Vec<(f32, f32, f32)> = colors
        .iter()
        .map(|&(r, g, b)| rgb_to_hsl_v3(r, g, b))
        .collect();

    let mut group = c.benchmark_group("hsl_to_rgb");
    group.throughput(Throughput::Elements(hsl_values.len() as u64));

    group.bench_function("v1 original", |b| {
        b.iter(|| {
            for &(h, s, l) in &hsl_values {
                core::hint::black_box(hsl_to_rgb_v1(h, s, l));
            }
        });
    });

    group.bench_function("v2 direct-sector", |b| {
        b.iter(|| {
            for &(h, s, l) in &hsl_values {
                core::hint::black_box(hsl_to_rgb_v2(h, s, l));
            }
        });
    });

    group.bench_function("v3 fixed-point", |b| {
        b.iter(|| {
            for &(h, s, l) in &hsl_values {
                core::hint::black_box(hsl_to_rgb_v3(h, s, l));
            }
        });
    });

    group.finish();
}

fn bench_round_trip(c: &mut Criterion) {
    let colors = test_colors();
    let mut group = c.benchmark_group("round_trip");
    group.throughput(Throughput::Elements(colors.len() as u64));

    group.bench_function("v1 original", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                let (h, s, l) = rgb_to_hsl_v1(r, g, bb);
                core::hint::black_box(hsl_to_rgb_v1(h, s, l));
            }
        });
    });

    group.bench_function("v2", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                let (h, s, l) = rgb_to_hsl_v2(r, g, bb);
                core::hint::black_box(hsl_to_rgb_v2(h, s, l));
            }
        });
    });

    group.bench_function("v3", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                let (h, s, l) = rgb_to_hsl_v3(r, g, bb);
                core::hint::black_box(hsl_to_rgb_v3(h, s, l));
            }
        });
    });

    group.bench_function("v3+v2 combo", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                let (h, s, l) = rgb_to_hsl_v3(r, g, bb);
                core::hint::black_box(hsl_to_rgb_v2(h, s, l));
            }
        });
    });

    group.finish();
}

// ── HSV: current implementations (baseline) ──────────────────────────

/// Current rgb_to_hsv (copied from color_space.rs for benchmarking).
fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };

    let h = if h < 0.0 { h + 360.0 } else { h };

    let s = if max == 0.0 { 0.0 } else { delta / max };

    let v = max;

    (h, s * 100.0, v * 100.0)
}

/// Current hsv_to_rgb (copied from color_space.rs for benchmarking).
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let s = s / 100.0;
    let v = v / 100.0;
    let h = h % 360.0;

    if s <= 0.0 {
        let gray = (v * 255.0 + 0.5) as u8;
        return (gray, gray, gray);
    }

    let h = h / 60.0;
    let i = h as i32;
    let f = h - i as f32;

    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };

    (
        (r * 255.0 + 0.5) as u8,
        (g * 255.0 + 0.5) as u8,
        (b * 255.0 + 0.5) as u8,
    )
}

// ── HSV: optimized implementations ───────────────────────────────────

/// Integer-pipeline RGB→HSV: integer max/min/delta with branchless hue
/// selection via arithmetic masks and pipelined float divisions.
/// Same hue logic as rgb_to_hsl v3; HSV saturation = delta/max is simpler.
fn rgb_to_hsv_v2(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min; // u8

    if delta == 0 {
        return (0.0, 0.0, max as f32 * (100.0 / 255.0));
    }

    let v = max as f32 * (100.0 / 255.0);

    // Two independent divisions — CPU pipelines these
    let inv_delta = 1.0 / delta as f32;
    let inv_max = 1.0 / max as f32;

    let s = delta as f32 * 100.0 * inv_max;

    // Branchless hue (same as rgb_to_hsl v3)
    let hr = (g as f32 - b as f32) * inv_delta;
    let hg = (b as f32 - r as f32) * inv_delta + 2.0;
    let hb = (r as f32 - g as f32) * inv_delta + 4.0;

    let r_mask = ((r >= g) as u8 & (r >= b) as u8) as f32;
    let g_mask = (g >= b) as u8 as f32 * (1.0 - r_mask);
    let b_mask = 1.0 - r_mask - g_mask;

    let hr = hr + (g < b) as u8 as f32 * 6.0;
    let h = (r_mask * hr + g_mask * hg + b_mask * hb) * 60.0;

    (h, s, v)
}

/// Direct-sector HSV→RGB: unified c/x/m approach with single 6-way match.
/// HSV: c = V*S, m = V - c (simpler than HSL's chroma formula).
fn hsv_to_rgb_v2(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let s = s / 100.0;
    let v = v / 100.0;

    if s == 0.0 {
        let gray = (v * 255.0 + 0.5) as u8;
        return (gray, gray, gray);
    }

    let h = (h % 360.0) / 60.0;
    let c = v * s;
    let m = v - c;

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

// ── HSV benchmarks ───────────────────────────────────────────────────

fn bench_rgb_to_hsv(c: &mut Criterion) {
    let colors = test_colors();
    let mut group = c.benchmark_group("rgb_to_hsv");
    group.throughput(Throughput::Elements(colors.len() as u64));

    group.bench_function("current", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(rgb_to_hsv(r, g, bb));
            }
        });
    });

    group.bench_function("v2 int pipeline", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                core::hint::black_box(rgb_to_hsv_v2(r, g, bb));
            }
        });
    });

    group.finish();
}

fn bench_hsv_to_rgb(c: &mut Criterion) {
    let colors = test_colors();
    let hsv_values: Vec<(f32, f32, f32)> = colors
        .iter()
        .map(|&(r, g, b)| rgb_to_hsv(r, g, b))
        .collect();

    let mut group = c.benchmark_group("hsv_to_rgb");
    group.throughput(Throughput::Elements(hsv_values.len() as u64));

    group.bench_function("current", |b| {
        b.iter(|| {
            for &(h, s, v) in &hsv_values {
                core::hint::black_box(hsv_to_rgb(h, s, v));
            }
        });
    });

    group.bench_function("v2 direct-sector", |b| {
        b.iter(|| {
            for &(h, s, v) in &hsv_values {
                core::hint::black_box(hsv_to_rgb_v2(h, s, v));
            }
        });
    });

    group.finish();
}

fn bench_hsv_round_trip(c: &mut Criterion) {
    let colors = test_colors();
    let mut group = c.benchmark_group("hsv_round_trip");
    group.throughput(Throughput::Elements(colors.len() as u64));

    group.bench_function("current", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                let (h, s, v) = rgb_to_hsv(r, g, bb);
                core::hint::black_box(hsv_to_rgb(h, s, v));
            }
        });
    });

    group.bench_function("v2", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                let (h, s, v) = rgb_to_hsv_v2(r, g, bb);
                core::hint::black_box(hsv_to_rgb_v2(h, s, v));
            }
        });
    });

    group.bench_function("v2+v1 combo", |b| {
        b.iter(|| {
            for &(r, g, bb) in &colors {
                let (h, s, v) = rgb_to_hsv_v2(r, g, bb);
                core::hint::black_box(hsv_to_rgb(h, s, v));
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_rgb_to_hsl,
    bench_hsl_to_rgb,
    bench_round_trip,
    bench_rgb_to_hsv,
    bench_hsv_to_rgb,
    bench_hsv_round_trip,
);
criterion_main!(benches);
