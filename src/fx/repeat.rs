use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::prelude::Rect;

use crate::effect::{Effect, FilterMode};
use crate::render_effect::EffectRenderer;
use crate::shader::Shader;

#[derive(Clone)]
pub struct Repeat {
    fx: Effect,
    original: Effect,
    mode: RepeatMode
}

impl Repeat {
    pub fn new(fx: Effect, mode: RepeatMode) -> Self {
        let original = fx.clone();
        Self { fx, original, mode }
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
                self.fx = self.original.clone();
                Some(overflow)
            }
        }
    }
}

impl Shader for Repeat {
    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        match self.mode {
            RepeatMode::Forever => {
                let overflow = self.fx.process(duration, buf, area);
                if overflow.is_some() {
                    self.fx = self.original.clone();
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
                    self.fx = self.original.clone();
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

    fn done(&self) -> bool {
        match self.mode {
            RepeatMode::Times(0)                 => true,
            RepeatMode::Duration(Duration::ZERO) => true,
            _                                    => false,
        }
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

    fn cell_selection(&mut self, strategy: FilterMode) {
        self.fx.cell_selection(strategy);
    }
}

#[derive(Clone)]
pub enum RepeatMode {
    Forever,
    Times(u32),
    Duration(Duration),
}