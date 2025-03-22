use crate::effect::Effect;
use crate::shader::Shader;
use crate::widget::EffectSpan;
use crate::{CellFilter, ColorSpace, Duration, EffectTimer};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

#[derive(Clone, Debug)]
pub struct NeverComplete {
    effect: Effect,
}

impl NeverComplete {
    pub fn new(effect: Effect) -> Self {
        Self { effect }
    }
}

impl Shader for NeverComplete {
    fn name(&self) -> &'static str {
        "never_complete"
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        self.effect.process(duration, buf, area);
        None
    }

    fn done(&self) -> bool                      { false }
    fn clone_box(&self) -> Box<dyn Shader>      { Box::new(self.clone()) }
    fn area(&self) -> Option<Rect>              { self.effect.area() }
    fn set_area(&mut self, area: Rect)          { self.effect.set_area(area) }

    fn filter(&mut self, strategy: CellFilter) {
        self.effect.filter(strategy);
    }

    fn reverse(&mut self) {
        self.effect.reverse()
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
    }

    fn cell_filter(&self) -> Option<CellFilter> {
        self.effect.cell_filter()
    }

    fn reset(&mut self) {
        self.effect.reset();
    }

    fn as_effect_span(&self, offset: Duration) -> EffectSpan {
        EffectSpan::new(self, offset, vec![self.effect.as_effect_span(offset)])
    }

    fn color_space(&self) -> ColorSpace {
        self.effect.color_space()
    }

    fn set_color_space(&mut self, color_space: ColorSpace) {
        self.effect.set_color_space(color_space);
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::EffectExpression;
        let nested = self.effect.to_dsl()?;
        EffectExpression::parse(&format!("fx::never_complete({})", nested))
    }
}

#[cfg(test)]
#[cfg(feature = "dsl")]
mod tests {
    use crate::fx::{consume_tick, never_complete};
    use crate::Shader;

    #[test]
    fn to_dsl() {
        let dsl = never_complete(consume_tick())
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, "fx::never_complete(fx::consume_tick())");
    }
}