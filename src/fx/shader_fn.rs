use bon::{bon, builder, Builder};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::{ref_count, CellFilter, CellIterator, Duration, EffectTimer, RefCount, Shader};
use crate::fx::invoke_fn;
use crate::ThreadSafetyMarker;

#[derive(Builder, Clone)]
pub struct ShaderFn<S: Clone> {
    state: S,
    original_state: Option<S>,
    name: &'static str,
    code: ShaderFnSignature<S>,

    #[builder(into)]
    timer: EffectTimer,

    cell_filter: Option<CellFilter>,
    area: Option<Rect>,
}

#[cfg(feature = "sendable")]
type FnIterSignature<S> = dyn FnMut(&mut S, ShaderFnContext, CellIterator) + Send + 'static;
#[cfg(feature = "sendable")]
type FnBufSignature<S> = dyn FnMut(&mut S, ShaderFnContext, &mut Buffer) + Send + 'static;

#[cfg(not(feature = "sendable"))]
type FnIterSignature<S> = dyn FnMut(&mut S, ShaderFnContext, CellIterator) + 'static;
#[cfg(not(feature = "sendable"))]
type FnBufSignature<S> = dyn FnMut(&mut S, ShaderFnContext, &mut Buffer) + 'static;

#[derive(Clone)]
pub enum ShaderFnSignature<S> {
    Iter(RefCount<FnIterSignature<S>>),
    Buffer(RefCount<FnBufSignature<S>>),
}

impl<S> ShaderFnSignature<S> {
    pub fn new_iter<F>(f: F) -> Self
        where F: FnMut(&mut S, ShaderFnContext, CellIterator) + ThreadSafetyMarker + 'static
    {
        Self::Iter(ref_count(f))
    }

    pub fn new_buffer<F>(f: F) -> Self
        where F: FnMut(&mut S, ShaderFnContext, &mut Buffer) + ThreadSafetyMarker + 'static
    {
        Self::Buffer(ref_count(f))
    }
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

#[bon]
impl<S: Clone + ThreadSafetyMarker + 'static> ShaderFn<S> {
    #[builder]
    pub(self) fn with_iterator<F, T>(
        name: Option<&'static str>,
        state: S,
        code: F,
        timer: T,
        cell_filter: Option<CellFilter>,
        area: Option<Rect>
    ) -> Self
    where F: FnMut(&mut S, ShaderFnContext, CellIterator) + ThreadSafetyMarker + 'static,
          T: Into<EffectTimer>
    {
        Self {
            name: name.unwrap_or("shader_fn"),
            original_state: Some(state.clone()),
            state,
            code: ShaderFnSignature::new_iter(code),
            timer: timer.into(),
            cell_filter,
            area
        }
    }

    #[builder]
    pub(self) fn with_buffer<F, T>(
        name: Option<&'static str>,
        state: S,
        code: F,
        timer: T,
        cell_filter: Option<CellFilter>,
        area: Option<Rect>
    ) -> Self
    where F: FnMut(&mut S, ShaderFnContext, &mut Buffer) + ThreadSafetyMarker + 'static,
          T: Into<EffectTimer>
    {
        Self {
            name: name.unwrap_or("shader_fn"),
            original_state: Some(state.clone()),
            state,
            code: ShaderFnSignature::new_buffer(code),
            timer: timer.into(),
            cell_filter,
            area
        }
    }
}


impl<S: Clone + ThreadSafetyMarker + 'static> Shader for ShaderFn<S> {
    fn name(&self) -> &'static str {
        self.name
    }

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
                invoke_fn!(f, &mut self.state, ctx, cells)
            }
            ShaderFnSignature::Buffer(f) => {
                let ctx = ShaderFnContext::new(area, self.cell_filter.clone(), duration, &self.timer);
                invoke_fn!(f, &mut self.state, ctx, buf)
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
        self.state = self.original_state.as_ref().unwrap().clone();
    }
}
