use ratatui::buffer::Buffer;
use ratatui::layout::Size;
use ratatui::prelude::Rect;
use ratatui::widgets::Clear;
use ratatui::widgets::Widget;
use crate::{CellFilter, CellIterator, Duration};
use crate::effect::Effect;
use crate::effect_timer::EffectTimer;
use crate::widget::EffectSpan;
use crate::interpolation::Interpolatable;
use crate::rect_ext::CenteredShrink;
use crate::shader::Shader;

#[derive(Clone)]
pub struct ResizeArea {
    fx: Option<Effect>,
    area: Option<Rect>,
    original_area: Option<Rect>,
    initial_size: Size,
    timer: EffectTimer,
}

impl ResizeArea {
    pub fn new(
        fx: Option<Effect>,
        initial_size: Size,
        timer: EffectTimer
    ) -> Self {
        Self { fx, initial_size, timer, original_area: None, area: None }
    }
}

impl Shader for ResizeArea {
    fn name(&self) -> &'static str {
        "resize_area"
    }

    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) -> Option<Duration> {
        if self.original_area.is_none() {
            self.original_area = Some(area);
        }

        let target_area = self.original_area.unwrap();
        
        let a = self.timer.alpha();
        let overflow = self.timer.process(duration);

        let w = self.initial_size.width.lerp(&target_area.width, a);
        let h = self.initial_size.height.lerp(&target_area.height, a);
        
        let resized_area = target_area.inner_centered(w, h);
        Clear.render(resized_area, buf);
        self.set_area(resized_area);
        
        if let Some(fx) = &mut self.fx {
            fx.set_area(resized_area);
            let hosted_overflow = fx.process(duration, buf, resized_area);
            // only return the overflow if the fx is done and this translate is done
            match (overflow, hosted_overflow) {
                (Some(a), Some(b)) => Some(a.min(b)),
                _ => None
            }
        } else {
            overflow
        }
    }

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {
        // nothing to do
    }

    fn done(&self) -> bool {
        self.timer.done()
            && (self.fx.as_ref().is_some_and(|fx| fx.done()) || self.fx.is_none())
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.area.clone()
    }

    fn set_area(&mut self, area: Rect) {
        self.area = Some(area);
        if let Some(fx) = self.fx.as_mut() {
            fx.set_area(area);
        }
    }

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        if let Some(fx) = self.fx.as_mut() {
            fx.set_cell_selection(strategy);
        }
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn timer(&self) -> Option<EffectTimer> {
        Some(self.timer.clone())
    }

    fn as_effect_span(&self, offset: Duration) -> EffectSpan {
        match &self.fx {
            Some(fx) => EffectSpan::new(self, offset, vec![fx.as_effect_span(offset)]),
            None     => EffectSpan::new(self, offset, Vec::default())
        }
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        self.fx.as_ref().and_then(Effect::cell_selection)
    }

    fn reset(&mut self) {
        self.timer.reset();
        if let Some(fx) = self.fx.as_mut() {
            fx.reset();
        }
    }
}