use alloc::{boxed::Box, vec::Vec};

use ratatui_core::{
    buffer::{Buffer, Cell},
    layout::{Position, Rect},
    style::Color,
};
use Interpolation::CircOut;

use crate::{
    cell_filter::FilterProcessor, default_shader_impl, effect_timer::EffectTimer,
    fx::sliding_window_alpha::SlidingWindowAlpha, interpolation::Interpolation, shader::Shader,
    CellFilter, ColorCache, ColorSpace, DirectionalVariance, Duration, Motion,
};

#[derive(Clone, Debug)]
pub(super) struct SweepIn {
    gradient_length: u16,
    randomness_extent: u16,
    faded_color: Color,
    timer: EffectTimer,
    direction: Motion,
    area: Option<Rect>,
    cell_filter: Option<FilterProcessor>,
    color_space: ColorSpace,
    rng: crate::SimpleRng,
}

impl SweepIn {
    pub fn new(
        direction: Motion,
        gradient_length: u16,
        randomness: u16,
        faded_color: Color,
        lifetime: EffectTimer,
    ) -> Self {
        Self {
            direction,
            gradient_length,
            randomness_extent: randomness,
            faded_color,
            timer: if direction.flips_timer() { lifetime.mirrored() } else { lifetime },
            area: None,
            cell_filter: None,
            color_space: ColorSpace::default(),
            rng: crate::SimpleRng::new(0),
        }
    }
}

impl Shader for SweepIn {
    default_shader_impl!(area, timer, filter, color_space, clone);

    fn name(&self) -> &'static str {
        if self.timer.is_reversed() ^ self.direction.flips_timer() {
            "sweep_out"
        } else {
            "sweep_in"
        }
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let alpha = self.timer.alpha();
        let direction = self.direction;

        let window_alpha = SlidingWindowAlpha::builder()
            .direction(direction)
            .progress(alpha)
            .area(area)
            .gradient_len(self.gradient_length + self.randomness_extent)
            .build();

        let mut axis_jitter =
            DirectionalVariance::with_rng(self.rng, direction, self.randomness_extent);

        let mut color_cache: ColorCache<u8, 8> = ColorCache::new();

        let mut apply_alpha = |cell: &mut Cell, pos: Position| {
            match window_alpha.alpha(pos) {
                0.0 => {
                    cell.set_fg(self.faded_color);
                    cell.set_bg(self.faded_color);
                },
                1.0 => {}, // nothing to do
                a => {
                    let faded = self.faded_color;
                    let mod_a = CircOut.alpha(a);

                    if cell.fg == Color::Reset {
                        cell.fg = Color::White;
                    };
                    if cell.bg == Color::Reset {
                        cell.bg = Color::Black;
                    };

                    let cache_key = (mod_a.clamp(0.0, 1.0) * 255.0) as u8;
                    let fg = color_cache.memoize_fg(cell.fg, cache_key, |c| {
                        self.color_space.lerp(&faded, c, mod_a)
                    });
                    let bg = color_cache.memoize_bg(cell.bg, cache_key, |c| {
                        self.color_space.lerp(&faded, c, mod_a)
                    });

                    cell.set_fg(fg);
                    cell.set_bg(bg);
                },
            }
        };

        let area = area.intersection(buf.area); // safe area
        let cell_filter = self
            .cell_filter
            .as_ref()
            .map(FilterProcessor::validator);

        if self.randomness_extent == 0
            || [Motion::LeftToRight, Motion::RightToLeft].contains(&direction)
        {
            for y in area.y..area.bottom() {
                let row_variance = axis_jitter.next();
                for x in area.x..area.right() {
                    let pos = Position { x, y };
                    let cell = &mut buf[pos];

                    if cell_filter
                        .as_ref()
                        .is_some_and(|c| !c.is_valid(pos, cell))
                    {
                        continue;
                    }
                    apply_alpha(cell, offset(pos, row_variance));
                }
            }
        } else {
            let col_variances = (area.x..area.x + area.width)
                .map(|_| axis_jitter.next().1)
                .collect::<Vec<i16>>();

            for y in area.y..area.bottom() {
                for x in area.x..area.right() {
                    let pos = Position { x, y };
                    let cell = buf.cell_mut(pos).unwrap();

                    if cell_filter
                        .as_ref()
                        .is_some_and(|c| !c.is_valid(pos, cell))
                    {
                        continue;
                    }
                    let col_variance = (0, col_variances[(x - area.x) as usize]);
                    apply_alpha(cell, offset(pos, col_variance));
                }
            }
        }
    }

    fn set_rng(&mut self, rng: crate::SimpleRng) {
        self.rng = rng;
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::{DslFormat, EffectExpression};

        let direction = if self.timer.is_reversed() ^ self.direction.flips_timer() {
            self.direction.flipped()
        } else {
            self.direction
        };

        EffectExpression::parse(&format!(
            "fx::{}({}, {}, {}, {}, {})",
            self.name(),
            direction.dsl_format(),
            self.gradient_length,
            self.randomness_extent,
            self.faded_color.dsl_format(),
            self.timer.dsl_format()
        ))
    }
}

fn offset(p: Position, translate: (i16, i16)) -> Position {
    Position {
        x: (p.x as i16 + translate.0).max(0) as _,
        y: (p.y as i16 + translate.1).max(0) as _,
    }
}

#[cfg(test)]
#[cfg(feature = "dsl")]
mod tests {
    use indoc::indoc;
    use ratatui_core::style::Color;

    use crate::{fx, Motion};

    #[test]
    fn to_dsl_slide_in() {
        let dsl = fx::sweep_in(Motion::LeftToRight, 10, 5, Color::from_u32(0), 1000)
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, indoc! {
            "fx::sweep_in(
                 Motion::LeftToRight,
                 10,
                 5,
                 Color::from_u32(0),
                 EffectTimer::from_ms(1000, Interpolation::Linear)
             )"
        });
    }

    #[test]
    fn to_dsl_slide_out() {
        let dsl = fx::sweep_out(Motion::UpToDown, 10, 5, Color::from_u32(0), 1000)
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, indoc! {
            "fx::sweep_out(
                 Motion::UpToDown,
                 10,
                 5,
                 Color::from_u32(0),
                 EffectTimer::from_ms(1000, Interpolation::Linear)
             )"
        });
    }
}
