use std::time::Duration;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use crate::{CellFilter, Effect, EffectTimer, Interpolation, Shader};

#[derive(Clone)]
pub struct PingPong {
    fx: Effect,
    fx_original: Effect,
    is_reversing: bool,
    strategy: CellFilter,
}

impl PingPong {
    pub fn new(fx: Effect) -> Self {
        Self {
            fx_original: fx.clone(),
            fx,
            is_reversing: false,
            strategy: CellFilter::default(),
        }
    }
}

impl Shader for PingPong {

    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) -> Option<Duration> {
        let overflow = self.fx.process(duration, buf, area);
        if let Some(overflow) = overflow {
            if !self.is_reversing {
                self.is_reversing = true;
                self.fx = self.fx_original.clone();
                self.fx.reverse();
                return self.fx.process(overflow, buf, area)
            }
        }

        overflow
    }

    fn done(&self) -> bool {
        self.is_reversing && self.fx.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.fx.area()
    }

    fn set_area(&mut self, area: Rect) {
        self.fx.set_area(area);
    }

    fn cell_selection(&mut self, strategy: CellFilter) {
        self.strategy = strategy;
    }

    fn reverse(&mut self) {
        self.fx.reverse();
    }
}