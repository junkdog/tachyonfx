use std::time::Duration;
use derive_builder::Builder;
use ratatui::buffer::Buffer;
use ratatui::layout::Position;
use ratatui::prelude::Rect;
use ratatui::widgets::Clear;
use ratatui::widgets::Widget;
use crate::effect::{Effect, FilterMode};
use crate::effect_timer::EffectTimer;
use crate::interpolation::Interpolatable;
use crate::rect_ext::CenteredShrink;
use crate::render_effect::EffectRenderer;
use crate::shader::Shader;

#[derive(Clone, Default)]
pub struct Translate {
    fx: Option<Effect>,
    area: Option<Rect>,
    original: Option<Rect>,
    translate_by: (i16, i16),
    lifetime: EffectTimer,
}

impl Translate {
    pub fn new(
        fx: Option<Effect>,
        translate_by: (i16, i16),
        lifetime: EffectTimer
    ) -> Self {
        Self { fx, translate_by, lifetime, ..Self::default() }
    }
}

impl Shader for Translate {
    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) -> Option<Duration> {
        if area.width == 0 { return None; }

        if self.original.is_none() {
            self.original = Some(area);
        }

        let overflow = self.lifetime.process(duration);
        let alpha = self.lifetime.alpha();

        let mut lerped_area = self.original.clone().unwrap();
        let x = lerped_area.x as i16 + (0i16.lerp(&self.translate_by.0, alpha));
        let y = lerped_area.y as i16 + (0i16.lerp(&self.translate_by.1, alpha));
        lerped_area.x = x.max(0) as u16;
        lerped_area.y = y.max(0) as u16;

        // lerped_area = self.original.unwrap();
        self.set_area(lerped_area);
        if let Some(fx) = self.fx.as_mut() {
            fx.process(duration, buf, lerped_area);
        }

        overflow
    }

    fn done(&self) -> bool {
        self.lifetime.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        if let Some(fx) = self.fx.as_ref() {
            if fx.area().is_some() {
                return fx.area();
            }
        }
        self.area
    }

    fn set_area(&mut self, area: Rect) {
        self.area = Some(area);
        if let Some(fx) = self.fx.as_mut() {
            fx.set_area(area)
        }
    }

    fn cell_selection(&mut self, strategy: FilterMode) {
        if let Some(fx) = self.fx.as_mut() {
            fx.cell_selection(strategy)
        }
    }

    fn reverse(&mut self) {
        self.lifetime = self.lifetime.clone().reversed()
    }
}