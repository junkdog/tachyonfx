use bon::{builder, Builder};
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;

use crate::fx::sliding_window_alpha::SlidingWindowAlpha;
use crate::{Motion, DirectionalVariance};
use crate::{CellFilter, Duration, EffectTimer, Shader};
use crate::dsl::{DslError, DslFormat, EffectExpression};

/// A shader that applies a directional sliding effect to terminal cells.
#[derive(Builder, Clone, Debug)]
pub struct SlideCell {
    /// The color behind the sliding cell.
    color_behind_cell: Color,
    /// The direction of the sliding effect.
    direction: Motion,
    /// The length of the gradient used for the sliding effect.
    gradient_length: u16,
    /// The extent of randomness applied to the sliding effect.
    #[builder(default)]
    randomness_extent: u16,
    /// The timer controlling the duration and progress of the effect.
    #[builder(into)]
    timer: EffectTimer,
    /// The area within which the effect is applied.
    area: Option<Rect>,
    /// The cell selection strategy used to filter cells.
    #[builder(default)]
    cell_filter: CellFilter,
}

impl SlideCell {
    fn slided_cell(&self, alpha: f32) -> char {
        let alpha = alpha.clamp(0.0, 1.0);
        let char_idx = (LAST_IDX as f32 * alpha).round() as usize;

        match self.direction {
            Motion::LeftToRight | Motion::RightToLeft => SHRINK_H[char_idx],
            Motion::UpToDown    | Motion::DownToUp    => SHRINK_V[char_idx],
        }
    }
}

impl Shader for SlideCell {
    fn name(&self) -> &'static str {
        if self.timer.is_reversed() ^ self.direction.flips_timer() {
            "slide_in"
        } else {
            "slide_out"
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

        let update_cell = |cell: &mut Cell, pos: Position| {
            match window_alpha.alpha(pos) {
                0.0 => {},
                1.0 => {
                    cell.set_char(' ');
                    cell.fg = cell.bg;
                    cell.bg = self.color_behind_cell;
                }
                a => {
                    cell.set_char(self.slided_cell(a));
                    cell.fg = cell.bg;
                    cell.bg = self.color_behind_cell;
                }
            }
        };

        let area = area.intersection(buf.area);
        let cell_filter = self.cell_filter.selector(area);

        if self.randomness_extent == 0 || [Motion::LeftToRight, Motion::RightToLeft].contains(&direction) {
            for y in area.y..area.bottom() {
                let row_variance = axis_jitter.next();
                for x in area.x..area.right() {
                    let pos = Position { x, y };
                    let cell = buf.cell_mut(pos).unwrap();

                    if cell_filter.is_valid(pos, cell) {
                        update_cell(cell, offset(pos, row_variance));
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
                        update_cell(cell, offset(pos, col_variance));
                    }
                }
            }
        }
    }

    fn done(&self) -> bool { self.timer.done() }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.area
    }

    fn set_area(&mut self, area: Rect) {
        self.area = Some(area);
    }

    fn filter(&mut self, strategy: CellFilter) {
        self.cell_filter = strategy;
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn timer(&self) -> Option<EffectTimer> {
        Some(self.timer)
    }

    fn cell_filter(&self) -> Option<CellFilter> {
        Some(self.cell_filter.clone())
    }

    fn to_dsl(&self) -> Result<EffectExpression, DslError> {
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
            self.color_behind_cell.dsl_format(),
            self.timer.dsl_format()
        ))
    }
}

const SHRINK_V: &[char; 9] = &['█', '▇', '▆', '▅', '▄', '▃', '▂', '▁', ' '];
const SHRINK_H: &[char; 9] = &['█', '▉', '▊', '▋', '▌', '▍', '▎', '▏', ' '];
const LAST_IDX: usize = SHRINK_H.len() - 1;

fn offset(p: Position, translate: (i16, i16)) -> Position {
    Position {
        x: (p.x as i16 + translate.0).max(0) as _,
        y: (p.y as i16 + translate.1).max(0) as _,
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use ratatui::prelude::Color;
    use crate::{fx, Motion, Shader};

    #[test]
    fn to_dsl_slide_in() {
        let dsl = fx::slide_in(
            Motion::LeftToRight,
            10,
            5,
            Color::from_u32(0),
            1000,
        ).to_dsl().unwrap().to_string();


        assert_eq!(dsl.to_string(), indoc! {
            "fx::slide_in(
                 LeftToRight,
                 10,
                 5,
                 Color::from_u32(0),
                 EffectTimer::from_ms(
                     1000,
                     Linear
                 )
             )"
        });
    }

    #[test]
    fn to_dsl_slide_out() {
        let dsl = fx::slide_out(
            Motion::UpToDown,
            10,
            5,
            Color::from_u32(0),
            1000,
        ).to_dsl().unwrap().to_string();


        assert_eq!(dsl.to_string(), indoc! {
            "fx::slide_out(
                 UpToDown,
                 10,
                 5,
                 Color::from_u32(0),
                 EffectTimer::from_ms(
                     1000,
                     Linear
                 )
             )"
        });
    }
}