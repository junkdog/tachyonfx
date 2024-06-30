use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::{CellFilter, CellIterator, EffectTimer, Shader};

#[derive(Clone)]
pub struct ShaderFn<S> {
    state: S,
    code: Rc<RefCell<dyn FnMut(&mut S, ShaderFnContext, CellIterator)>>,
    timer: EffectTimer,
    cell_filter: Option<CellFilter>,
    area: Option<Rect>,
}

/// Context provided to the shader function, containing timing and area information.
pub struct ShaderFnContext<'a> {
    pub last_tick: Duration,
    pub timer: &'a EffectTimer,
    pub area: Rect,
}

impl<'a> ShaderFnContext<'a> {
    pub fn new(area: Rect, last_tick: Duration, timer: &'a EffectTimer) -> Self {
        Self {
            last_tick,
            timer,
            area
        }
    }

    pub fn alpha(&self) -> f32 {
        self.timer.alpha()
    }
}

impl<S: Clone + 'static> ShaderFn<S> {
    pub fn new<F, T>(
        state: S,
        code: F,
        timer: T
    ) -> Self
        where F: FnMut(&mut S, ShaderFnContext, CellIterator) + 'static,
              T: Into<EffectTimer>
    {
        Self {
            state,
            code: Rc::new(RefCell::new(code)),
            timer: timer.into(),
            cell_filter: None,
            area: None
        }
    }
}

impl<S: Clone + 'static> Shader for ShaderFn<S> {
    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) -> Option<Duration> {
        let overflow = self.timer.process(duration);

        let requested_cells = self.cell_iter(buf, area);
        self.code.borrow_mut()(&mut self.state, ShaderFnContext::new(area, duration, &self.timer), requested_cells);

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
