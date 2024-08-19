use std::time::Duration;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use crate::{CellFilter, CellIterator, Effect, EffectTimer, Shader};
use crate::Interpolation::Linear;
use crate::widget::EffectSpan;

/// Specifies the position where the additional duration should be applied in a `Prolong` effect.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ProlongPosition {
    Start,
    End,
}

#[derive(Clone)]
pub struct Prolong {
    inner: Effect,
    timer: EffectTimer,
    position: ProlongPosition,
}

impl Prolong {
    pub fn new(
        position: ProlongPosition,
        additional_duration: EffectTimer,
        inner: Effect,
    ) -> Self {
        Self {
            inner,
            timer: additional_duration,
            position,
        }
    }
}

/// A shader that wraps an inner effect and prolongs its duration either at the start or end.
impl Shader for Prolong {
    fn name(&self) -> &'static str {
        match self.position {
            ProlongPosition::Start => "prolong_start",
            ProlongPosition::End   => "prolong_end",
        }
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        match self.position {
            ProlongPosition::Start => {
                let overflow = self.timer.process(duration);
                self.inner.process(overflow.unwrap_or_default(), buf, area)
            }
            ProlongPosition::End => {
                let overflow = self.inner.process(duration, buf, area);
                self.timer.process(overflow?)
            }
        }
    }

    fn execute(&mut self, alpha: f32, area: Rect, cell_iter: CellIterator) {}

    /// Checks if the prolonged effect is done.
    ///
    /// # Returns
    ///
    /// `true` if both the additional duration and inner effect are done, `false` otherwise.
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

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        self.inner.set_cell_selection(strategy);
    }

    /// Returns the total duration of the prolonged effect.
    ///
    /// # Returns
    ///
    /// An `EffectTimer` representing the sum of the additional duration and the inner effect's duration.
    fn timer(&self) -> Option<EffectTimer> {
        let self_duration = self.timer.duration();
        let inner_duration = self.inner.timer().unwrap_or_default().duration();

        Some(EffectTimer::new(self_duration + inner_duration, Linear))
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        self.inner.cell_selection()
    }

    fn as_effect_span(&self, offset: Duration) -> EffectSpan {
        let inner_offset = match self.position {
            ProlongPosition::Start => offset + self.timer.duration(),
            ProlongPosition::End   => offset
        };
        EffectSpan::new(self, offset, vec![self.inner.as_effect_span(inner_offset)])
    }

    fn reset(&mut self) {
        self.timer.reset();
        self.inner.reset();
    }
}