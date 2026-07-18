//! Ambient effects: aperiodic, never-completing, meant to sit under a UI.
//!
//! Every other effect in this crate is a *transition* — it runs once against a
//! timer and finishes. These are different in two ways that shape the design:
//!
//!   * **They never complete.** [`Shader::done`] is always `false` and there is
//!     no timer, so [`Shader::execute`] accumulates its own elapsed time from
//!     the tick duration. Driving them from a timer's alpha instead would mean
//!     looping it, which restarts the field and leaves a visible seam once a
//!     cycle.
//!   * **They each know which cells they can act on.** An effect that paints an
//!     absolute colour needs blank cells; one that modulates an existing colour
//!     needs cells that carry a glyph. Applied to the wrong cells an effect
//!     runs happily and does nothing at all — the failure mode is silence, so
//!     the constructors set a default filter rather than leaving it to the
//!     caller. An explicit `.with_filter()` still overrides it.

use alloc::boxed::Box;

use ratatui_core::{buffer::Buffer, layout::Rect, style::Color};

use crate::{
    CellFilter, ColorSpace, Duration, ToRgbComponents, cell_filter::FilterProcessor,
    default_shader_impl, effect_timer::EffectTimer, math, noise, shader::Shader,
};

/// Which ambient effect an [`Ambient`] shader runs.
///
/// One shader type rather than seven keeps the elapsed-time bookkeeping in a
/// single place, and keeps the whole thing `Clone` without boxing closures.
#[derive(Clone, Copy, Debug)]
pub(super) enum Kind {
    NoiseField {
        from: Color,
        to: Color,
        scale: f32,
        speed: f32,
        octaves: u32,
    },
    ColorWave {
        stops: [Color; 3],
        speed: f32,
        spread: f32,
    },
    Glow {
        color: Color,
        radius: f32,
        falloff: f32,
        period: f32,
    },
    Breathe {
        intensity: f32,
        period: f32,
    },
    Flicker {
        intensity: f32,
        speed: f32,
    },
    Shimmer {
        intensity: f32,
        scale: f32,
        speed: f32,
    },
    Drift {
        from: Color,
        to: Color,
        speed: f32,
    },
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::NoiseField { .. } => "noise_field",
            Kind::ColorWave { .. } => "color_wave",
            Kind::Glow { .. } => "glow",
            Kind::Breathe { .. } => "breathe",
            Kind::Flicker { .. } => "flicker",
            Kind::Shimmer { .. } => "shimmer",
            Kind::Drift { .. } => "drift",
        }
    }

    /// The cells this effect can actually act on.
    pub(super) fn default_filter(self) -> CellFilter {
        match self {
            // Paints an absolute colour: fills the blanks, leaves text legible.
            //
            // `Not(NonEmpty)`, not `Not(Text)`: `Text` matches the space
            // character too, so `Not(Text)` selects neither blanks nor glyphs
            // and the effect paints nothing at all.
            Kind::NoiseField { .. } | Kind::ColorWave { .. } | Kind::Glow { .. } => {
                CellFilter::Not(Box::new(CellFilter::NonEmpty))
            },
            // Modulates an existing colour: needs a glyph to act on.
            Kind::Breathe { .. }
            | Kind::Flicker { .. }
            | Kind::Shimmer { .. }
            | Kind::Drift { .. } => CellFilter::NonEmpty,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct Ambient {
    kind: Kind,
    /// Seconds since the effect started. Monotonic and unbounded — the effects
    /// here are all periodic in their own parameters, so it never needs wrapping.
    elapsed: f32,
    area: Option<Rect>,
    color_space: ColorSpace,
    cell_filter: Option<FilterProcessor>,
}

/// Builds an ambient effect, pre-filtered to the cells its kind can act on.
pub(super) fn ambient(kind: Kind) -> crate::Effect {
    use crate::IntoEffect;
    let filter = kind.default_filter();
    Ambient::new(kind).into_effect().with_filter(filter)
}

impl Ambient {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            elapsed: 0.0,
            area: None,
            color_space: ColorSpace::Rgb,
            cell_filter: None,
        }
    }
}

fn lerp_rgb(from: Color, to: Color, t: f32) -> Color {
    let (ar, ag, ab) = from.to_rgb();
    let (br, bg, bb) = to.to_rgb();
    let t = t.clamp(0.0, 1.0);
    let mix = |a: u8, b: u8| math::round(a as f32 + (b as f32 - a as f32) * t) as u8;
    Color::Rgb(mix(ar, br), mix(ag, bg), mix(ab, bb))
}

/// Scales a colour's brightness. Above 1.0 moves towards white, below towards
/// black; 1.0 leaves it untouched.
fn scale_brightness(color: Color, factor: f32) -> Color {
    if factor >= 1.0 {
        lerp_rgb(color, Color::Rgb(255, 255, 255), (factor - 1.0).min(1.0))
    } else {
        lerp_rgb(color, Color::Rgb(0, 0, 0), 1.0 - factor)
    }
}

impl Shader for Ambient {
    default_shader_impl!(area, filter, color_space, clone);

    fn name(&self) -> &'static str {
        self.kind.name()
    }

    /// Ambient effects never finish; the caller decides when to stop rendering
    /// them.
    fn done(&self) -> bool {
        false
    }

    fn timer(&self) -> Option<EffectTimer> {
        None
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
    }

    fn execute(&mut self, duration: Duration, area: Rect, buf: &mut Buffer) {
        self.elapsed += duration.as_secs_f32();

        let kind = self.kind;
        let time = self.elapsed;
        let cell_iter = self.cell_iter(buf, area);

        match kind {
            Kind::NoiseField {
                from,
                to,
                scale,
                speed,
                octaves,
            } => cell_iter.for_each_cell(move |pos, cell| {
                let t = noise::fbm2(
                    pos.x as f32 * scale,
                    pos.y as f32 * scale + time * speed,
                    octaves,
                );
                cell.set_bg(lerp_rgb(from, to, t));
            }),

            Kind::ColorWave {
                stops,
                speed,
                spread,
            } => cell_iter.for_each_cell(move |pos, cell| {
                let n = stops.len() as f32;
                let phase = math::rem_euclid(pos.x as f32 * spread + time * speed, n);
                let index = math::floor(phase) as usize % stops.len();
                let next = (index + 1) % stops.len();
                let frac = phase - math::floor(phase);
                cell.set_bg(lerp_rgb(stops[index], stops[next], frac));
            }),

            Kind::Glow {
                color,
                radius,
                falloff,
                period,
            } => {
                let cx = area.x as f32 + area.width as f32 / 2.0;
                let cy = area.y as f32 + area.height as f32 / 2.0;
                let swell = noise::breathe(time / period);

                cell_iter.for_each_cell(move |pos, cell| {
                    let dx = pos.x as f32 - cx;
                    // Terminal cells are roughly twice as tall as they are
                    // wide, so an unscaled radius draws a vertical ellipse.
                    let dy = (pos.y as f32 - cy) * 2.0;
                    let distance = math::sqrt(dx * dx + dy * dy);
                    let intensity =
                        (1.0 - math::powf(distance / radius, falloff)).clamp(0.0, 1.0);
                    cell.set_bg(lerp_rgb(cell.bg, color, intensity * swell));
                });
            },

            Kind::Breathe { intensity, period } => {
                // Map the 0..1 curve onto 1-intensity ..= 1.
                let factor = 1.0 - intensity * (1.0 - noise::breathe(time / period));
                cell_iter.for_each_cell(move |_, cell| {
                    cell.set_fg(scale_brightness(cell.fg, factor));
                });
            },

            Kind::Flicker { intensity, speed } => {
                cell_iter.for_each_cell(move |pos, cell| {
                    let factor = noise::flicker(time * speed, pos.x as f32 * 17.31, intensity);
                    cell.set_fg(scale_brightness(cell.fg, factor));
                });
            },

            Kind::Shimmer {
                intensity,
                scale,
                speed,
            } => cell_iter.for_each_cell(move |pos, cell| {
                let n = noise::fbm2(pos.x as f32 * scale, time * speed + pos.y as f32 * 0.5, 2);
                // Centred on 1.0 so text brightens and dims around its true
                // colour rather than only ever getting darker.
                let factor = 1.0 + intensity * (n - 0.5);
                cell.set_fg(scale_brightness(cell.fg, factor));
            }),

            Kind::Drift { from, to, speed } => {
                let t = noise::noise1(time * speed);
                let color = lerp_rgb(from, to, t);
                cell_iter.for_each_cell(move |_, cell| {
                    cell.set_fg(color);
                });
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui_core::{buffer::Buffer, layout::Rect, style::Color};

    use crate::{CellFilter, Duration, fx};

    fn area() -> Rect {
        Rect::new(0, 0, 12, 4)
    }

    /// A buffer with one line of text and three blank ones, so both families of
    /// effect have something to act on.
    fn buffer() -> Buffer {
        let mut buf = Buffer::empty(area());
        buf.set_string(0, 1, "hello world", ratatui_core::style::Style::default());
        buf
    }

    fn painting() -> [crate::Effect; 3] {
        [
            fx::noise_field(Color::Black, Color::Blue, 0.2, 0.05, 3),
            fx::color_wave(Color::Red, Color::Green, Color::Blue, 0.6, 0.06),
            fx::glow(Color::Cyan, 8.0, 2.0, 3.0),
        ]
    }

    fn modulating() -> [crate::Effect; 4] {
        [
            fx::breathe(0.5, 3.0),
            fx::flicker(0.4, 2.0),
            fx::shimmer(0.5, 0.15, 0.8),
            fx::drift(Color::Blue, Color::Magenta, 0.35),
        ]
    }

    /// The defining property: these are meant to sit under a UI indefinitely,
    /// so nothing should ever mark them finished.
    #[test]
    fn ambient_effects_never_complete() {
        for mut effect in painting().into_iter().chain(modulating()) {
            let mut buf = buffer();
            for _ in 0..100 {
                effect.process(Duration::from_millis(100), &mut buf, area());
            }
            assert!(
                !effect.done() && effect.running(),
                "{} should still be running after 10s",
                effect.name()
            );
        }
    }

    /// Applied to the wrong cells an effect runs and does nothing at all, with
    /// no error anywhere -- so the constructors pick the filter, and that choice
    /// is worth asserting.
    #[test]
    fn constructors_target_the_cells_they_can_act_on() {
        for effect in painting() {
            assert!(
                matches!(effect.cell_filter(), Some(CellFilter::Not(inner)) if **inner == CellFilter::NonEmpty),
                "{} paints, so it must target blank cells",
                effect.name()
            );
        }
        for effect in modulating() {
            assert!(
                matches!(effect.cell_filter(), Some(CellFilter::NonEmpty)),
                "{} modulates, so it must target glyph cells",
                effect.name()
            );
        }
    }

    #[test]
    fn painting_effects_fill_blanks_and_leave_text_alone() {
        for mut effect in painting() {
            let before = buffer();
            let mut after = buffer();
            effect.process(Duration::from_millis(500), &mut after, area());

            assert_ne!(
                before[(0, 0)].bg,
                after[(0, 0)].bg,
                "{} should have painted the blank row",
                effect.name()
            );
            assert_eq!(
                before[(0, 1)].symbol(),
                after[(0, 1)].symbol(),
                "{} must not disturb glyphs",
                effect.name()
            );
        }
    }

    #[test]
    fn modulating_effects_reach_text() {
        for mut effect in modulating() {
            let before = buffer();
            let mut after = buffer();
            // Two ticks: some of these start at a neutral point.
            effect.process(Duration::from_millis(600), &mut after, area());
            effect.process(Duration::from_millis(600), &mut after, area());

            assert_ne!(
                before[(0, 1)].fg,
                after[(0, 1)].fg,
                "{} should have modulated the text row",
                effect.name()
            );
        }
    }

    /// The failure this catches: an effect that compiles, paints, and reports
    /// itself as running, while being visually static. `noise_field` shipped
    /// that way -- its documented speed was so low the field drifted about one
    /// RGB unit per second, which reads as a still image.
    ///
    /// Measured at the parameters used in each constructor's doc example, so
    /// the thing being pinned is what a user actually copies.
    #[test]
    fn every_effect_produces_visible_motion() {
        /// Mean absolute RGB change per channel per cell over one second.
        /// Around 1 is imperceptible; 10 is a clear but gentle drift.
        const MIN_DELTA: f32 = 8.0;

        let cases: [(crate::Effect, u16, bool); 7] = [
            (fx::noise_field(Color::Rgb(10, 12, 30), Color::Rgb(110, 60, 160), 0.2, 1.2, 3), 2, false),
            (fx::color_wave(Color::Red, Color::Green, Color::Blue, 1.2, 0.06), 2, false),
            (fx::glow(Color::Cyan, 20.0, 2.0, 3.0), 2, false),
            (fx::breathe(0.4, 3.0), 5, true),
            (fx::flicker(0.15, 2.0), 5, true),
            (fx::shimmer(0.4, 0.15, 0.8), 5, true),
            (fx::drift(Color::Blue, Color::Magenta, 1.0), 5, true),
        ];

        for (mut effect, row, foreground) in cases {
            let name = effect.name();
            let area = Rect::new(0, 0, 40, 8);
            let mut buf = Buffer::empty(area);
            buf.set_string(
                0,
                5,
                "abcdefghij".repeat(4).as_str(),
                ratatui_core::style::Style::default().fg(Color::Rgb(150, 190, 220)),
            );

            let sample = |b: &Buffer| -> alloc::vec::Vec<(i32, i32, i32)> {
                (0..40)
                    .map(|x| {
                        let c = if foreground { b[(x, row)].fg } else { b[(x, row)].bg };
                        match c {
                            Color::Rgb(r, g, bl) => (i32::from(r), i32::from(g), i32::from(bl)),
                            _ => (0, 0, 0),
                        }
                    })
                    .collect()
            };

            effect.process(Duration::from_millis(33), &mut buf, area);
            let before = sample(&buf);
            for _ in 0..30 {
                effect.process(Duration::from_millis(33), &mut buf, area);
            }
            let after = sample(&buf);

            let total: i32 = before
                .iter()
                .zip(&after)
                .map(|(a, b)| (a.0 - b.0).abs() + (a.1 - b.1).abs() + (a.2 - b.2).abs())
                .sum();
            let delta = total as f32 / (40.0 * 3.0);

            assert!(
                delta >= MIN_DELTA,
                "{name} moved {delta:.2} per channel in 1s, below the {MIN_DELTA} \
                 visibility floor -- it will look static"
            );
        }
    }

    /// Every constructor is reachable from a default DSL, which is what lets an
    /// application expose them as configuration rather than code.
    #[cfg(feature = "dsl")]
    #[test]
    fn effects_compile_from_the_default_dsl() {
        for source in [
            "fx::noise_field(Color::Black, Color::Blue, 0.2, 0.05, 3)",
            "fx::color_wave(Color::Red, Color::Green, Color::Blue, 0.6, 0.06)",
            "fx::glow(Color::Cyan, 20.0, 2.0, 3.0)",
            "fx::breathe(0.4, 3.0)",
            "fx::flicker(0.15, 2.0)",
            "fx::shimmer(0.4, 0.15, 0.8)",
            "fx::drift(Color::Blue, Color::Magenta, 0.35)",
        ] {
            let dsl = crate::dsl::EffectDsl::new();
            assert!(
                dsl.compiler().compile(source).is_ok(),
                "should compile: {source}"
            );
        }
    }
}


