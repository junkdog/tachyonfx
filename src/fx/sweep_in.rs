use ratatui::layout::Rect;
use ratatui::prelude::Color;

use Interpolation::CircOut;

use crate::{CellIterator, ColorMapper};
use crate::CellFilter;
use crate::effect_timer::EffectTimer;
use crate::fx::sliding_window_alpha::SlidingWindowAlpha;
use crate::interpolation::{Interpolatable, Interpolation};
use crate::shader::Shader;

#[derive(Clone)]
pub struct SweepIn {
    gradient_length: u16,
    faded_color: Color,
    timer: EffectTimer,
    direction: Direction,
    area: Option<Rect>,
    cell_filter: CellFilter,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Direction {
    LeftToRight,
    RightToLeft,
    UpToDown,
    DownToUp,
}

impl Direction {
    pub(crate) fn flipped(&self) -> Self {
        match self {
            Self::LeftToRight => Self::RightToLeft,
            Self::RightToLeft => Self::LeftToRight,
            Self::UpToDown    => Self::DownToUp,
            Self::DownToUp    => Self::UpToDown,
        }
    }

    pub(crate) fn flips_timer(&self) -> bool {
        self == &Direction::RightToLeft || self == &Direction::DownToUp
    }
}

impl SweepIn {
    pub fn new(
        direction: Direction,
        gradient_length: u16,
        faded_color: Color,
        lifetime: EffectTimer,
    ) -> Self {
        Self {
            direction,
            gradient_length,
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

    fn execute(&mut self, alpha: f32, area: Rect, cell_iter: CellIterator) {
        let direction = self.direction;

        let window_alpha = SlidingWindowAlpha::builder()
            .direction(direction)
            .progress(alpha)
            .area(area)
            .gradient_len(self.gradient_length)
            .build();

        let mut fg_mapper = ColorMapper::default();
        let mut bg_mapper = ColorMapper::default();

        cell_iter.for_each(|(pos, cell)| {
            match window_alpha.alpha(pos) {
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
        });
    }

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
        Some(self.timer.clone())
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        Some(self.cell_filter.clone())
    }
}