use bon::builder;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;

use crate::fx::sliding_window_alpha::SlidingWindowAlpha;
use crate::fx::{Direction, DirectionalVariance};
use crate::{CellFilter, CellIterator, Duration, EffectTimer, Shader};

/// A shader that applies a directional sliding effect to terminal cells.
#[derive(Clone)]
#[builder]
pub struct SlideCell {
    /// The color behind the sliding cell.
    color_behind_cell: Color,
    /// The direction of the sliding effect.
    direction: Direction,
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
            Direction::LeftToRight | Direction::RightToLeft => SHRINK_H[char_idx],
            Direction::UpToDown    | Direction::DownToUp    => SHRINK_V[char_idx],
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

        if self.randomness_extent == 0 || [Direction::LeftToRight, Direction::RightToLeft].contains(&direction) {
            for y in area.y..area.y + area.height {
                let row_variance = axis_jitter.next();
                for x in area.x..area.x + area.width {
                    let pos = Position { x, y };
                    let cell = buf.cell_mut(pos).unwrap();
                    match window_alpha.alpha(offset(pos, row_variance)) {
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
                }
            }
        } else {
            let col_variances = (area.x..area.x + area.width).into_iter()
                .map(|_| axis_jitter.next().1)
                .collect::<Vec<i16>>();

            for y in area.y..area.y + area.height {
                for x in area.x..area.x + area.width {
                    let pos = Position { x, y };
                    let cell = buf.cell_mut(pos).unwrap();
                    let col_variance = (0, col_variances[(x - area.x) as usize]);

                    match window_alpha.alpha(offset(pos, col_variance)) {
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
                }
            }
        }

        overflow
    }

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {}

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

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        self.cell_filter = strategy;
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn timer(&self) -> Option<EffectTimer> {
        Some(self.timer.clone())
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        Some(self.cell_filter.clone())
    }
}

const SHRINK_V: &'static [char; 9] = &['█', '▇', '▆', '▅', '▄', '▃', '▂', '▁', ' '];
const SHRINK_H: &'static [char; 9] = &['█', '▉', '▊', '▋', '▌', '▍', '▎', '▏', ' '];
const LAST_IDX: usize = SHRINK_H.len() - 1;

fn offset(p: Position, translate: (i16, i16)) -> Position {
    Position {
        x: (p.x as i16 + translate.0).max(0) as _,
        y: (p.y as i16 + translate.1).max(0) as _,
    }
}