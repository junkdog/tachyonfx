use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Color;

use Interpolation::CircOut;

use crate::ColorMapper;
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
}

impl SweepIn {
    pub fn new(
        gradient_length: u16,
        faded_color: Color,
        lifetime: EffectTimer,
    ) -> Self {
        Self {
            gradient_length,
            faded_color,
            lifetime,
            area: None,
            cell_filter: CellFilter::All,
        }
    }
}

impl Shader for SweepIn {
    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect,
    ) -> Option<Duration> {
        let alpha = self.lifetime.alpha();
        let overflow = self.lifetime.process(duration);

        let gradient_len = self.gradient_length as f32;
        let x_start = (area.x as f32 - gradient_len) + ((area.width as f32 + gradient_len) * alpha);
        let x_end = x_start + gradient_len;

        let gradient_range = x_start..x_end;
        let window_alpha = |x: u16| {
            // fade in, left to right using a linear gradient
            match x as f32 {
                x if gradient_range.contains(&x) => 1.0 - (x - x_start) / gradient_len,
                x if x < gradient_range.start    => 1.0,
                _                                => 0.0,
            }
        };
        
        let cell_filter = self.cell_filter.selector(area);
        
        let mut fg_mapper = ColorMapper::default();
        let mut bg_mapper = ColorMapper::default();
        
        self.cell_iter(buf, area)
            .filter(|(pos, cell)| cell_filter.is_valid(*pos, cell))
            .for_each(|(pos, cell)| {
                let a = window_alpha(pos.x);

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

        overflow
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
}