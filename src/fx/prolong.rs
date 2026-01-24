use alloc::boxed::Box;

use ratatui_core::{buffer::Buffer, layout::Rect};

use crate::{CellFilter, ColorSpace, Duration, Effect, EffectTimer, Interpolation::Linear, Shader};

/// Specifies the position where the additional duration should be applied in a `Prolong`
/// effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ProlongPosition {
    Start,
    End,
}

#[derive(Clone, Debug)]
pub(super) struct Prolong {
    inner: Effect,
    timer: EffectTimer,
    position: ProlongPosition,
}

impl Prolong {
    pub fn new(position: ProlongPosition, additional_duration: EffectTimer, inner: Effect) -> Self {
        Self { inner, timer: additional_duration, position }
    }
}

/// A shader that wraps an inner effect and prolongs its duration either at the start or
/// end.
impl Shader for Prolong {
    fn name(&self) -> &'static str {
        match self.position {
            ProlongPosition::Start => "prolong_start",
            ProlongPosition::End => "prolong_end",
        }
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        match self.position {
            ProlongPosition::Start => {
                let overflow = self.timer.process(duration);
                self.inner
                    .process(overflow.unwrap_or_default(), buf, area)
            },
            ProlongPosition::End => {
                let overflow = self.inner.process(duration, buf, area);
                self.timer.process(overflow?)
            },
        }
    }

    /// Checks if the prolonged effect is done.
    ///
    /// # Returns
    ///
    /// `true` if both the additional duration and inner effect are done, `false`
    /// otherwise.
    fn done(&self) -> bool {
        self.timer.done() && self.inner.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.inner.area()
    }

    fn set_area(&mut self, area: Rect) {
        self.inner.set_area(area);
    }

    fn filter(&mut self, strategy: CellFilter) {
        self.inner.filter(strategy);
    }

    fn set_color_space(&mut self, color_space: ColorSpace) {
        self.inner.set_color_space(color_space);
    }

    fn color_space(&self) -> ColorSpace {
        self.inner.color_space()
    }

    /// Returns the total duration of the prolonged effect.
    ///
    /// # Returns
    ///
    /// An `EffectTimer` representing the sum of the additional duration and the inner
    /// effect's duration.
    fn timer(&self) -> Option<EffectTimer> {
        let self_duration = self.timer.duration();
        let inner_duration = self.inner.timer().unwrap_or_default().duration();

        Some(EffectTimer::new(self_duration + inner_duration, Linear))
    }

    fn cell_filter(&self) -> Option<&CellFilter> {
        self.inner.cell_filter()
    }

    fn reset(&mut self) {
        self.timer.reset();
        self.inner.reset();
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::{DslFormat, EffectExpression};

        EffectExpression::parse(&format!(
            "fx::{}({}, {})",
            self.name(),
            self.timer.dsl_format(),
            self.inner.to_dsl()?
        ))
    }
}

#[cfg(test)]
#[cfg(feature = "dsl")]
mod tests {
    use indoc::indoc;

    use crate::{fx, fx::consume_tick};

    #[test]
    fn to_dsl_prolong_start() {
        let dsl = fx::prolong_start(100, consume_tick())
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, indoc! {
            "fx::prolong_start(EffectTimer::from_ms(100, Interpolation::Linear), fx::consume_tick())"
        });
    }

    #[test]
    fn to_dsl_prolong_end() {
        let dsl = fx::prolong_end(100, consume_tick())
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, indoc! {
            "fx::prolong_end(EffectTimer::from_ms(100, Interpolation::Linear), fx::consume_tick())"
        });
    }
}
