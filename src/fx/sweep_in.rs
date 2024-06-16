use std::ops::Range;

use ratatui::layout::{Position, Rect};
use ratatui::prelude::Color;

use Interpolation::CircOut;

use crate::{CellIterator, ColorMapper};
use crate::effect::CellFilter;
use crate::effect_timer::EffectTimer;
use crate::interpolation::{Interpolatable, Interpolation};
use crate::shader::Shader;

#[derive(Clone)]
pub struct SweepIn {
    gradient_length: u16,
    faded_color: Color,
    lifetime: EffectTimer,
    area: Option<Rect>,
    cell_filter: CellFilter,
    direction: Direction,
}

#[derive(Clone, Copy)]
pub enum Direction {
    LeftToRight,
    RightToLeft,
    UpToDown,
    DownToUp,
}

impl SweepIn {
    pub fn new(
        direction: Direction,
        gradient_length: u16,
        faded_color: Color,
        lifetime: EffectTimer,
    ) -> Self {
        let timer = match direction {
            Direction::RightToLeft => lifetime.reversed(),
            Direction::LeftToRight => lifetime,
            Direction::UpToDown => lifetime,
            Direction::DownToUp => lifetime.reversed(),
        };

        Self {
            direction,
            gradient_length,
            faded_color,
            lifetime: timer,
            area: None,
            cell_filter: CellFilter::All,
        }
    }

    fn horizontal_gradient(&self, area: Rect, alpha: f32) -> Range<f32> {
        let gradient_len = self.gradient_length as f32;
        let x_start = (area.x as f32 - gradient_len) + ((area.width as f32 + gradient_len) * alpha);
        let x_end = x_start + gradient_len;

        x_start..x_end
    }

    fn vertical_gradient(&self, area: Rect, progress: f32) -> Range<f32> {
        let gradient_len = self.gradient_length as f32;
        let y_start = (area.y as f32 - gradient_len) + ((area.height as f32 + gradient_len) * progress);
        let y_end = y_start + gradient_len;

        y_start..y_end
    }

}

impl Shader for SweepIn {
    fn execute(&mut self, alpha: f32, area: Rect, cell_iter: CellIterator) {
        let direction = self.direction;
        let gradient = match direction {
            Direction::LeftToRight | Direction::RightToLeft =>
                self.horizontal_gradient(area, alpha),
            Direction::UpToDown | Direction::DownToUp =>
                self.vertical_gradient(area, alpha),
        };

        let window_alpha = window_alpha_fn(direction, gradient);

        let mut fg_mapper = ColorMapper::default();
        let mut bg_mapper = ColorMapper::default();

        cell_iter.for_each(|(pos, cell)| {
            let a = window_alpha(pos);

            match a {
                0.0 => {
                    cell.set_fg(self.faded_color);
                    cell.set_bg(self.faded_color);
                },
                1.0 => {} // nothing to do
                _ => {
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
        self.lifetime.done()
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

    fn cell_selection(&mut self, strategy: CellFilter) {
        self.cell_filter = strategy;
    }

    fn reverse(&mut self) {
        self.lifetime = self.lifetime.reversed();
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.lifetime)
    }

    fn cell_filter(&self) -> Option<CellFilter> {
        Some(self.cell_filter.clone())
    }
}

fn window_alpha_fn(
    direction: Direction,
    gradient: Range<f32>,
) -> Box::<dyn Fn(Position) -> f32> {
    let gradient_len = gradient.end - gradient.start;
    match direction {
        Direction::LeftToRight => Box::new(move |p: Position| -> f32 {
            match p.x as f32 {
                x if gradient.contains(&x) => 1.0 - (x - gradient.start) / gradient_len,
                x if x < gradient.start    => 1.0,
                _                          => 0.0,
            }
        }),
        Direction::RightToLeft => Box::new(move |p: Position| -> f32 {
            match p.x as f32 {
                x if gradient.contains(&x) => (x - gradient.start) / gradient_len,
                x if x >= gradient.end     => 1.0,
                _                          => 0.0,
            }
        }),
        Direction::UpToDown => Box::new(move |p: Position| -> f32 {
            match p.y as f32 {
                y if gradient.contains(&y) => 1.0 - (y - gradient.start) / gradient_len,
                y if y < gradient.start    => 1.0,
                _                          => 0.0,
            }
        }),
        Direction::DownToUp => Box::new(move |p: Position| -> f32 {
            match p.y as f32 {
                y if gradient.contains(&y) => (y - gradient.start) / gradient_len,
                y if y >= gradient.start   => 1.0,
                _                          => 0.0,
            }
        }),
    }
}