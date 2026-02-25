use alloc::boxed::Box;

use ratatui_core::{buffer::Buffer, layout::Rect};

use crate::{effect::Effect, shader::Shader, CellFilter, ColorSpace, Duration, EffectTimer};

/// A shader that wraps another effect and ensures it runs exactly once before reporting
/// completion.
///
/// This shader is particularly useful for zero-duration effects that need to be included
/// in sequences or parallel compositions. Without this wrapper, zero-duration effects
/// would be skipped entirely in such compositions.
///
/// The wrapped effect will execute once, regardless of its completion status, after which
/// the RunOnce shader will report completion.
#[derive(Clone, Debug)]
pub(crate) struct RunOnce {
    effect: Effect,
    has_run: bool,
}

impl RunOnce {
    pub(crate) fn new(effect: Effect) -> Self {
        Self { effect, has_run: false }
    }
}

impl Shader for RunOnce {
    fn name(&self) -> &'static str {
        "run_once"
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        if !self.has_run {
            self.has_run = true;
            self.effect.process(duration, buf, area)
        } else {
            None
        }
    }

    fn done(&self) -> bool {
        self.has_run
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

    fn reverse(&mut self) {
        self.effect.reverse();
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        None
    }

    fn cell_filter(&self) -> Option<&CellFilter> {
        self.effect.cell_filter()
    }

    fn reset(&mut self) {
        self.has_run = false;
        self.effect.reset();
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
        EffectExpression::parse(&format!("fx::run_once({nested})"))
    }
}

#[cfg(test)]
mod tests {
    use crate::fx;

    #[test]
    fn test_run_once_execution() {
        let mut effect = fx::run_once(fx::consume_tick());
        let mut buf =
            ratatui_core::buffer::Buffer::empty(ratatui_core::layout::Rect::new(0, 0, 10, 10));
        let area = ratatui_core::layout::Rect::new(0, 0, 10, 10);

        // Should not be done initially
        assert!(!effect.done());

        // After first process, should be done
        effect.process(crate::Duration::from_millis(16), &mut buf, area);
        assert!(effect.done());

        // Subsequent processes should not change state
        effect.process(crate::Duration::from_millis(16), &mut buf, area);
        assert!(effect.done());
    }

    #[test]
    fn test_run_once_reset() {
        let mut effect = fx::run_once(fx::consume_tick());
        let mut buf =
            ratatui_core::buffer::Buffer::empty(ratatui_core::layout::Rect::new(0, 0, 10, 10));
        let area = ratatui_core::layout::Rect::new(0, 0, 10, 10);

        // Run once
        effect.process(crate::Duration::from_millis(16), &mut buf, area);
        assert!(effect.done());

        // Reset and verify it can run again
        effect.reset();
        assert!(!effect.done());

        effect.process(crate::Duration::from_millis(16), &mut buf, area);
        assert!(effect.done());
    }

    #[cfg(feature = "dsl")]
    #[test]
    fn test_run_once_dsl() {
        let dsl = fx::run_once(fx::consume_tick())
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, "fx::run_once(fx::consume_tick())");
    }

    #[cfg(feature = "dsl")]
    #[test]
    fn test_run_once_dsl_roundtrip() {
        use crate::dsl::EffectDsl;

        let dsl = EffectDsl::new();
        let result = dsl
            .compiler()
            .compile("fx::run_once(fx::consume_tick())");

        assert!(result.is_ok());
        let effect = result.unwrap();
        assert_eq!(effect.name(), "run_once");
    }
}
