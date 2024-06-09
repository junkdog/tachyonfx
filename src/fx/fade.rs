use std::time::Duration;
use derive_builder::Builder;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Color;
use crate::color_mapper::ColorMapper;
use crate::effect::{Effect, CellFilter, IntoEffect};
use crate::effect_timer::EffectTimer;
use crate::shader::Shader;


#[derive(Builder, Clone)]
#[builder(pattern = "owned")]
pub struct FadeColors {
    fg: Option<Color>,
    bg: Option<Color>,
    lifetime: EffectTimer,
    #[builder(default)]
    area: Option<Rect>,
    #[builder(default)]
    cell_filter: CellFilter,
}

impl FadeColors {
    pub fn builder() -> FadeColorsBuilder {
        FadeColorsBuilder::default()
    }
}

impl From<FadeColorsBuilder> for Effect {
    fn from(value: FadeColorsBuilder) -> Self {
        value.build().unwrap().into_effect()
    }
}

impl Shader for FadeColors {
    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect,
    ) -> Option<Duration> {
        let alpha = self.lifetime.alpha();
        let remainder = self.lifetime.process(duration);

        let cell_filter = self.cell_filter.selector(area);
        let mut fg_mapper = ColorMapper::default();
        let mut bg_mapper = ColorMapper::default();
        
        self.cell_iter(buf, area)
            .filter(|(pos, cell)| cell_filter.is_valid(*pos, cell))
            .for_each(|(_, cell)| {
                if let Some(fg) = self.fg.as_ref() {
                    let color = fg_mapper.mapping(cell.fg, fg, alpha);
                    cell.set_fg(color);
                }

                if let Some(bg) = self.bg.as_ref() {
                    let color = bg_mapper.mapping(cell.bg, bg, alpha);
                    cell.set_bg(color);
                }
            });

        remainder
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
        self.area = Some(area);
    }

    fn cell_selection(&mut self, strategy: CellFilter) {
        self.cell_filter = strategy;
    }

    fn reverse(&mut self) {
        self.lifetime = self.lifetime.reversed();
    }
}
