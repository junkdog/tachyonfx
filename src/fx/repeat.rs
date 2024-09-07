use ratatui::buffer::Buffer;
use ratatui::prelude::Rect;
use crate::{CellFilter, CellIterator, Duration, EffectTimer};

use crate::effect::Effect;
use crate::widget::EffectSpan;
use crate::shader::Shader;

#[derive(Clone)]
pub struct Repeat {
    fx: Effect,
    mode: RepeatMode,
    original_mode: RepeatMode,
}

impl Repeat {
    pub fn new(fx: Effect, mode: RepeatMode) -> Self {
        Self { fx, mode, original_mode: mode }
    }

    fn process_effect(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) -> Option<Duration> {
        match self.fx.process(duration, buf, area) {
            None => None,
            Some(overflow) => {
                self.fx.reset();
                Some(overflow)
            }
        }
    }
}

impl Shader for Repeat {
    fn name(&self) -> &'static str {
        "repeat"
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        match self.mode {
            RepeatMode::Forever => {
                let overflow = self.fx.process(duration, buf, area);
                if overflow.is_some() {
                    self.fx.reset();
                }
                None
            }
            RepeatMode::Times(1) => {
                let overflow = self.fx.process(duration, buf, area);
                if overflow.is_some() {
                    self.mode = RepeatMode::Times(0);
                }

                overflow
            }
            RepeatMode::Times(n) => {
                let overflow = self.fx.process(duration, buf, area);
                if overflow.is_some() {
                    self.mode = RepeatMode::Times(n - 1);
                    self.fx.reset();
                }

                overflow
            }
            RepeatMode::Duration(d) => {
                if d < duration {
                    let overflow = duration - d;
                    self.mode = RepeatMode::Duration(Duration::ZERO);
                    self.process_effect(d, buf, area).map(|d| Some(d + overflow)).unwrap_or(Some(overflow))
                } else {
                    self.mode = RepeatMode::Duration(d - duration);
                    self.process_effect(duration, buf, area)
                }
            }
        }
    }

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {
        // nothing to do
    }

    fn done(&self) -> bool {
        matches!(self.mode, RepeatMode::Times(0) | RepeatMode::Duration(Duration::ZERO))
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.fx.area()
    }

    fn set_area(&mut self, area: Rect) {
        self.fx.set_area(area)
    }

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        self.fx.set_cell_selection(strategy);
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
    }

    fn timer(&self) -> Option<EffectTimer> {
        match self.mode {
            RepeatMode::Forever     => self.fx.timer(),
            RepeatMode::Times(n)    => self.fx.timer().map(|t| t * n),
            RepeatMode::Duration(d) => Some(EffectTimer::from(d)),
        }
    }

    fn as_effect_span(&self, offset: Duration) -> EffectSpan {
        EffectSpan::new(self, offset, vec![self.fx.as_effect_span(offset)])
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        self.fx.cell_selection()
    }

    fn reset(&mut self) {
        self.fx.reset();
        self.mode = self.original_mode;
    }
}

#[derive(Clone, Copy)]
pub enum RepeatMode {
    Forever,
    Times(u32),
    Duration(Duration),
}