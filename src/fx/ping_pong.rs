use alloc::boxed::Box;

use ratatui_core::{buffer::Buffer, layout::Rect};

use crate::{CellFilter, ColorSpace, Duration, Effect, EffectTimer, Shader};

#[derive(Clone, Debug)]
pub(super) struct PingPong {
    fx: Effect,
    is_reversing: bool,
    strategy: CellFilter,
}

impl PingPong {
    pub fn new(fx: Effect) -> Self {
        Self {
            fx,
            is_reversing: false,
            strategy: CellFilter::default(),
        }
    }
}

impl Shader for PingPong {
    fn name(&self) -> &'static str {
        "ping_pong"
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        let overflow = self.fx.process(duration, buf, area);

        if overflow.is_some() && !self.is_reversing {
            self.is_reversing = true;
            self.fx.reset();
            self.fx.reverse();
            None // consumes any overflow when reversing, to reset the area
        } else {
            overflow
        }
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

    fn filter(&mut self, strategy: CellFilter) {
        self.strategy = strategy;
    }

    fn reverse(&mut self) {
        self.fx.reverse();
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
    }

    fn timer(&self) -> Option<EffectTimer> {
        self.fx.timer().as_ref().map(|t| *t * 2)
    }

    fn cell_filter(&self) -> Option<&CellFilter> {
        Some(&self.strategy)
    }

    fn set_color_space(&mut self, color_space: ColorSpace) {
        self.fx.set_color_space(color_space);
    }

    fn color_space(&self) -> ColorSpace {
        self.fx.color_space()
    }

    fn reset(&mut self) {
        // Do NOT reset the wrapped effect - ping_pong needs to preserve the effect's current
        // state to continue the animation seamlessly when direction reverses
        self.is_reversing = false;
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        crate::dsl::EffectExpression::parse(&format!("fx::ping_pong({})", self.fx.to_dsl()?))
    }
}
