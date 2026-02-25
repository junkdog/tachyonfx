use alloc::boxed::Box;

use ratatui_core::{buffer::Buffer, layout::Rect};

use crate::{effect::Effect, shader::Shader, CellFilter, ColorSpace, Duration, EffectTimer};

#[derive(Clone, Debug)]
pub(super) struct Repeat {
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
        area: Rect,
    ) -> Option<Duration> {
        match self.fx.process(duration, buf, area) {
            None => None,
            Some(overflow) => {
                self.fx.reset();
                Some(overflow)
            },
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
            },
            RepeatMode::Times(1) => {
                let overflow = self.fx.process(duration, buf, area);
                if overflow.is_some() {
                    self.mode = RepeatMode::Times(0);
                }

                overflow
            },
            RepeatMode::Times(n) => {
                let overflow = self.fx.process(duration, buf, area);
                if overflow.is_some() {
                    self.mode = RepeatMode::Times(n - 1);
                    self.fx.reset();
                }

                overflow
            },
            RepeatMode::Duration(d) => {
                if d < duration {
                    let overflow = duration - d;
                    self.mode = RepeatMode::Duration(Duration::ZERO);
                    self.process_effect(d, buf, area)
                        .map_or(Some(overflow), |d| Some(d + overflow))
                } else {
                    self.mode = RepeatMode::Duration(d - duration);
                    self.process_effect(duration, buf, area)
                }
            },
        }
    }

    fn done(&self) -> bool {
        matches!(
            self.mode,
            RepeatMode::Times(0) | RepeatMode::Duration(Duration::ZERO)
        )
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
        self.fx.filter(strategy);
    }

    fn set_color_space(&mut self, color_space: ColorSpace) {
        self.fx.set_color_space(color_space);
    }

    fn color_space(&self) -> ColorSpace {
        self.fx.color_space()
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
    }

    fn timer(&self) -> Option<EffectTimer> {
        match self.mode {
            RepeatMode::Forever => self.fx.timer(),
            RepeatMode::Times(n) => self.fx.timer().map(|t| t * n),
            RepeatMode::Duration(d) => Some(EffectTimer::from(d)),
        }
    }

    fn cell_filter(&self) -> Option<&CellFilter> {
        self.fx.cell_filter()
    }

    fn reset(&mut self) {
        self.fx.reset();
        self.mode = self.original_mode;
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::DslFormat;

        let fx = self.fx.to_dsl()?;
        crate::dsl::EffectExpression::parse(&format!(
            "fx::repeat({fx}, {})",
            self.mode.dsl_format()
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RepeatMode {
    Forever,
    Times(u32),
    Duration(Duration),
}

#[cfg(test)]
#[cfg(feature = "dsl")]
mod tests {
    use indoc::indoc;

    use crate::{
        fx::{consume_tick, repeat, RepeatMode},
        Duration,
    };

    #[test]
    fn to_dsl() {
        let dsl = repeat(consume_tick(), RepeatMode::Forever)
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, indoc! {
        "fx::repeat(fx::consume_tick(), RepeatMode::Forever)"});

        let dsl = repeat(consume_tick(), RepeatMode::Times(2))
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, indoc! {
        "fx::repeat(fx::consume_tick(), RepeatMode::Times(2))"});

        let dsl = repeat(
            consume_tick(),
            RepeatMode::Duration(Duration::from_millis(1)),
        )
        .to_dsl()
        .unwrap()
        .to_string();

        assert_eq!(dsl, indoc! {
            "fx::repeat(fx::consume_tick(), RepeatMode::Duration(Duration::from_millis(1)))"
        });
    }
}
