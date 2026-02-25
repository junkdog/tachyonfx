use alloc::boxed::Box;
use core::{fmt, fmt::Debug};

use bon::{bon, Builder};
#[cfg(feature = "dsl")]
use compact_str::ToCompactString;
use ratatui_core::{buffer::Buffer, layout::Rect};

use crate::{
    cell_filter::FilterProcessor, default_shader_impl, fx::invoke_fn, ref_count, CellFilter,
    CellIterator, Duration, EffectTimer, RefCount, Shader, ThreadSafetyMarker,
};

#[derive(Builder, Clone)]
pub(super) struct ShaderFn<S: Clone> {
    state: S,
    original_state: Option<S>,
    name: &'static str,
    code: ShaderFnSignature<S>,

    #[builder(into)]
    timer: EffectTimer,

    cell_filter: Option<FilterProcessor>,
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
    where
        F: FnMut(&mut S, ShaderFnContext, CellIterator) + ThreadSafetyMarker + 'static,
    {
        Self::Iter(ref_count(f))
    }

    pub fn new_buffer<F>(f: F) -> Self
    where
        F: FnMut(&mut S, ShaderFnContext, &mut Buffer) + ThreadSafetyMarker + 'static,
    {
        Self::Buffer(ref_count(f))
    }
}

/// Context provided to the shader function, containing timing and area information.
#[derive(Debug)]
pub struct ShaderFnContext<'a> {
    pub last_tick: Duration,
    pub timer: &'a EffectTimer,
    pub area: Rect,
    filter: Option<FilterProcessor>,
}

impl<'a> ShaderFnContext<'a> {
    fn new(
        area: Rect,
        filter: Option<FilterProcessor>,
        last_tick: Duration,
        timer: &'a EffectTimer,
    ) -> Self {
        Self { last_tick, timer, area, filter }
    }

    pub fn alpha(&self) -> f32 {
        self.timer.alpha()
    }

    pub fn filter(&self) -> Option<&FilterProcessor> {
        self.filter.as_ref()
    }
}

#[bon]
impl<S: Clone + ThreadSafetyMarker + 'static> ShaderFn<S> {
    #[builder]
    pub(crate) fn with_iterator<F, T>(
        name: Option<&'static str>,
        state: S,
        code: F,
        timer: T,
        cell_filter: Option<CellFilter>,
        area: Option<Rect>,
    ) -> Self
    where
        F: FnMut(&mut S, ShaderFnContext, CellIterator) + ThreadSafetyMarker + 'static,
        T: Into<EffectTimer>,
    {
        Self {
            name: name.unwrap_or("shader_fn"),
            original_state: Some(state.clone()),
            state,
            code: ShaderFnSignature::new_iter(code),
            timer: timer.into(),
            cell_filter: cell_filter.map(FilterProcessor::from),
            area,
        }
    }

    #[builder]
    pub(crate) fn with_buffer<F, T>(
        name: Option<&'static str>,
        state: S,
        code: F,
        timer: T,
        cell_filter: Option<CellFilter>,
        area: Option<Rect>,
    ) -> Self
    where
        F: FnMut(&mut S, ShaderFnContext, &mut Buffer) + ThreadSafetyMarker + 'static,
        T: Into<EffectTimer>,
    {
        Self {
            name: name.unwrap_or("shader_fn"),
            original_state: Some(state.clone()),
            state,
            code: ShaderFnSignature::new_buffer(code),
            timer: timer.into(),
            cell_filter: cell_filter.map(FilterProcessor::from),
            area,
        }
    }
}

impl<S: Clone + ThreadSafetyMarker + 'static> Shader for ShaderFn<S> {
    default_shader_impl!(area, timer, filter, clone);

    fn name(&self) -> &'static str {
        self.name
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        let overflow = self.timer.process(duration);

        let cell_filter = self.cell_filter.as_ref().cloned();

        match self.code.clone() {
            ShaderFnSignature::Iter(f) => {
                let processor = self.cell_filter.as_ref();
                let cells = CellIterator::new(buf, area, processor);
                let ctx = ShaderFnContext::new(area, cell_filter, duration, &self.timer);
                invoke_fn!(f, &mut self.state, ctx, cells);
            },
            ShaderFnSignature::Buffer(f) => {
                let ctx = ShaderFnContext::new(area, cell_filter, duration, &self.timer);
                invoke_fn!(f, &mut self.state, ctx, buf);
            },
        }

        overflow
    }

    fn reset(&mut self) {
        self.timer.reset();
        self.state = self.original_state.as_ref().unwrap().clone();
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::DslError;
        Err(DslError::UnsupportedEffect { name: self.name().to_compact_string() })
    }
}

impl<S> Debug for ShaderFnSignature<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShaderFnSignature::Iter(_) => write!(f, "Iter(<function>)"),
            ShaderFnSignature::Buffer(_) => write!(f, "Buffer(<function>)"),
        }
    }
}

impl<S: Clone> Debug for ShaderFn<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShaderFn")
            .field("name", &self.name)
            .field("code", &self.code)
            .field("timer", &self.timer)
            .field("cell_filter", &self.cell_filter)
            .field("area", &self.area)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use alloc::{
        format,
        string::{String, ToString},
    };

    use ratatui_core::{buffer::Buffer, layout::Rect};

    use super::*;
    use crate::{EffectTimer, Interpolation::Linear};

    #[test]
    fn test_shader_fn_reset_preserves_original_state() {
        #[derive(Debug, Clone, PartialEq)]
        struct TestState {
            counter: u32,
            name: String,
        }

        let initial_state = TestState { counter: 0, name: "initial".to_string() };

        let mut shader = ShaderFn::with_iterator()
            .name("test_shader")
            .state(initial_state.clone())
            .code(|state: &mut TestState, _ctx, _cells| {
                state.counter += 1;
                state.name = format!("modified_{}", state.counter);
            })
            .timer(EffectTimer::from_ms(1000, Linear))
            .call();

        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));
        let area = Rect::new(0, 0, 10, 10);

        // Process to modify state
        shader.process(Duration::from_millis(100), &mut buf, area);
        assert_ne!(shader.state, initial_state);

        // Reset should restore original state
        shader.reset();
        assert_eq!(shader.state, initial_state);
        assert!(!shader.done());
    }

    #[test]
    fn test_effect_fn_preserves_original_state() {
        use crate::fx;

        #[derive(Debug, Clone, PartialEq)]
        struct TestState {
            counter: u32,
        }

        let initial_state = TestState { counter: 0 };

        let mut effect = fx::effect_fn(
            initial_state,
            EffectTimer::from_ms(1000, Linear),
            |state: &mut TestState, _ctx, _cells| {
                state.counter += 1;
            },
        );

        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 5));
        let area = Rect::new(0, 0, 5, 5);

        // Process multiple times to modify internal state
        effect.process(Duration::from_millis(100), &mut buf, area);
        effect.process(Duration::from_millis(100), &mut buf, area);

        // Reset should restore original state and timer
        effect.reset();
        assert!(!effect.done());
    }
}
