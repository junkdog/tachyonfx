use std::fmt;
use std::fmt::Formatter;
use crate::fx::{consume_tick, dissolve, never_complete, ping_pong, repeating};
use crate::dsl::arguments::Arguments;
use crate::dsl::environment::DslEnv;
use crate::dsl::DslError;
use crate::{fx, Effect};
use crate::dsl::expressions::Expr;
use crate::dsl::parsers::parse_expr;


struct Interpreter {
    name: &'static str,
    eval: Box<dyn Fn(&mut Arguments) -> Result<Effect, DslError>>,
}

#[derive(Debug)]
pub struct EffectDsl {
    compilers: Vec<Interpreter>,
}

impl Interpreter {
    fn new(
        name: &'static str,
        eval: impl Fn(&mut Arguments) -> Result<Effect, DslError> + 'static
    ) -> Self {
        Self {
            name,
            eval: Box::new(eval),
        }
    }
}

impl EffectDsl {
    pub fn new() -> Self {
        register_default_interpreters(Self {
            compilers: Vec::new(),
        })
    }

    pub fn register(
        self,
        name: &'static str,
        compiler: impl Fn(&mut Arguments) -> Result<Effect, DslError> + 'static
    ) -> Self {
        let mut this = self;
        this.compilers.push(Interpreter::new(name, compiler));
        this
    }

    pub fn interpreter(&self) -> DslInterpreter {
        DslInterpreter {
            dsl: self,
            environment: DslEnv::new(),
        }
    }

    pub(super) fn eval(
        &self,
        env: &DslEnv,
        input: Expr
    ) -> Result<Effect, DslError> {
        match input {
            Expr::Fx { name, arguments } => self.compilers
                .iter()
                .find(|d| d.name == name)
                .ok_or(DslError::UnknownEffect { name })
                .and_then(|d| {
                    let mut args = Arguments::new(arguments.into(), self, env);
                    let effect = (d.eval)(&mut args);

                    match () {
                        _ if effect.is_err() => effect,
                        // todo: check on each interpreter if there are any remaining arguments
                        _ if !args.args().is_empty() => Err(DslError::TooManyArguments {
                            expected: args.original_arg_count() - args.args().len(),
                            actual: args.original_arg_count(),
                        }),
                        _ => effect,
                    }
                }),
            Expr::Sequence(exprs) => {
                let mut args = Arguments::new(exprs.into(), self, env);
                let effects = (0..args.args_count())
                    .map(|_| args.effect())
                    .collect::<Result<Vec<Effect>, DslError>>()?;

                Ok(fx::sequence(&effects))
            },
            Expr::Parallel(exprs) => {
                let mut args = Arguments::new(exprs.into(), self, env);
                let effects = (0..args.args_count())
                    .map(|_| args.effect())
                    .collect::<Result<Vec<Effect>, DslError>>()?;

                Ok(fx::parallel(&effects))
            },
            _ => Err(DslError::InvalidExpression {
                expected: "effect",
                actual: input.type_name(),
            }),
        }
    }
}

pub struct DslInterpreter<'ctx> {
    dsl: &'ctx EffectDsl,
    environment: DslEnv,
}

impl DslInterpreter<'_> {
    pub fn bind<K, T>(mut self, name: K, value: T) -> Self
    where
        K: Into<String>,
        T: 'static,
    {
        self.environment = self.environment.bind(name, value);
        self
    }

    pub fn eval(self, input: &str) -> Result<Effect, DslError> {
        parse_expr(input)
            .ok_or_else(|| DslError::ParseError(input.to_string()))
            .and_then(|expr| self.dsl.eval(&self.environment, expr))
    }
}

fn register_default_interpreters(effect_dsl: EffectDsl) -> EffectDsl {
    effect_dsl
        .register("coalesce",       interpreters::coalesce)
        .register("coalesce_from",  interpreters::coalesce_from)
        .register("consume_tick",   |_args| consume_tick().into())
        .register("delay",          interpreters::delay)
        .register("dissolve",       |args| dissolve(args.effect_timer()?).into())
        .register("dissolve_to",    interpreters::dissolve_to)
        .register("fade_from",      interpreters::fade_from)
        .register("fade_from_fg",   interpreters::fade_from_fg)
        .register("fade_to",        interpreters::fade_to)
        .register("fade_to_fg",     interpreters::fade_to_fg)
        .register("never_complete", |args| never_complete(args.effect()?).into())
        .register("ping_pong",      |args| ping_pong(args.effect()?).into())
        .register("prolong_end",    interpreters::prolong_end)
        .register("prolong_start",  interpreters::prolong_start)
        .register("repeat",         interpreters::repeat)
        .register("sleep",          interpreters::sleep)
        .register("repeating",      |args| repeating(args.effect()?).into())
        .register("slide_in",       interpreters::slide_in)
        .register("slide_out",      interpreters::slide_out)
        .register("sweep_in",       interpreters::sweep_in)
        .register("sweep_out",      interpreters::sweep_out)
        .register("with_duration",  interpreters::with_duration)
        .register(
            "timed_never_complete",
            interpreters::timed_never_complete
        )
}

impl From<Effect> for Result<Effect, DslError> {
    fn from(effect: Effect) -> Self {
        Ok(effect)
    }
}

mod interpreters {
    use crate::dsl::dsl::Arguments;
    use crate::dsl::DslError;
    use crate::{fx, Effect};

    pub(super) fn coalesce(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::coalesce(args.effect_timer()?).into()
    }

    pub(super) fn coalesce_from(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::coalesce_from(
            args.style()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn fade_to_fg(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::fade_to_fg(
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn fade_from_fg(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::fade_from_fg(
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn fade_to(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::fade_to(
            args.color()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn dissolve_to(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::dissolve_to(
            args.style()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn fade_from(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::fade_from(
            args.color()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn sweep_out(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::sweep_out(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn sleep(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::sleep(args.effect_timer()?).into()
    }

    pub(super) fn delay(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::delay(args.effect_timer()?, args.effect()?).into()
    }

    pub(super) fn prolong_start(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::prolong_start(args.effect_timer()?, args.effect()?).into()
    }

    pub(super) fn prolong_end(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::prolong_end(args.effect_timer()?, args.effect()?).into()
    }
    
    pub(super) fn repeat(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::repeat(args.effect()?, args.repeat_mode()?).into()
    }

    pub(super) fn sweep_in(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::sweep_in(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn slide_in(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::slide_in(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn slide_out(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::slide_out(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn with_duration(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::with_duration(
            args.duration()?,
            args.effect()?
        ).into()
    }

    pub(super) fn timed_never_complete(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::timed_never_complete(args.duration()?, args.effect()?).into()
    }
}

impl fmt::Debug for Interpreter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Interpreter")
            .field("name", &self.name)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use super::*;
    use crate::Interpolation::QuadOut;
    use crate::{fx, Duration, EffectTimer, Interpolation, Motion, Shader};
    use ratatui::style::{Color, Style};
    use Interpolation::Linear;
    use crate::dsl::dsl::interpreters::sweep_in;
    use crate::dsl::expressions::Value;
    use crate::fx::{parallel, sequence, sleep, RepeatMode};

    #[test]
    fn happy_path_no_bound_vars() {
        let input = r#"fx::sweep_in(
                Motion::LeftToRight,
                10,
                0,
                Color::from_u32(0x1d2021),
                (Duration::from_millis(1000), QuadOut)
            )"#;

        let dsl = EffectDsl::new();
        let effect = dsl.interpreter().eval(input)
            .expect("effect to be compiled");

        assert_eq!(effect.name(), "sweep_in");
        assert_eq!(effect.timer(), Some(EffectTimer::from_ms(1000, QuadOut)));
    }

    #[test]
    fn happy_path_with_bound_vars() {
        let expected = fx::sweep_in(
            Motion::LeftToRight,
            10,
            0,
            Color::from_u32(0x1d2021),
            EffectTimer::from_ms(1000, QuadOut)
        );

        let input = r#"fx::sweep_in(
                motion,
                10,
                0,
                c,
                (1000, QuadOut)
            )"#;


        let dsl = EffectDsl::new();
        let effect = dsl.interpreter()
            .bind("motion", Motion::LeftToRight)
            .bind("c", Color::from_u32(0x1d2021))
            .eval(input)
            .expect("effect to be compiled");


        assert_eq!(effect.name(), "sweep_in");
        assert_eq!(format!("{effect:?}"), format!("{expected:?}"));
    }

    #[test]
    fn sequence_and_parallel() {
        let expected = fx::sequence(&[
            fx::parallel(&[fx::consume_tick(), fx::consume_tick()]),
            fx::sleep(100)
        ]);

        let input = r#"fx::sequence(&[
            fx::parallel(&[fx::consume_tick(), fx::consume_tick()]),
            fx::sleep(100)
        ])"#;

        let dsl = EffectDsl::new();
        let effect = dsl.interpreter()
            .eval(input)
            .expect("effect to be compiled");

        assert_eq!(format!("{effect:?}"), format!("{expected:?}"));
    }

    #[test]
    fn error_unknown_effect() {
        let input = r#"fx::nonexistent()"#;
        let ctx = EffectDsl::new();
        let err = ctx.interpreter().eval(input).unwrap_err();
        assert!(matches!(err, DslError::UnknownEffect { .. }));
    }

    #[test]
    fn error_invalid_argument() {
        let input = r#"fx::sweep_in("wrong", 10, 0, Color::from_u32(0x1d2021), 1000)"#;
        let ctx = EffectDsl::new();
        let err = ctx.interpreter().eval(input).unwrap_err();
        assert!(matches!(err, DslError::WrongArgumentType {
            position: 0,
            expected: "motion"
        }), "{:?}", err);
    }

    #[test]
    fn too_many_arguments() {
        let input = r#"fx::sweep_in(
                Motion::LeftToRight,
                10,
                0,
                Color::from_u32(0x1d2021),
                (1000, QuadOut),
                "extra"
            )"#;

        let ctx = EffectDsl::new();
        let err = ctx.interpreter().eval(input).unwrap_err();
        assert!(matches!(err, DslError::TooManyArguments { .. }), "{:?}", err);
    }

    #[test]
    fn test_coalesce_interpreter() {
        let dsl = EffectDsl::new();
        let env = DslEnv::new();
        let exprs = vec![
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))
        ];
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::coalesce(&mut args).unwrap();
        assert_eq!(effect.name(), "coalesce");
        assert_eq!(effect.timer(), Some(EffectTimer::from_ms(500, Linear).reversed()));
    }

    #[test]
    fn test_coalesce_from_interpreter() {
        let dsl = EffectDsl::new();
        let style = Style::default().fg(Color::Red);
        let exprs = vec![
            Expr::Literal(Value::Style(style)),
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::coalesce_from(&mut args).unwrap();
        assert_eq!(effect.name(), "coalesce_from");
    }

    #[test]
    fn test_fade_to_fg_interpreter() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::Color(Color::Red)),
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::fade_to_fg(&mut args).unwrap();
        assert_eq!(effect.name(), "fade_to");
    }

    #[test]
    fn test_fade_from_fg_interpreter() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::Color(Color::Red)),
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::fade_from_fg(&mut args).unwrap();
        assert_eq!(effect.name(), "fade_from");
    }

    #[test]
    fn test_fade_to_interpreter() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::Color(Color::Red)),
            Expr::Literal(Value::Color(Color::Blue)),
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::fade_to(&mut args).unwrap();
        assert_eq!(effect.name(), "fade_to");
    }

    #[test]
    fn test_dissolve_to_interpreter() {
        let dsl = EffectDsl::new();
        let style = Style::default().fg(Color::Red);
        let exprs = vec![
            Expr::Literal(Value::Style(style)),
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::dissolve_to(&mut args).unwrap();
        assert_eq!(effect.name(), "dissolve_to");
    }

    #[test]
    fn test_fade_from_interpreter() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::Color(Color::Red)),
            Expr::Literal(Value::Color(Color::Blue)),
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::fade_from(&mut args).unwrap();
        assert_eq!(effect.name(), "fade_from");
    }

    #[test]
    fn test_sweep_out_interpreter() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::Motion(Motion::LeftToRight)),
            Expr::Literal(Value::U32(10)),
            Expr::Literal(Value::U32(0)),
            Expr::Literal(Value::Color(Color::Red)),
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::sweep_out(&mut args).unwrap();
        assert_eq!(effect.name(), "sweep_out");
    }

    #[test]
    fn test_repeat_interpreter() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Fx {
                name: "dissolve".to_string(),
                arguments: vec![Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))]
            },
            Expr::Literal(Value::RepeatMode(RepeatMode::Times(3)))
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::repeat(&mut args).unwrap();
        assert_eq!(effect.name(), "repeat");
    }

    #[test]
    fn test_sleep_interpreter() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::sleep(&mut args).unwrap();
        assert_eq!(effect.name(), "sleep");
    }
    
    #[test]
    fn test_sweep_in_interpreter() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::Motion(Motion::LeftToRight)),
            Expr::Literal(Value::U32(10)),
            Expr::Literal(Value::U32(0)),
            Expr::Literal(Value::Color(Color::Red)),
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::sweep_in(&mut args).unwrap();
        assert_eq!(effect.name(), "sweep_in");
    }

    #[test]
    fn test_slide_in_interpreter() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::Motion(Motion::LeftToRight)),
            Expr::Literal(Value::U32(10)),
            Expr::Literal(Value::U32(0)),
            Expr::Literal(Value::Color(Color::Red)),
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::slide_in(&mut args).unwrap();
        assert_eq!(effect.name(), "slide_in");
    }

    #[test]
    fn test_slide_out_interpreter() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::Motion(Motion::LeftToRight)),
            Expr::Literal(Value::U32(10)),
            Expr::Literal(Value::U32(0)),
            Expr::Literal(Value::Color(Color::Red)),
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::slide_out(&mut args).unwrap();
        assert_eq!(effect.name(), "slide_out");
    }

    #[test]
    fn test_with_duration_interpreter() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::Duration(Duration::from_millis(1000))),
            Expr::Fx {
                name: "dissolve".to_string(),
                arguments: vec![Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))]
            }
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::with_duration(&mut args).unwrap();
        assert_eq!(effect.name(), "with_duration");
    }

    #[test]
    fn test_timed_never_complete_interpreter() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::Duration(Duration::from_millis(1000))),
            Expr::Fx {
                name: "dissolve".to_string(),
                arguments: vec![Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))]
            }
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::timed_never_complete(&mut args).unwrap();
        assert_eq!(effect.name(), "with_duration");
    }

    #[test]
    fn test_delay_interpreter() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear))),
            Expr::Fx {
                name: "dissolve".to_string(),
                arguments: vec![Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))]
            }
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::delay(&mut args).unwrap();
        assert_eq!(effect.name(), "sequence");
    }

    #[test]
    fn test_prolong_start_interpreter() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear))),
            Expr::Fx {
                name: "dissolve".to_string(),
                arguments: vec![Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))]
            }
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::prolong_start(&mut args).unwrap();
        assert_eq!(effect.name(), "prolong_start");
    }

    #[test]
    fn test_prolong_end_interpreter() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear))),
            Expr::Fx {
                name: "dissolve".to_string(),
                arguments: vec![Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))]
            }
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        let effect = interpreters::prolong_end(&mut args).unwrap();
        assert_eq!(effect.name(), "prolong_end");
    }

    // Error cases
    #[test]
    fn test_interpreter_missing_arguments() {
        let dsl = EffectDsl::new();
        let exprs = vec![];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        assert!(interpreters::fade_to_fg(&mut args).is_err());
    }

    #[test]
    fn test_interpreter_wrong_argument_type() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::String("wrong".to_string())),
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        assert!(interpreters::fade_to_fg(&mut args).is_err());
    }
}