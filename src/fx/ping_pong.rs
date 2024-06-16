use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use std::time::Duration;

use crate::{CellFilter, CellIterator, Effect, EffectTimer, Shader};

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

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {
        // nothing to do
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

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        self.fx.timer_mut()
    }

    fn cell_filter(&self) -> Option<CellFilter> {
        Some(self.strategy.clone())
    }
}