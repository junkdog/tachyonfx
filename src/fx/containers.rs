use alloc::{boxed::Box, vec::Vec};

use ratatui_core::{buffer::Buffer, layout::Rect};

use crate::{
    effect::Effect, shader::Shader, CellFilter, ColorSpace, Duration, EffectTimer,
    Interpolation::Linear,
};

#[derive(Default, Clone, Debug)]
pub(super) struct SequentialEffect {
    effects: Vec<Effect>,
    current: usize,
}

#[derive(Clone, Debug)]
pub(super) struct ParallelEffect {
    effects: Vec<Effect>,
    pending_offsets: Vec<Duration>,
}

impl SequentialEffect {
    pub fn new(effects: Vec<Effect>) -> Self {
        Self { effects, current: 0 }
    }
}

impl ParallelEffect {
    pub fn new(effects: Vec<Effect>) -> Self {
        Self { effects, pending_offsets: Vec::new() }
    }

    /// Computes right-alignment offsets so that shorter children are delayed
    /// to end at the same time as the longest child.
    fn compute_offsets(&self) -> Vec<Duration> {
        let t_max = self
            .effects
            .iter()
            .filter_map(Effect::timer)
            .map(|t| t.duration())
            .max()
            .unwrap_or(Duration::ZERO);

        self.effects
            .iter()
            .map(|fx| {
                fx.timer()
                    .map(|t| t.duration())
                    .and_then(|d| t_max.checked_sub(d))
                    .unwrap_or(Duration::ZERO)
            })
            .collect()
    }
}

impl Shader for ParallelEffect {
    fn name(&self) -> &'static str {
        "parallel"
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        let mut remaining = Some(duration);

        for i in 0..self.effects.len() {
            if !self.effects[i].running() {
                continue;
            }

            let is_reversed = !self.pending_offsets.is_empty();
            let child_duration = if is_reversed && self.pending_offsets[i] > Duration::ZERO {
                // consume offset time before forwarding to child
                let consumed = duration.min(self.pending_offsets[i]);
                self.pending_offsets[i] -= consumed;
                duration - consumed
            } else {
                duration
            };

            let effect_area = self.effects[i].area().unwrap_or(area);
            match self.effects[i].process(child_duration, buf, effect_area) {
                None => remaining = None,
                Some(d) if remaining.is_some() => {
                    remaining = Some(d.min(remaining.unwrap()));
                },
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
        self.effects
            .iter_mut()
            .for_each(|e| e.set_area(area));
    }

    fn filter(&mut self, filter: CellFilter) {
        self.effects
            .iter_mut()
            .for_each(|e| e.filter(filter.clone()));
    }

    fn reverse(&mut self) {
        self.effects.iter_mut().for_each(Effect::reverse);
        self.pending_offsets = if self.pending_offsets.is_empty() {
            self.compute_offsets()
        } else {
            Vec::new()
        };
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
    }

    fn timer(&self) -> Option<EffectTimer> {
        self.effects
            .iter()
            .filter_map(Effect::timer)
            .map(|t| t.duration())
            .max()
            .map(|d| EffectTimer::new(d, Linear))
    }

    fn cell_filter(&self) -> Option<&CellFilter> {
        None
    }

    fn reset(&mut self) {
        self.effects.iter_mut().for_each(Effect::reset);
        if !self.pending_offsets.is_empty() {
            self.pending_offsets = self.compute_offsets();
        }
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        to_dsl(self.name(), &self.effects)
    }

    fn set_color_space(&mut self, color_space: ColorSpace) {
        self.effects
            .iter_mut()
            .for_each(|e| e.set_color_space(color_space));
    }
}

impl Shader for SequentialEffect {
    fn name(&self) -> &'static str {
        "sequence"
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
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
        self.effects
            .iter_mut()
            .for_each(|e| e.set_area(area));
    }

    fn filter(&mut self, filter: CellFilter) {
        self.effects
            .iter_mut()
            .for_each(|e| e.filter(filter.clone()));
    }

    fn reverse(&mut self) {
        self.effects.iter_mut().for_each(Effect::reverse);
        self.effects.reverse();
        self.current = 0;
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
    }

    fn timer(&self) -> Option<EffectTimer> {
        let duration: Duration = self
            .effects
            .iter()
            .map(Effect::timer)
            .filter(Option::is_some)
            .map(|t| t.unwrap().duration())
            .sum();

        if duration.is_zero() {
            None
        } else {
            Some(EffectTimer::new(duration, Linear))
        }
    }

    fn cell_filter(&self) -> Option<&CellFilter> {
        None
    }

    fn reset(&mut self) {
        self.current = 0;
        self.effects.iter_mut().for_each(Effect::reset);
    }

    fn set_color_space(&mut self, color_space: ColorSpace) {
        self.effects
            .iter_mut()
            .for_each(|e| e.set_color_space(color_space));
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        to_dsl(self.name(), &self.effects)
    }
}

#[cfg(feature = "dsl")]
fn to_dsl(
    name: &'static str,
    effects: &[Effect],
) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
    use crate::dsl::EffectExpression;
    let effects = effects
        .iter()
        .map(Effect::to_dsl)
        .map(|dsl| dsl.map(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    EffectExpression::parse(&format!("{name}(&[{}])", effects.join(", ")))
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use ratatui_core::{layout::Margin, style::Color};

    use super::*;
    use crate::{fx::fade_to_fg, ShaderExt};

    #[test]
    fn test_cell_filter_propagation() {
        let fx = fade_to_fg(Color::Black, 1);

        let mut effect = SequentialEffect::new(vec![
            fx.clone().with_filter(CellFilter::All),
            fx.clone()
                .with_filter(CellFilter::Inner(Margin::new(1, 1))),
            fx,
        ]);

        // same effect as calling Effect::filter
        effect.propagate_filter(CellFilter::Text);

        assert_eq!(*effect.effects[0].cell_filter().unwrap(), CellFilter::All);
        assert_eq!(
            *effect.effects[1].cell_filter().unwrap(),
            CellFilter::Inner(Margin::new(1, 1))
        );
        assert_eq!(*effect.effects[2].cell_filter().unwrap(), CellFilter::Text);
        assert!(!effect.done());
    }
}

#[cfg(test)]
#[cfg(feature = "dsl")]
mod dsl_tests {
    use indoc::indoc;

    use crate::fx;

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
