use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::prelude::Rect;
use crate::CellIterator;

use crate::effect::{Effect, CellFilter};
use crate::effect_timer::EffectTimer;
use crate::interpolation::Interpolatable;
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

        let mut lerped_area = self.original.unwrap();
        let x = lerped_area.x as i16 + (0i16.lerp(&self.translate_by.0, alpha));
        let y = lerped_area.y as i16 + (0i16.lerp(&self.translate_by.1, alpha));
        lerped_area.x = x.max(0) as u16;
        lerped_area.y = y.max(0) as u16;

        self.set_area(lerped_area);
        if let Some(fx) = self.fx.as_mut() {
            fx.process(duration, buf, lerped_area);
        }

        overflow
    }

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {
        // nothing to do
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

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        if let Some(fx) = self.fx.as_mut() {
            fx.set_cell_selection(strategy)
        }
    }

    fn reverse(&mut self) {
        self.lifetime = self.lifetime.reversed()
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        todo!()
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        todo!()
    }
}