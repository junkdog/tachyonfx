use std::time::Duration;
use ratatui::buffer::Buffer;
use ratatui::prelude::Rect;
use ratatui::widgets::Clear;
use ratatui::widgets::Widget;
use crate::effect::{Effect, FilterMode};
use crate::effect_timer::EffectTimer;
use crate::interpolation::Interpolatable;
use crate::rect_ext::CenteredShrink;
use crate::render_effect::EffectRenderer;
use crate::shader::Shader;

#[derive(Clone)]
pub struct ResizeArea {
    fx: Option<Effect>,
    target_area: Rect,
    initial_w: u16,
    initial_h: u16,
    lifetime: EffectTimer,
}

impl ResizeArea {
    pub fn new(
        fx: Option<Effect>,
        initial_w: u16,
        initial_h: u16,
        lifetime: EffectTimer
    ) -> Self {
        Self { fx, initial_w, initial_h, lifetime, target_area: Rect::default() }
    }
}

impl Shader for ResizeArea {
    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) -> Option<Duration> {
        
        let a = self.lifetime.alpha();
        let remaining = self.lifetime.process(duration);
        
        let w = self.initial_w.lerp(&area.width, a);
        let h = self.initial_h.lerp(&area.height, a);
        
        let resized_area = area.inner_centered(w, h);
        Clear.render(resized_area, buf);
        self.set_area(resized_area);
        
        if let Some(fx) = self.fx.as_mut() {
            buf.render_effect(fx, resized_area, duration);
        }
        
        remaining
    }

    fn done(&self) -> bool {
        self.lifetime.done()
            && (self.fx.as_ref().is_some_and(|fx| fx.done()) || self.fx.is_none())
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        Some(self.target_area)
    }

    fn set_area(&mut self, area: Rect) {
        if let Some(fx) = self.fx.as_mut() {
            fx.set_area(area);
        }
        
        self.target_area = area;
    }

    fn cell_selection(&mut self, strategy: FilterMode) {
        if let Some(fx) = self.fx.as_mut() {
            fx.cell_selection(strategy);
        }
    }

    fn reverse(&mut self) {
        self.lifetime = self.lifetime.clone().reversed();
    }
}