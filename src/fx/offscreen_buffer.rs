use std::cell::RefCell;
use std::rc::Rc;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use crate::{CellFilter, CellIterator, Duration, Effect, Shader};
use crate::widget::EffectSpan;

#[derive(Clone)]
pub struct OffscreenBuffer {
    fx: Effect,
    render_target: Rc<RefCell<Buffer>>,
}

impl OffscreenBuffer {
    pub fn new(fx: Effect, render_target: Rc<RefCell<Buffer>>) -> Self {
        Self { fx, render_target }
    }
}

impl Shader for OffscreenBuffer {
    fn name(&self) -> &'static str {
        "offscreen_buffer"
    }

    fn process(
        &mut self,
        duration: Duration,
        _buf: &mut Buffer,
        _area: Rect
    ) -> Option<Duration> {
        let area = self.area().unwrap(); // guaranteed to be Some
        let target = &mut self.render_target.as_ref().borrow_mut();
        self.fx.process(duration, target, area);

        None
    }

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {}

    fn done(&self) -> bool {
        self.fx.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.fx.area()
            .unwrap_or_else(|| *self.render_target.as_ref().borrow().area())
            .into()
    }

    fn set_area(&mut self, area: Rect) {
        self.fx.set_area(area);
    }

    fn set_cell_selection(&mut self, filter: CellFilter) {
        self.fx.set_cell_selection(filter);
    }

    fn as_effect_span(&self, offset: Duration) -> EffectSpan {
        EffectSpan::new(self, offset, vec![self.fx.as_effect_span(offset)])
    }
}