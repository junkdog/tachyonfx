use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};
use ratatui::prelude::Color;

use Interpolation::CircOut;

use crate::effect_timer::EffectTimer;
use crate::fx::sliding_window_alpha::SlidingWindowAlpha;
use crate::fx::{Direction, DirectionalVariance};
use crate::interpolation::{Interpolatable, Interpolation};
use crate::shader::Shader;
use crate::CellFilter;
use crate::{CellIterator, ColorMapper, Duration};

#[derive(Clone)]
pub struct SweepIn {
    gradient_length: u16,
    randomness_extent: u16,
    faded_color: Color,
    timer: EffectTimer,
    direction: Direction,
    area: Option<Rect>,
    cell_filter: CellFilter,
}


impl SweepIn {
    pub fn new(
        direction: Direction,
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
            cell_filter: CellFilter::All,
        }
    }
}

impl Shader for SweepIn {
    fn name(&self) -> &'static str {
        if self.timer.is_reversed() ^ self.direction.flips_timer() {
            "sweep_out"
        } else {
            "sweep_in"
        }
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        let (overflow, alpha) = self.timer_mut()
            .map(|t| (t.process(duration), t.alpha()))
            .unwrap_or((None, 1.0));

        let direction = self.direction;

        let window_alpha = SlidingWindowAlpha::builder()
            .direction(direction)
            .progress(alpha)
            .area(area)
            .gradient_len(self.gradient_length + self.randomness_extent)
            .build();

        let mut axis_jitter = DirectionalVariance::from(area, direction, self.randomness_extent);

        let mut fg_mapper = ColorMapper::default();
        let mut bg_mapper = ColorMapper::default();

        let mut apply_alpha = |cell: &mut Cell, alpha: f32| {
            match alpha {
                0.0 => {
                    cell.set_fg(self.faded_color);
                    cell.set_bg(self.faded_color);
                },
                1.0 => {} // nothing to do
                a => {
                    let fg = fg_mapper
                        .map(cell.fg, a, |c| self.faded_color.tween(&c, a, CircOut));
                    let bg = bg_mapper
                        .map(cell.bg, a, |c| self.faded_color.tween(&c, a, CircOut));

                    cell.set_fg(fg);
                    cell.set_bg(bg);
                }
            }
        };

        if self.randomness_extent == 0 || [Direction::LeftToRight, Direction::RightToLeft].contains(&direction) {
            for y in area.y..area.y + area.height {
                let row_variance = axis_jitter.next();
                for x in area.x..area.x + area.width {
                    let pos = Position { x, y };
                    let cell = buf.cell_mut(pos).unwrap();

                    apply_alpha(cell, window_alpha.alpha(offset(pos, row_variance)));
                }
            }
        } else {
            let col_variances = (area.x..area.x + area.width)
                .map(|_| axis_jitter.next().1)
                .collect::<Vec<i16>>();

            for y in area.y..area.y + area.height {
                for x in area.x..area.x + area.width {
                    let pos = Position { x, y };
                    let cell = buf.cell_mut(pos).unwrap();
                    let col_variance = (0, col_variances[(x - area.x) as usize]);

                    apply_alpha(cell, window_alpha.alpha(offset(pos, col_variance)));
                }
            }
        }

        overflow
    }
    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {}

    fn done(&self) -> bool {
        self.timer.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.area
    }

    fn set_area(&mut self, area: Rect) {
        self.area = Some(area)
    }

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        self.cell_filter = strategy;
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn timer(&self) -> Option<EffectTimer> {
        Some(self.timer)
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        Some(self.cell_filter.clone())
    }
}

fn offset(p: Position, translate: (i16, i16)) -> Position {
    Position {
        x: (p.x as i16 + translate.0).max(0) as _,
        y: (p.y as i16 + translate.1).max(0) as _,
    }
}