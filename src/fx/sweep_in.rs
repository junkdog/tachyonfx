use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};
use ratatui::prelude::Color;

use Interpolation::CircOut;

use crate::effect_timer::EffectTimer;
use crate::fx::sliding_window_alpha::SlidingWindowAlpha;
use crate::interpolation::Interpolation;
use crate::shader::Shader;
use crate::{CellFilter, Duration, DirectionalVariance, LruCache, Motion, default_shader_impl, ColorSpace};

#[derive(Clone, Debug)]
pub struct SweepIn {
    gradient_length: u16,
    randomness_extent: u16,
    faded_color: Color,
    timer: EffectTimer,
    direction: Motion,
    area: Option<Rect>,
    cell_filter: Option<CellFilter>,
    color_space: ColorSpace,
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
            timer: if direction.flips_timer() { lifetime.reversed() } else { lifetime },
            area: None,
            cell_filter: None,
            color_space: ColorSpace::default(),
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

        let mut axis_jitter = DirectionalVariance::from(area, direction, self.randomness_extent);

        let mut fg_cache: LruCache<(Color, f32), Color, 4> = LruCache::default();
        let mut bg_cache: LruCache<(Color, f32), Color, 4> = LruCache::default();

        let mut apply_alpha = |cell: &mut Cell, pos: Position| {
            match window_alpha.alpha(pos) {
                0.0 => {
                    cell.set_fg(self.faded_color);
                    cell.set_bg(self.faded_color);
                },
                1.0 => {} // nothing to do
                a => {
                    let faded = self.faded_color;
                    let mod_a = CircOut.alpha(a);
                    let fg = fg_cache
                        .memoize(&(cell.fg, a), |(c, _)| self.color_space.lerp(&faded, c, mod_a));
                    let bg = bg_cache
                        .memoize(&(cell.bg, a), |(c, _)| self.color_space.lerp(&faded, c, mod_a));

                    cell.set_fg(fg);
                    cell.set_bg(bg);
                }
            }
        };


        let area = area.intersection(buf.area); // safe area
        let cell_filter = self.cell_filter.as_ref().unwrap_or(&CellFilter::All).selector(area);

        if self.randomness_extent == 0 || [Motion::LeftToRight, Motion::RightToLeft].contains(&direction) {
            for y in area.y..area.bottom() {
                let row_variance = axis_jitter.next();
                for x in area.x..area.right() {
                    let pos = Position { x, y };
                    let cell = &mut buf[pos];

                    if cell_filter.is_valid(pos, cell) {
                        apply_alpha(cell, offset(pos, row_variance));
                    }
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

                    if cell_filter.is_valid(pos, cell) {
                        let col_variance = (0, col_variances[(x - area.x) as usize]);
                        apply_alpha(cell, offset(pos, col_variance));
                    }
                }
            }
        }
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::{DslFormat, EffectExpression};

        let direction = if self.timer.is_reversed() ^ self.direction.flips_timer()  {
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
    use crate::{fx, Motion, Shader};
    use indoc::indoc;
    use ratatui::prelude::Color;

    #[test]
    fn to_dsl_slide_in() {
        let dsl = fx::sweep_in(
            Motion::LeftToRight,
            10,
            5,
            Color::from_u32(0),
            1000,
        ).to_dsl().unwrap().to_string();


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
        let dsl = fx::sweep_out(
            Motion::UpToDown,
            10,
            5,
            Color::from_u32(0),
            1000,
        ).to_dsl().unwrap().to_string();


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