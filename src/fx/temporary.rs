use alloc::boxed::Box;

use ratatui_core::{buffer::Buffer, layout::Rect};

use crate::{
    effect::{Effect, IntoEffect},
    effect_timer::EffectTimer,
    interpolation::Interpolation::Linear,
    shader::Shader,
    CellFilter, ColorSpace, Duration,
};

#[derive(Clone, Debug)]
pub(super) struct TemporaryEffect {
    effect: Effect,
    timer: EffectTimer,
}

impl TemporaryEffect {
    pub fn new(effect: Effect, duration: Duration) -> Self {
        Self { effect, timer: EffectTimer::new(duration, Linear) }
    }
}

impl Shader for TemporaryEffect {
    fn name(&self) -> &'static str {
        "with_duration"
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        let remaining = self.timer.process(duration);
        let effect_area = self.effect.area().unwrap_or(area);
        self.effect.process(duration, buf, effect_area);
        remaining
    }

    fn done(&self) -> bool {
        self.timer.done() || self.effect.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.effect.area()
    }

    fn set_area(&mut self, area: Rect) {
        self.effect.set_area(area);
    }

    fn filter(&mut self, strategy: CellFilter) {
        self.effect.filter(strategy);
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn timer(&self) -> Option<EffectTimer> {
        Some(self.timer)
    }

    fn cell_filter(&self) -> Option<&CellFilter> {
        self.effect.cell_filter()
    }

    fn reset(&mut self) {
        self.effect.reset();
        self.timer.reset();
    }

    fn set_color_space(&mut self, color_space: ColorSpace) {
        self.effect.set_color_space(color_space);
    }

    fn color_space(&self) -> ColorSpace {
        self.effect.color_space()
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::{DslFormat, EffectExpression};
        EffectExpression::parse(&format!(
            "fx::with_duration({}, {})",
            self.timer.duration().dsl_format(),
            self.effect.to_dsl()?
        ))
    }
}

pub trait IntoTemporaryEffect {
    fn with_duration(self, duration: Duration) -> Effect;
}

impl IntoTemporaryEffect for Effect {
    fn with_duration(self, duration: Duration) -> Effect {
        TemporaryEffect::new(self, duration).into_effect()
    }
}

#[cfg(test)]
#[cfg(feature = "dsl")]
mod tests {
    use indoc::indoc;

    use crate::{fx, Duration};

    #[test]
    fn to_dsl() {
        let dsl = fx::with_duration(Duration::from_millis(1000), fx::sleep(100))
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, indoc! {
            "fx::with_duration(Duration::from_millis(1000), fx::sleep(100))"
        });
    }
}
