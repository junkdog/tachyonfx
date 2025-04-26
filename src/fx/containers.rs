use ratatui::buffer::Buffer;
use ratatui::layout::{Rect};
use crate::{CellFilter, ColorSpace, Duration, EffectTimer};
use crate::effect::Effect;
use crate::widget::EffectSpan;
use crate::Interpolation::Linear;
use crate::shader::Shader;

#[derive(Default, Clone, Debug)]
pub struct SequentialEffect {
    effects: Vec<Effect>,
    current: usize,
}

#[derive(Default, Clone, Debug)]
pub struct ParallelEffect {
    effects: Vec<Effect>,
}

impl SequentialEffect {
    pub fn new(effects: Vec<Effect>) -> Self {
        Self { effects, current: 0 }
    }
}

impl ParallelEffect {
    pub fn new(effects: Vec<Effect>) -> Self {
        Self { effects }
    }
}

impl Shader for ParallelEffect {
    fn name(&self) -> &'static str {
        "parallel"
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        let mut remaining = Some(duration);

        for effect in self.effects.iter_mut().filter(|e| e.running()) {
            let effect_area = effect.area().unwrap_or(area);
            match effect.process(duration, buf, effect_area) {
                None => remaining = None,
                Some(d) if remaining.is_some() => {
                    remaining = Some(d.min(remaining.unwrap()));
                }
                _ => (),
            }
        }

        remaining
    }

    fn done(&self) -> bool {
        self.effects.iter().all(Effect::done)
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        None
    }

    fn set_area(&mut self, area: Rect) {
        self.effects.iter_mut().for_each(|e| e.set_area(area));
    }

    fn filter(&mut self, filter: CellFilter) {
        self.effects.iter_mut().for_each(|e| e.filter(filter.clone()));
    }

    fn reverse(&mut self) {
        self.effects.iter_mut().for_each(Effect::reverse)
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
    }

    fn timer(&self) -> Option<EffectTimer> {
        self.effects.iter()
            .filter_map(|fx| fx.timer())
            .map(|t| t.duration())
            .max()
            .map(|d| EffectTimer::new(d, Linear))
    }

    fn cell_filter(&self) -> Option<CellFilter> {
        None
    }

    fn reset(&mut self) {
        self.effects.iter_mut().for_each(Effect::reset)
    }

    fn as_effect_span(&self, offset: Duration) -> EffectSpan {
        let children = self.effects.iter()
            .map(|e| e.as_effect_span(offset))
            .collect();

        EffectSpan::new(self, offset, children)
    }



    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        to_dsl(self.name(), &self.effects)
    }

    fn set_color_space(&mut self, color_space: ColorSpace) {
        self.effects.iter_mut().for_each(|e| e.set_color_space(color_space));
    }
}

impl Shader for SequentialEffect {
    fn name(&self) -> &'static str {
        "sequence"
    }

    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect,
    ) -> Option<Duration> {

        let mut remaining = Some(duration);
        while remaining.is_some() && !self.done() {
            let effect = &mut self.effects[self.current];
            let effect_area = effect.area().unwrap_or(area);
            remaining = effect.process(remaining.unwrap(), buf, effect_area);

            if effect.done() {
                self.current += 1;
            }
        }

        remaining
    }

    fn done(&self) -> bool {
        self.current >= self.effects.len()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        None
    }

    fn set_area(&mut self, area: Rect) {
        self.effects.iter_mut().for_each(|e| e.set_area(area));
    }

    fn filter(&mut self, filter: CellFilter) {
        self.effects.iter_mut().for_each(|e| e.filter(filter.clone()));
    }

    fn reverse(&mut self) {
        self.effects.iter_mut().for_each(Effect::reverse)
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> { None }

    fn timer(&self) -> Option<EffectTimer> {
        let duration: Duration = self.effects.iter()
            .map(|fx| fx.timer())
            .filter(|t| t.is_some())
            .map(|t| t.unwrap().duration())
            .sum();

        if duration.is_zero() {
            None
        } else {
            Some(EffectTimer::new(duration, Linear))
        }
    }

    fn cell_filter(&self) -> Option<CellFilter> { None }

    fn reset(&mut self) {
        self.current = 0;
        self.effects.iter_mut().for_each(Effect::reset)
    }

    fn set_color_space(&mut self, color_space: ColorSpace) {
        self.effects.iter_mut().for_each(|e| e.set_color_space(color_space));
    }

    fn as_effect_span(&self, offset: Duration) -> EffectSpan {
        let mut acc = Duration::ZERO;
        let children = self.effects.iter()
            .map(|e| {
                let span = e.as_effect_span(offset + acc);
                acc += e.timer().map(|t| t.duration()).unwrap_or_default();
                span
            })
            .collect();

        EffectSpan::new(self, offset, children)
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        to_dsl(self.name(), &self.effects)
    }
}

#[cfg(feature = "dsl")]
fn to_dsl(
    name: &'static str,
    effects: &[Effect]
) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
    use crate::dsl::EffectExpression;
    let effects = effects.iter()
        .map(|e| e.to_dsl())
        .map(|dsl| dsl.map(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    EffectExpression::parse(&format!("{name}(&[{}])", effects.join(", ")))
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Margin;
    use ratatui::style::Color;
    use crate::fx::fade_to_fg;
    use crate::ShaderExt;
    use super::*;

    #[test]
    fn test_cell_filter_propagation() {
        let fx = fade_to_fg(Color::Black, 1);

        let mut effect = SequentialEffect::new(vec![
            fx.clone().with_filter(CellFilter::All),
            fx.clone().with_filter(CellFilter::Inner(Margin::new(1, 1))),
            fx.clone(),
        ]);

        // same effect as calling Effect::filter
        effect.propagate_filter(CellFilter::Text);

        assert_eq!(
            effect.effects[0].cell_filter().unwrap(),
            CellFilter::All
        );
        assert_eq!(
            effect.effects[1].cell_filter().unwrap(),
            CellFilter::Inner(Margin::new(1, 1))
        );
        assert_eq!(
            effect.effects[2].cell_filter().unwrap(),
            CellFilter::Text
        );
        assert_eq!(effect.done(), false);
    }
}

#[cfg(test)]
#[cfg(feature = "dsl")]
mod dsl_tests {
    use indoc::indoc;
    use crate::{fx, Shader};

    #[test]
    fn parallel() {
        let dsl = fx::parallel(&[fx::consume_tick(), fx::consume_tick()])
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, indoc! {
            "fx::parallel(&[fx::consume_tick(), fx::consume_tick()])"
        });
    }

    #[test]
    fn sequence() {
        let dsl = fx::sequence(&[fx::consume_tick(), fx::consume_tick()])
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, indoc! {
            "fx::sequence(&[fx::consume_tick(), fx::consume_tick()])"
        });
    }
}