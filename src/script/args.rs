use std::any::Any;
use std::collections::{BTreeMap, VecDeque};
use ratatui::layout::{Margin, Rect};
use ratatui::prelude::{Color, Style};
use crate::{Duration, Effect, EffectTimer, Motion};
use crate::script::env::ScriptEnv;
use crate::script::parser::Expr;

pub struct InputArgs<'a> {
    args: VecDeque<Expr>,
    vars: &'a ScriptEnv,
}

impl<'a> InputArgs<'a> {
    pub(super) fn new(
        args: VecDeque<Expr>,
        vars: &'a ScriptEnv
    ) -> Self {
        Self { args, vars }
    }

    pub(super) fn args(&self) -> &VecDeque<Expr> {
        &self.args
    }

    pub fn duration(&mut self) -> Option<Duration> {
        match self.next()? {
            Expr::Duration(d) => Some(d),
            Expr::U32(ms)     => Some(Duration::from_millis(ms as _)),
            Expr::Var(name)   => self.vars.get(name).cloned(),
            _                 => None,
        }
    }

    pub fn effect_timer(&mut self) -> Option<EffectTimer> {
        match self.next()? {
            Expr::Timer(t)  => Some(t),
            Expr::U32(ms)   => Some(ms.into()),
            Expr::Var(name) => self.vars.get(name).cloned(),
            _               => None,
        }
    }

    pub fn read_u16(&mut self) -> Option<u16> {
        self.read_u32().map(|u| u as _)
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        match self.next()? {
            Expr::U32(u)    => Some(u),
            Expr::Var(name) => self.vars.get(name).cloned(),
            _               => None,
        }
    }

    pub fn read_f32(&mut self) -> Option<f32> {
        match self.next()? {
            Expr::F32(f)    => Some(f),
            Expr::Var(name) => self.vars.get(name).cloned(),
            _               => None,
        }
    }

    pub fn string(&mut self) -> Option<String> {
        match self.next()? {
            Expr::String(s) => Some(s),
            Expr::Var(name) => self.vars.get(name).cloned(),
            _               => None,
        }
    }

    pub fn effect(&mut self) -> Option<Effect> {
        match self.next()? {
            Expr::Fx { name, parameters } => {
                // todo: recursive deserialization?
                None
            },
            Expr::Var(name) => self.vars.get(name).cloned(),
            _ => None,
        }
    }

    pub fn color(&mut self) -> Option<Color> {
        match self.next()? {
            Expr::Color(c)  => Some(c),
            Expr::Var(name) => self.vars.get(name).cloned(),
            _               => None,
        }
    }

    pub fn style(&mut self) -> Option<Style> {
        match self.next()? {
            Expr::Style(s)  => Some(s),
            Expr::Var(name) => self.vars.get(name).cloned(),
            _               => None,
        }
    }

    pub fn motion(&mut self) -> Option<Motion> {
        match self.next()? {
            Expr::Motion(m) => Some(m),
            Expr::Var(name) => self.vars.get(name).cloned(),
            _               => None,
        }
    }

    pub fn margin(&mut self) -> Option<Margin> {
        match self.next()? {
            Expr::Margin(m) => Some(m),
            Expr::Var(name) => self.vars.get(name).cloned(),
            _               => None,
        }
    }

    pub fn rect(&mut self) -> Option<Rect> {
        match self.next()? {
            Expr::Rect(r)   => Some(r),
            Expr::Var(name) => self.vars.get(name).cloned(),
            _               => None,
        }
    }

    fn next(&mut self) -> Option<Expr> {
        self.args.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, VecDeque};
    use ratatui::layout::{Margin, Rect};
    use ratatui::prelude::{Color, Style};
    use crate::{Duration, EffectTimer, Interpolation, Motion};
    use crate::script::args::InputArgs;
    use crate::script::env::ScriptEnv;
    use crate::script::parser::Expr;

    fn empty_env() -> ScriptEnv {
        ScriptEnv::new()
    }

    #[test]
    fn test_duration_parsing() {
        let binding = empty_env();
        let mut args = InputArgs::new(
            vec![
                Expr::Duration(Duration::from_millis(500)),
                Expr::U32(1000),
            ].into(),
            &binding
        );

        assert_eq!(args.duration(), Some(Duration::from_millis(500)));
        assert_eq!(args.duration(), Some(Duration::from_millis(1000)));
        assert_eq!(args.duration(), None);
    }

    #[test]
    fn test_effect_timer_parsing() {
        let binding = empty_env();
        let mut args = InputArgs::new(
            vec![
                Expr::Timer(EffectTimer::from_ms(500, Interpolation::Linear)),
                Expr::U32(1000),
            ].into(),
            &binding
        );

        assert_eq!(args.effect_timer(), Some(EffectTimer::from_ms(500, Interpolation::Linear)));
        assert_eq!(args.effect_timer(), Some(EffectTimer::from_ms(1000, Interpolation::Linear)));
        assert_eq!(args.effect_timer(), None);
    }

    #[test]
    fn test_numeric_parsing() {
        let binding = empty_env();
        let mut args = InputArgs::new(
            vec![
                Expr::U32(42),
                Expr::F32(3.14),
            ].into(),
            &binding
        );

        assert_eq!(args.read_u32(), Some(42));
        assert_eq!(args.read_f32(), Some(3.14));
        assert_eq!(args.read_u32(), None);
    }

    #[test]
    fn test_string_parsing() {
        let binding = empty_env();
        let mut args = InputArgs::new(
            vec![
                Expr::String("hello".to_string()),
                Expr::U32(42), // Wrong type
                Expr::String("world".to_string()),
            ].into(),
            &binding
        );

        assert_eq!(args.string(), Some("hello".to_string()));
        assert_eq!(args.string(), None); // U32 should be skipped
        assert_eq!(args.string(), Some("world".to_string()));
    }

    #[test]
    fn test_color_parsing() {
        let binding = empty_env();
        let mut args = InputArgs::new(
            vec![
                Expr::Color(Color::Red),
                Expr::Color(Color::Blue),
            ].into(),
            &binding
        );

        assert_eq!(args.color(), Some(Color::Red));
        assert_eq!(args.color(), Some(Color::Blue));
        assert_eq!(args.color(), None);
    }

    #[test]
    fn test_style_parsing() {
        let style = Style::default().fg(Color::Red);
        let binding = empty_env();
        let mut args = InputArgs::new(
            vec![
                Expr::Style(style),
            ].into(),
            &binding
        );

        assert_eq!(args.style(), Some(style));
        assert_eq!(args.style(), None);
    }

    #[test]
    fn test_motion_parsing() {
        let binding = empty_env();
        let mut args = InputArgs::new(
            vec![
                Expr::Motion(Motion::LeftToRight),
                Expr::Motion(Motion::UpToDown),
            ].into(),
            &binding
        );

        assert_eq!(args.motion(), Some(Motion::LeftToRight));
        assert_eq!(args.motion(), Some(Motion::UpToDown));
        assert_eq!(args.motion(), None);
    }

    #[test]
    fn test_margin_parsing() {
        let margin = Margin::new(10, 20);
        let binding = empty_env();
        let mut args = InputArgs::new(
            vec![
                Expr::Margin(margin),
            ].into(),
            &binding
        );

        assert_eq!(args.margin(), Some(margin));
        assert_eq!(args.margin(), None);
    }

    #[test]
    fn test_rect_parsing() {
        let rect = Rect::new(0, 0, 100, 100);
        let binding = empty_env();
        let mut args = InputArgs::new(
            vec![
                Expr::Rect(rect),
            ].into(),
            &binding
        );

        assert_eq!(args.rect(), Some(rect));
        assert_eq!(args.rect(), None);
    }

    #[test]
    fn test_effect_parsing() {
        let binding = empty_env();
        let mut args = InputArgs::new(
            vec![
                Expr::Fx {
                    name: "test".to_string(),
                    parameters: vec![Expr::U32(500)]
                },
            ].into(),
            &binding
        );

        // Currently returns None as noted in the TODO
        assert!(args.effect().is_none());
    }

    #[test]
    fn test_mixed_arguments() {
        let binding = empty_env();
        let mut args = InputArgs::new(
            vec![
                Expr::U32(500),
                Expr::Motion(Motion::LeftToRight),
                Expr::Color(Color::Blue),
                Expr::Timer(EffectTimer::from_ms(1000, Interpolation::Linear)),
            ].into(),
            &binding
        );

        assert_eq!(args.read_u32(), Some(500));
        assert_eq!(args.motion(), Some(Motion::LeftToRight));
        assert_eq!(args.color(), Some(Color::Blue));
        assert_eq!(args.effect_timer(), Some(EffectTimer::from_ms(1000, Interpolation::Linear)));
        assert_eq!(args.read_u32(), None);
    }

    #[test]
    fn test_u16_conversion() {
        let binding = empty_env();
        let mut args = InputArgs::new(
            vec![
                Expr::U32(65535), // Max u16
                Expr::U32(65536), // Too large for u16
            ].into(),
            &binding
        );

        assert_eq!(args.read_u16(), Some(65535));
        assert_eq!(args.read_u16(), Some(0)); // Truncated
    }

    #[test]
    fn test_empty_args() {
        let binding = empty_env();
        let mut args = InputArgs::new(VecDeque::new(), &binding);

        assert_eq!(args.duration(), None);
        assert_eq!(args.effect_timer(), None);
        assert_eq!(args.read_u32(), None);
        assert_eq!(args.read_f32(), None);
        assert_eq!(args.string(), None);
        assert_eq!(args.color(), None);
        assert_eq!(args.style(), None);
        assert_eq!(args.motion(), None);
        assert_eq!(args.margin(), None);
        assert_eq!(args.rect(), None);
    }
}