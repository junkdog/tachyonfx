use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use std::time::Duration;

use crate::{CellFilter, CellIterator, Effect, EffectTimer, Shader};

#[derive(Clone)]
pub struct PingPong {
    fx: Effect,
    fx_original: Effect,
    fx_area_old: Option<Rect>,
    is_reversing: bool,
    strategy: CellFilter,
}

impl PingPong {
    pub fn new(fx: Effect) -> Self {
        Self {
            fx_original: fx.clone(),
            fx,
            fx_area_old: None,
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
        // if let Some(fx) = self.fx_area_old {
        //     self.fx.set_area(fx);
        // }
        // self.fx_area_old = None; // old fx area is only valid for one frame

        let overflow = self.fx.process(duration, buf, area);

        if overflow.is_some() && !self.is_reversing {
            self.is_reversing = true;
            // self.fx_area_old = self.fx.area();
            self.fx = self.fx_original.clone();
            self.fx.reverse();
            None // consumes any overflow when reversing, to reset the area
        } else {
            overflow
        }
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

        // let area = self.fx.area().or_else(|| self.fx_area_old);
        // if area.is_some() {
        //     area
        // } else {
        //     None
        // }
    }

    fn set_area(&mut self, area: Rect) {
        self.fx.set_area(area);
    }

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        self.strategy = strategy;
    }

    fn reverse(&mut self) {
        self.fx.reverse();
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        Some(self.strategy.clone())
    }
}