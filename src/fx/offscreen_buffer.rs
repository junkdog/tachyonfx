use alloc::boxed::Box;

#[cfg(feature = "dsl")]
use compact_str::ToCompactString;
use ratatui_core::{buffer::Buffer, layout::Rect};

use crate::{CellFilter, ColorSpace, Duration, Effect, RefCount, Shader};

#[derive(Clone, Debug)]
pub(super) struct OffscreenBuffer {
    fx: Effect,
    render_target: RefCount<Buffer>,
}

impl OffscreenBuffer {
    pub fn new(fx: Effect, render_target: RefCount<Buffer>) -> Self {
        Self { fx, render_target }
    }
}

impl Shader for OffscreenBuffer {
    fn name(&self) -> &'static str {
        "offscreen_buffer"
    }

    fn process(&mut self, duration: Duration, _buf: &mut Buffer, _area: Rect) -> Option<Duration> {
        let area = self.area().unwrap(); // guaranteed to be Some
        #[cfg(not(feature = "sendable"))]
        {
            let target = &mut self.render_target.as_ref().borrow_mut();
            self.fx.process(duration, target, area);
        };
        #[cfg(feature = "sendable")]
        {
            let mut target = self.render_target.lock().unwrap();
            self.fx.process(duration, &mut target, area);
        };

        None
    }

    fn done(&self) -> bool {
        self.fx.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    #[cfg(not(feature = "sendable"))]
    fn area(&self) -> Option<Rect> {
        self.fx
            .area()
            .unwrap_or_else(|| *self.render_target.as_ref().borrow().area())
            .into()
    }

    #[cfg(feature = "sendable")]
    fn area(&self) -> Option<Rect> {
        self.fx
            .area()
            .unwrap_or_else(|| self.render_target.lock().unwrap().area)
            .into()
    }

    fn set_area(&mut self, area: Rect) {
        self.fx.set_area(area);
    }

    fn filter(&mut self, filter: CellFilter) {
        self.fx.filter(filter);
    }

    fn cell_filter(&self) -> Option<&CellFilter> {
        self.fx.cell_filter()
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::DslError;
        Err(DslError::UnsupportedEffect { name: self.name().to_compact_string() })
    }

    fn set_color_space(&mut self, color_space: ColorSpace) {
        self.fx.set_color_space(color_space);
    }

    fn color_space(&self) -> ColorSpace {
        self.fx.color_space()
    }

    fn reset(&mut self) {
        self.fx.reset();
    }
}
