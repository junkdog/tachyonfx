use crate::dsl::environment::DslEnv;
use crate::dsl::dsl::EffectDsl;
use crate::dsl::DslError;
use crate::{Duration, Effect, EffectTimer, Interpolation, Motion};
use ratatui::layout::{Margin, Rect};
use ratatui::prelude::{Color, Style};
use std::collections::VecDeque;
use crate::dsl::expressions::{Expr, FnCall, Value};
use crate::fx::RepeatMode;

#[derive(Debug)]
pub struct Arguments<'a> {
    args: VecDeque<Expr>,
    vars: &'a DslEnv,
    context: &'a EffectDsl,
    initial_arg_count: usize,
}

impl<'a> Arguments<'a> {
    pub(super) fn new(
        args: VecDeque<Expr>,
        context: &'a EffectDsl,
        vars: &'a DslEnv
    ) -> Self {
        let initial_arg_count = args.len();
        Self { args, vars, context, initial_arg_count }
    }

    pub(super) fn args(&self) -> &VecDeque<Expr> {
        &self.args
    }

    pub(super) fn args_count(&self) -> usize {
        self.args.len()
    }

    pub fn duration(&mut self) -> Result<Duration, DslError> {
        match self.next("duration")? {
            Expr::Call { function: FnCall::DurationFromMillis, args } => {
                let mut inner_args = Arguments::new(args.into(), self.context, self.vars);
                let ms = inner_args.read_u32()?;
                Ok(Duration::from_millis(ms as _))
            },
            Expr::Call { function: FnCall::DurationFromSeconds, args } => {
                let mut inner_args = Arguments::new(args.into(), self.context, self.vars);
                let seconds = inner_args.read_f32()?;
                Ok(Duration::from_secs_f32(seconds))
            },
            Expr::Literal(v)  => match v {
                Value::Duration(d) => Ok(d),
                Value::U32(ms)     => Ok(Duration::from_millis(ms as _)),
                _                  => self.wrong_type_error("duration"),
            },

            Expr::Var(name)   => self.bound_var(name),
            _                 => self.wrong_type_error("duration"),
        }
    }

    pub fn effect_timer(&mut self) -> Result<EffectTimer, DslError> {
        match self.next("timer")? {
            Expr::Call { function: FnCall::EffectTimerFromMs, args } => {
                let mut inner_args = Arguments::new(args.into(), self.context, self.vars);
                let ms = inner_args.read_u32()?;
                let interpolation = inner_args.interpolation()?;
                Ok(EffectTimer::from_ms(ms, interpolation))
            },
            Expr::Call { function: FnCall::EffectTimerNew, args } => {
                let mut inner_args = Arguments::new(args.into(), self.context, self.vars);
                let duration = inner_args.duration()?;
                let interpolation = inner_args.interpolation()?;
                Ok(EffectTimer::new(duration, interpolation))
            },
            Expr::Literal(Value::Timer(t)) => Ok(t),
            Expr::Literal(Value::U32(ms))  => Ok(ms.into()),
            Expr::Var(name)                => self.bound_var(name),
            _                              => self.wrong_type_error("timer"),
        }
    }

    pub fn interpolation(&mut self) -> Result<Interpolation, DslError> {
        match self.next("interpolation")? {
            Expr::Literal(Value::Interpolation(i)) => Ok(i),
            Expr::Var(name)                        => self.bound_var(name),
            _                                      => self.wrong_type_error("interpolation"),
        }
    }

    pub fn read_u16(&mut self) -> Result<u16, DslError> {
        u16::try_from(self.read_u32()?)
            .map_err(|_| DslError::CastOverflow {
                position: self.initial_arg_count - self.args.len() - 1, // -1 for the current argument
                from: "u32",
                to: "u16",
            })
    }

    pub fn read_u32(&mut self) -> Result<u32, DslError> {
        match self.next("u32")? {
            Expr::Literal(Value::U32(u)) => Ok(u),
            Expr::Var(name)              => self.bound_var(name),
            _                            => self.wrong_type_error("u32"),
        }
    }

    pub fn read_f32(&mut self) -> Result<f32, DslError> {
        match self.next("f32")? {
            Expr::Literal(Value::F32(f)) => Ok(f),
            Expr::Var(name)              => self.bound_var(name),
            _                            => self.wrong_type_error("f32"),
        }
    }

    pub fn string(&mut self) -> Result<String, DslError> {
        match self.next("string")? {
            Expr::Literal(Value::String(s)) => Ok(s),
            Expr::Var(name)                 => self.bound_var(name),
            _                               => self.wrong_type_error("string"),
        }
    }

    pub fn effect(&mut self) -> Result<Effect, DslError> {
        match self.next("effect")? {
            Expr::Fx { name, arguments } => self.compile_effect(Expr::Fx { name, arguments }),
            Expr::Sequence(effects)      => self.compile_effect(Expr::Sequence(effects)),
            Expr::Parallel(effects)      => self.compile_effect(Expr::Parallel(effects)),
            Expr::Var(name)              => self.bound_var(name),
            _                            => self.wrong_type_error("effect"),
        }
    }

    pub fn color(&mut self) -> Result<Color, DslError> {
        match self.next("color")? {
            Expr::Call { function: FnCall::ColorFromU32, args } => {
                let mut inner_args = Arguments::new(args.into(), self.context, self.vars);
                inner_args
                    .read_u32()
                    .map(Color::from_u32)
            }
            Expr::Literal(v)  => match v {
                Value::Color(c) => Ok(c),
                _               => self.wrong_type_error("color"),
            },

            Expr::Var(name) => self.bound_var(name),
            _               => self.wrong_type_error("color"),
        }
    }

    pub fn style(&mut self) -> Result<Style, DslError> {
        match self.next("style")? {
            Expr::Literal(v)  => match v {
                Value::Style(s) => Ok(s),
                _               => self.wrong_type_error("style"),
            },

            Expr::Var(name) => self.bound_var(name),
            _               => self.wrong_type_error("style"),
        }
    }

    pub fn motion(&mut self) -> Result<Motion, DslError> {
        match self.next("motion")? {
            Expr::Literal(v)  => match v {
                Value::Motion(m) => Ok(m),
                _                => self.wrong_type_error("motion"),
            },

            Expr::Var(name) => self.bound_var(name),
            _               => self.wrong_type_error("motion"),
        }
    }

    pub fn repeat_mode(&mut self) -> Result<RepeatMode, DslError> {
        match self.next("repeat_mode")? {
            Expr::Literal(v)  => match v {
                Value::RepeatMode(m) => Ok(m),
                _                    => self.wrong_type_error("repeat_mode"),
            },

            Expr::Var(name)     => self.bound_var(name),
            _                   => self.wrong_type_error("repeat_mode"),
        }
    }

    pub fn margin(&mut self) -> Result<Margin, DslError> {
        match self.next("margin")? {
            Expr::Literal(v)  => match v {
                Value::Margin(m) => Ok(m),
                _                => self.wrong_type_error("margin"),
            },
            Expr::Var(name) => self.bound_var(name),
            _               => self.wrong_type_error("margin"),
        }
    }

    pub fn rect(&mut self) -> Result<Rect, DslError> {
        match self.next("rect")? {
            Expr::Literal(v)  => match v {
                Value::Rect(r) => Ok(r),
                _              => self.wrong_type_error("rect"),
            },

            Expr::Var(name) => self.bound_var(name),
            _               => self.wrong_type_error("rect"),
        }
    }

    pub(super) fn original_arg_count(&self) -> usize {
        self.initial_arg_count
    }

    fn compile_effect(&self, expr: Expr) -> Result<Effect, DslError> {
        self.context.eval(self.vars, expr)
    }

    fn bound_var<T: Clone + 'static>(&self, name: String) -> Result<T, DslError> {
        self.vars.get(name).cloned()
    }

    fn next(&mut self, type_name: &'static str) -> Result<Expr, DslError> {
        self.args.pop_front()
            .ok_or(DslError::MissingArgument {
                position: self.initial_arg_count - self.args.len(),
                name: type_name,
            })
    }


    fn wrong_type_error<T>(&self, expected: &'static str) -> Result<T, DslError>  {
        Err(DslError::WrongArgumentType {
            position: self.initial_arg_count - self.args.len() - 1,
            expected,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::dsl::arguments::Arguments;
    use crate::dsl::environment::DslEnv;
    use crate::dsl::dsl::EffectDsl;
    use crate::dsl::DslError;
    use crate::{Duration, EffectTimer, Interpolation, Motion};
    use ratatui::layout::{Margin, Rect};
    use ratatui::prelude::{Color, Style};
    use std::collections::VecDeque;
    use crate::dsl::expressions::{Expr, Value};

    fn empty_env() -> DslEnv {
        DslEnv::new()
    }

    #[test]
    fn test_duration_parsing() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::Duration(Duration::from_millis(500))),
                Expr::Literal(Value::U32(1000)),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.duration(), Ok(Duration::from_millis(500)));
        assert_eq!(args.duration(), Ok(Duration::from_millis(1000)));
        assert_eq!(args.duration(), Err(DslError::MissingArgument {
            position: 2,
            name: "duration",
        }));
    }

    #[test]
    fn test_effect_timer_parsing() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Interpolation::Linear))),
                Expr::Literal(Value::U32(1000)),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.effect_timer(), Ok(EffectTimer::from_ms(500, Interpolation::Linear)));
        assert_eq!(args.effect_timer(), Ok(EffectTimer::from_ms(1000, Interpolation::Linear)));
        assert_eq!(args.effect_timer(), Err(DslError::MissingArgument {
            position: 2,
            name: "timer",
        }));
    }

    #[test]
    fn test_numeric_parsing() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::U32(42)),
                Expr::Literal(Value::F32(3.14)),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.read_u32(), Ok(42));
        assert_eq!(args.read_f32(), Ok(3.14));
        assert_eq!(args.read_u32(), Err(DslError::MissingArgument {
            position: 2,
            name: "u32",
        }));
    }

    #[test]
    fn test_string_parsing() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::String("hello".to_string())),
                Expr::Literal(Value::U32(42)), // Wrong type
                Expr::Literal(Value::String("world".to_string())),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.string(), Ok("hello".to_string()));
        assert_eq!(args.string(), Err(DslError::WrongArgumentType {
            position: 1,
            expected: "string",
        }));
        assert_eq!(args.string(), Ok("world".to_string()));
    }

    #[test]
    fn test_color_parsing() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::Color(Color::Red)),
                Expr::Literal(Value::Color(Color::Blue)),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.color(), Ok(Color::Red));
        assert_eq!(args.color(), Ok(Color::Blue));
        assert_eq!(args.color(), Err(DslError::MissingArgument {
            position: 2,
            name: "color",
        }));
    }

    #[test]
    fn test_style_parsing() {
        let context = EffectDsl::new();
        let style = Style::default().fg(Color::Red);
        let binding = empty_env();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::Style(style)),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.style(), Ok(style));
        assert_eq!(args.style(), Err(DslError::MissingArgument {
            position: 1,
            name: "style",
        }));
    }

    #[test]
    fn test_motion_parsing() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::Motion(Motion::LeftToRight)),
                Expr::Literal(Value::Motion(Motion::UpToDown)),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.motion(), Ok(Motion::LeftToRight));
        assert_eq!(args.motion(), Ok(Motion::UpToDown));
        assert_eq!(args.motion(), Err(DslError::MissingArgument {
            position: 2,
            name: "motion",
        }));
    }

    #[test]
    fn test_margin_parsing() {
        let margin = Margin::new(10, 20);
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::Margin(margin)),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.margin(), Ok(margin));
        assert_eq!(args.margin(), Err(DslError::MissingArgument {
            position: 1,
            name: "margin",
        }));
    }

    #[test]
    fn test_rect_parsing() {
        let rect = Rect::new(0, 0, 100, 100);
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::Rect(rect)),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.rect(), Ok(rect));
        assert_eq!(args.rect(), Err(DslError::MissingArgument {
            position: 1,
            name: "rect",
        }));
    }

    #[test]
    fn test_effect_parsing() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Fx {
                    name: "test".to_string(),
                    arguments: vec![Expr::Literal(Value::U32(500))]
                },
            ].into(),
            &context,
            &binding
        );

        let result = args.effect();
        assert!(result.is_err());
        assert_eq!(
            result.expect_err("expected error"),
            DslError::UnknownEffect {
                name: "test".to_string(),
            }
        );
    }

    #[test]
    fn test_mixed_arguments() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::U32(500)),
                Expr::Literal(Value::Motion(Motion::LeftToRight)),
                Expr::Literal(Value::Color(Color::Blue)),
                Expr::Literal(Value::Timer(EffectTimer::from_ms(1000, Interpolation::Linear))),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.read_u32(), Ok(500));
        assert_eq!(args.motion(), Ok(Motion::LeftToRight));
        assert_eq!(args.color(), Ok(Color::Blue));
        assert_eq!(args.effect_timer(), Ok(EffectTimer::from_ms(1000, Interpolation::Linear)));
        assert_eq!(args.read_u32(), Err(DslError::MissingArgument {
            position: 4,
            name: "u32",
        }));
    }

    #[test]
    fn test_u16_conversion() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::U32(65535)), // Max u16
                Expr::Literal(Value::U32(65536)), // Too large for u16
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.read_u16(), Ok(65535));
        assert_eq!(args.read_u16(), Err(DslError::CastOverflow {
            position: 1,
            from: "u32",
            to: "u16",
        })); // Truncated
    }

    #[test]
    fn test_empty_args() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(VecDeque::new(), &context, &binding);

        let missing = |idx, name| Err(DslError::MissingArgument {
            position: idx,
            name,
        });

        assert_eq!(args.duration(), missing(0, "duration"));
    }
}
