use alloc::boxed::Box;

#[cfg(feature = "dsl")]
use compact_str::ToCompactString;
use ratatui_core::{
    buffer::Buffer,
    layout::{Rect, Size},
};

use crate::{
    effect::Effect, effect_timer::EffectTimer, interpolation::Interpolatable,
    rect_ext::CenteredShrink, shader::Shader, CellFilter, ColorSpace, Duration,
};

#[derive(Clone, Debug)]
pub(super) struct ResizeArea {
    fx: Option<Effect>,
    area: Option<Rect>,
    original_area: Option<Rect>,
    initial_size: Size,
    timer: EffectTimer,
}

impl ResizeArea {
    pub fn new(fx: Option<Effect>, initial_size: Size, timer: EffectTimer) -> Self {
        Self {
            fx,
            initial_size,
            timer,
            original_area: None,
            area: None,
        }
    }
}

impl Shader for ResizeArea {
    fn name(&self) -> &'static str {
        "resize_area"
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        if self.original_area.is_none() {
            self.original_area = Some(area);
        }

        let target_area = self.original_area.unwrap();

        let a = self.timer.alpha();
        let overflow = self.timer.process(duration);

        let w = self
            .initial_size
            .width
            .lerp(&target_area.width, a);
        let h = self
            .initial_size
            .height
            .lerp(&target_area.height, a);

        let resized_area = target_area.inner_centered(w, h);
        for y in resized_area.top()..resized_area.bottom() {
            for x in resized_area.left()..resized_area.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.reset();
                }
            }
        }

        self.set_area(resized_area);

        if let Some(fx) = &mut self.fx {
            fx.set_area(resized_area);
            let hosted_overflow = fx.process(duration, buf, resized_area);
            // only return the overflow if the fx is done and this translate is done
            match (overflow, hosted_overflow) {
                (Some(a), Some(b)) => Some(a.min(b)),
                _ => None,
            }
        } else {
            overflow
        }
    }

    fn done(&self) -> bool {
        self.timer.done()
            && (self
                .fx
                .as_ref()
                .is_some_and(super::super::effect::Effect::done)
                || self.fx.is_none())
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.area
    }

    fn set_area(&mut self, area: Rect) {
        self.area = Some(area);
        if let Some(fx) = self.fx.as_mut() {
            fx.set_area(area);
        }
    }

    fn filter(&mut self, strategy: CellFilter) {
        if let Some(fx) = self.fx.as_mut() {
            fx.filter(strategy);
        }
    }

    fn set_color_space(&mut self, color_space: ColorSpace) {
        if let Some(fx) = self.fx.as_mut() {
            fx.set_color_space(color_space);
        }
    }

    fn color_space(&self) -> ColorSpace {
        self.fx
            .as_ref()
            .map(super::super::effect::Effect::color_space)
            .unwrap_or_default()
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn timer(&self) -> Option<EffectTimer> {
        Some(self.timer)
    }

    fn cell_filter(&self) -> Option<&CellFilter> {
        self.fx.as_ref().and_then(Effect::cell_filter)
    }

    fn reset(&mut self) {
        self.timer.reset();
        if let Some(fx) = self.fx.as_mut() {
            fx.reset();
        }
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        Err(crate::dsl::DslError::UnsupportedEffect { name: self.name().to_compact_string() })
    }
}
