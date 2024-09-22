use bon::{builder, Builder};
use ratatui::layout::Rect;
use ratatui::prelude::Color;

use crate::color_mapper::ColorMapper;
use crate::effect_timer::EffectTimer;
use crate::shader::Shader;
use crate::{CellFilter, CellIterator, Interpolatable};

#[derive(Builder, Clone)]
pub struct FadeColors {
    fg: Option<Color>,
    bg: Option<Color>,
    #[builder(into)]
    timer: EffectTimer,
    area: Option<Rect>,
    #[builder(default)]
    cell_filter: CellFilter,
}

impl Shader for FadeColors {
    fn name(&self) -> &'static str {
        if self.timer.is_reversed() { "fade_from" } else { "fade_to" }
    }

    fn execute(&mut self, alpha: f32, _area: Rect, cell_iter: CellIterator) {
        let mut fg_mapper = ColorMapper::default();
        let mut bg_mapper = ColorMapper::default();

        cell_iter.for_each(|(_, cell)| {
            if let Some(fg) = self.fg.as_ref() {
                let color = fg_mapper.map(cell.fg, alpha, |c| c.lerp(fg, alpha));
                cell.set_fg(color);
            }

            if let Some(bg) = self.bg.as_ref() {
                let color = bg_mapper.map(cell.bg, alpha, |c| c.lerp(bg, alpha));
                cell.set_bg(color);
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
        self.area = Some(area);
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
