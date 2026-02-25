use alloc::boxed::Box;

use ratatui_core::{buffer::Buffer, layout::Rect};

use crate::{CellFilter, ColorSpace, Duration, Effect, EffectTimer, RefRect, Shader};

/// A shader wrapper that applies effects to a dynamically changing rectangular area.
///
/// `DynamicArea` solves the problem of effects needing to adapt to changing layout areas
/// in real-time. Unlike regular effects which have static areas, `DynamicArea` uses a
/// shared, mutable area reference that can be updated during effect execution.
///
/// This is particularly useful in terminal UIs where widget areas frequently change due
/// to:
/// - Window resizing
/// - Dynamic layout updates
/// - Responsive design adjustments
/// - Content-driven sizing
#[derive(Clone, Debug)]
pub(super) struct DynamicArea {
    rect: RefRect,
    fx: Effect,
}

impl DynamicArea {
    /// Creates a new `DynamicArea` with the given area reference and effect.
    ///
    /// # Arguments
    ///
    /// * `area` - A shared reference to the rectangular area where the effect will be
    ///   applied
    /// * `fx` - The effect to wrap with dynamic area capabilities
    ///
    /// # Returns
    ///
    /// A new `DynamicArea` that will apply the effect to the area referenced by `area`,
    /// with the ability to update the area dynamically during execution.
    pub fn new(area: RefRect, fx: Effect) -> Self {
        Self { rect: area, fx }
    }
}

impl Shader for DynamicArea {
    fn name(&self) -> &'static str {
        "dynamic_area"
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, _area: Rect) -> Option<Duration> {
        self.fx.process(duration, buf, self.rect.get())
    }

    fn done(&self) -> bool {
        self.fx.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        Some(self.rect.get())
    }

    fn set_area(&mut self, area: Rect) {
        self.rect.set(area);
    }

    fn filter(&mut self, filter: CellFilter) {
        self.fx.filter(filter);
    }

    fn timer(&self) -> Option<EffectTimer> {
        self.fx.timer()
    }

    fn cell_filter(&self) -> Option<&CellFilter> {
        self.fx.cell_filter()
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
