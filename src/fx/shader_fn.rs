use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::{CellFilter, CellIterator, EffectTimer, Shader};

#[derive(Clone)]
pub struct ShaderFn<S: Clone> {
    state: S,
    original_state: S,
    code: ShaderFnSignature<S>,
    timer: EffectTimer,
    cell_filter: Option<CellFilter>,
    area: Option<Rect>,
}

#[derive(Clone)]
enum ShaderFnSignature<S> {
    Iter(Rc<RefCell<dyn FnMut(&mut S, ShaderFnContext, CellIterator)>>),
    Buffer(Rc<RefCell<dyn FnMut(&mut S, ShaderFnContext, &mut Buffer)>>),
}


/// Context provided to the shader function, containing timing and area information.
pub struct ShaderFnContext<'a> {
    pub last_tick: Duration,
    pub timer: &'a EffectTimer,
    pub area: Rect,
    pub filter: Option<CellFilter>,
}

impl<'a> ShaderFnContext<'a> {
    fn new(
        area: Rect,
        filter: Option<CellFilter>,
        last_tick: Duration,
        timer: &'a EffectTimer
    ) -> Self {
        Self {
            last_tick,
            timer,
            area,
            filter,
        }
    }

    pub fn alpha(&self) -> f32 {
        self.timer.alpha()
    }
}

impl<S: Clone + 'static> ShaderFn<S> {
    pub fn with_iterator<F, T>(
        state: S,
        code: F,
        timer: T
    ) -> Self
        where F: FnMut(&mut S, ShaderFnContext, CellIterator) + 'static,
              T: Into<EffectTimer>
    {
        Self {
            original_state: state.clone(),
            state,
            code: ShaderFnSignature::Iter(Rc::new(RefCell::new(code))),
            timer: timer.into(),
            cell_filter: None,
            area: None
        }
    }

    pub fn with_buffer<F, T>(
        state: S,
        code: F,
        timer: T
    ) -> Self
        where F: FnMut(&mut S, ShaderFnContext, &mut Buffer) + 'static,
              T: Into<EffectTimer>
    {
        Self {
            original_state: state.clone(),
            state,
            code: ShaderFnSignature::Buffer(Rc::new(RefCell::new(code))),
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

        match self.code.clone() {
            ShaderFnSignature::Iter(f) => {
                let cells = self.cell_iter(buf, area);
                let ctx = ShaderFnContext::new(area, self.cell_filter.clone(), duration, &self.timer);
                f.borrow_mut()(&mut self.state, ctx, cells)
            }
            ShaderFnSignature::Buffer(f) => {
                let ctx = ShaderFnContext::new(area, self.cell_filter.clone(), duration, &self.timer);
                f.borrow_mut()(&mut self.state, ctx, buf)
            }
        }

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

    fn reset(&mut self) {
        self.timer.reset();
        self.state = self.original_state.clone();
    }
}
