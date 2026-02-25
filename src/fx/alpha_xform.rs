use alloc::boxed::Box;
use core::ops::Range;

use ratatui_core::{buffer::Buffer, layout::Rect};
use Interpolation::Linear;

use crate::{
    default_shader_impl, math, CellFilter, ColorSpace, Duration, Effect, EffectTimer,
    Interpolation, Shader,
};

#[derive(Debug, Clone)]
pub(super) struct FreezeAt {
    alpha: f32,
    set_raw_alpha: bool,
    fx: Effect,
}

/// An effect that freezes another effect at a specific alpha (transition) value.
///
/// `FixedAlpha` sets the inner effect to a specific alpha value and keeps it there
/// indefinitely, effectively "freezing" the effect at that stage of its transition.
///
/// # Examples
///
/// ```
/// use tachyonfx::{fx, EffectTimer, Interpolation};
/// use ratatui_core::style::Color;
///
/// // Create a fade effect that stops at 70% of its transition
/// let fade = fx::fade_to_fg(Color::Red, (1000, Interpolation::Linear));
/// let frozen_fade = fx::freeze_at(0.7, false, fade);
/// ```
impl FreezeAt {
    pub fn new(alpha: f32, set_raw_alpha: bool, fx: Effect) -> Self {
        let alpha = alpha.clamp(0.0, 1.0);
        Self { alpha, fx, set_raw_alpha }
    }
}

impl Shader for FreezeAt {
    default_shader_impl!(clone);

    fn name(&self) -> &'static str {
        "freeze_at"
    }

    fn process(&mut self, _duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        if let Some(t) = self.fx.timer_mut() {
            // fix alpha on first frame
            if !t.started() {
                let interpolation = if self.set_raw_alpha { Linear } else { t.interpolation() };
                let d = t.remaining().as_secs_f32() * (1.0 - self.alpha);
                *t = EffectTimer::new(t.remaining(), interpolation);
                t.process(Duration::from_secs_f32(d));
            }
        }

        self.fx
            .process(Duration::from_millis(0), buf, area)
    }

    fn done(&self) -> bool {
        self.fx.timer().is_none()
    }

    fn area(&self) -> Option<Rect> {
        self.fx.area()
    }

    fn set_area(&mut self, area: Rect) {
        self.fx.set_area(area);
    }

    fn filter(&mut self, filter: CellFilter) {
        self.fx.filter(filter);
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
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

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        crate::dsl::EffectExpression::parse(&format!(
            "fx::freeze_at({}, {}, {})",
            self.alpha,
            self.set_raw_alpha,
            self.fx.to_dsl()?
        ))
    }

    fn reset(&mut self) {
        self.fx.reset();
    }
}

#[derive(Debug, Clone)]
pub(super) struct RemapAlpha {
    raw_alpha_range: Range<f32>,
    fx: Effect,
    timer: EffectTimer, // copy of fx timer but linear interpolation
    rest: f32,
}

impl RemapAlpha {
    pub fn new(raw_alpha_range: Range<f32>, fx: Effect) -> Self {
        let timer = fx.timer().unwrap_or_default();

        let start = raw_alpha_range.start;
        let end = raw_alpha_range.end;
        let raw_alpha_range = start.clamp(0.0, 1.0)..end.clamp(0.0, 1.0);

        let rest = 0.0;
        Self { raw_alpha_range, fx, timer, rest }
    }
}

impl Shader for RemapAlpha {
    default_shader_impl!(clone);

    fn name(&self) -> &'static str {
        "remap_alpha"
    }

    fn execute(&mut self, duration: Duration, area: Rect, buf: &mut Buffer) {
        if let Some(t) = self.fx.timer_mut() {
            if !t.started() {
                // deduct initial duration from the timer
                let skip_initial = t.duration().as_secs_f32() * self.raw_alpha_range.start;
                t.process(Duration::from_secs_f32(skip_initial));
            }
        }

        let range = self.raw_alpha_range.end - self.raw_alpha_range.start;
        let scaled_duration_ms = 1_000.0 * (duration.as_secs_f32() * range) + self.rest;

        self.fx
            .process(Duration::from_millis(scaled_duration_ms as _), buf, area);
        self.rest = scaled_duration_ms - math::floor(scaled_duration_ms);
    }

    fn done(&self) -> bool {
        self.timer.done()
    }

    fn area(&self) -> Option<Rect> {
        self.fx.area()
    }

    fn set_area(&mut self, area: Rect) {
        self.fx.set_area(area);
    }

    fn filter(&mut self, filter: CellFilter) {
        self.fx.filter(filter);
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn timer(&self) -> Option<EffectTimer> {
        Some(self.timer)
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

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        crate::dsl::EffectExpression::parse(&format!(
            "fx::remap_alpha({}, {}, {})",
            self.raw_alpha_range.start,
            self.raw_alpha_range.end,
            self.fx.to_dsl()?
        ))
    }

    fn reset(&mut self) {
        self.rest = 0.0;
        self.timer.reset();
        self.fx.reset();
    }
}
