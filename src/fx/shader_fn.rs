use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::{CellFilter, CellIterator, EffectTimer, Shader};

#[derive(Clone)]
pub struct ShaderFn {
    code: Rc<RefCell<dyn FnMut(f32, Duration, Rect, CellIterator)>>,
    timer: EffectTimer,
    cell_filter: Option<CellFilter>,
    area: Option<Rect>,
}

impl ShaderFn {
    pub fn new<F, T>(
        code: F,
        timer: T
    ) -> Self
        where F: FnMut(f32, Duration, Rect, CellIterator) + 'static,
              T: Into<EffectTimer>
    {
        Self {
            code: Rc::new(RefCell::new(code)),
            timer: timer.into(),
            cell_filter: None,
            area: None
        }
    }
}

impl Shader for ShaderFn {
    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) -> Option<Duration> {
        let overflow = self.timer.process(duration);
        let alpha = self.timer.alpha();

        let requested_cells = self.cell_iter(buf, area);

        self.code.borrow_mut()(alpha, duration, area, requested_cells);

        overflow
    }

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {}

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

    fn set_cell_selection(&mut self, filter: CellFilter) {
        self.cell_filter = Some(filter);
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        self.cell_filter.clone()
    }
}