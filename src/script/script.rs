use crate::fx::{consume_tick, dissolve, never_complete, ping_pong, repeating};
use crate::script::args::{type_name_of, InputArgs};
use crate::script::env::ScriptEnv;
use crate::script::parser::{parse_expr, Expr};
use crate::script::ScriptError;
use crate::{Effect, EffectTimer};
use std::any::Any;

struct EffectCompiler {
    name: &'static str,
    compiler: Box<dyn Fn(&mut InputArgs) -> Result<Effect, ScriptError>>,
}



pub struct ScriptContext {
    compilers: Vec<EffectCompiler>,
}

impl EffectCompiler {
    fn new(
        name: &'static str,
        compiler: impl Fn(&mut InputArgs) -> Result<Effect, ScriptError> + 'static
    ) -> Self {
        Self {
            name,
            compiler: Box::new(compiler),
        }
    }
}

impl ScriptContext {
    pub fn new() -> Self {
        register_default_compilers(Self {
            compilers: Vec::new(),
        })
    }

    pub fn register(
        self,
        name: &'static str,
        compiler: impl Fn(&mut InputArgs) -> Result<Effect, ScriptError> + 'static
    ) -> Self {
        let mut this = self;
        this.compilers.push(EffectCompiler::new(name, compiler));
        this
    }

    pub fn execute(
        &self,
        env: ScriptEnv,
        input: &str,
    ) -> Result<Effect, ScriptError> {
        parse_expr(input)
            .ok_or_else(|| ScriptError::ParseError(input.to_string()))
            .and_then(|expr| self.compile(&env, expr))
    }

    pub(super) fn compile(
        &self,
        env: &ScriptEnv,
        input: Expr
    ) -> Result<Effect, ScriptError> {
        match input {
            Expr::Fx { name, arguments: parameters } => self.compilers
                .iter()
                .find(|d| d.name == name)
                .ok_or(ScriptError::UnknownEffect { name })
                .and_then(|d| {
                    let mut args = InputArgs::new(parameters.into(), &self, &env);
                    let effect = (d.compiler)(&mut args);

                    match () {
                        _ if effect.is_err() => effect,
                        _ if !args.args().is_empty() => Err(ScriptError::TooManyArguments {
                            expected: args.original_arg_count() - args.args().len(),
                            actual: args.original_arg_count(),
                        }),
                        _ => effect,
                    }
                }),
            _ => Err(ScriptError::InvalidExpression {
                expected: "effect",
                actual: type_name_of(&input),
            }),
        }
    }
}

fn register_default_compilers(context: ScriptContext) -> ScriptContext {
    context
        .register("coalesce",       compilers::coalesce)
        .register("coalesce_from",  compilers::coalesce_from)
        .register("consume_tick",   |args| consume_tick().into())
        .register("delay",          compilers::delay)
        .register("dissolve",       |args| dissolve(args.effect_timer()?).into())
        .register("dissolve_to",    compilers::dissolve_to)
        .register("fade_from",      compilers::fade_from)
        .register("fade_from_fg",   compilers::fade_from_fg)
        .register("fade_to",        compilers::fade_to)
        .register("fade_to_fg",     compilers::fade_to_fg)
        .register("never_complete", |args| never_complete(args.effect()?).into())
        .register("ping_pong",      |args| ping_pong(args.effect()?).into())
        .register("prolong_end",    compilers::prolong_end)
        .register("prolong_start",  compilers::prolong_start)
        .register("repeating",      |args| repeating(args.effect()?).into())
        .register("slide_in",       compilers::slide_in)
        .register("slide_out",      compilers::slide_out)
        .register("sweep_in",       compilers::sweep_in)
        .register("sweep_out",      compilers::sweep_out)
        .register("with_duration",  compilers::with_duration)
        .register(
            "timed_never_complete",
            compilers::timed_never_complete
        )
}

mod compilers {
    use crate::script::script::InputArgs;
    use crate::script::ScriptError;
    use crate::{fx, Effect};

    pub(super) fn coalesce(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::coalesce(args.effect_timer()?).into()
    }

    pub(super) fn coalesce_from(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::coalesce_from(
            args.style()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn fade_to_fg(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::fade_to_fg(
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn fade_from_fg(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::fade_from_fg(
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn fade_to(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::fade_to(
            args.color()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn dissolve_to(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::dissolve_to(
            args.style()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn fade_from(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::fade_from(
            args.color()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn sweep_out(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::sweep_out(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn sleep(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::sleep(args.effect_timer()?).into()
    }

    pub(super) fn delay(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::delay(args.effect_timer()?, args.effect()?).into()
    }

    pub(super) fn prolong_start(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::prolong_start(args.effect_timer()?, args.effect()?).into()
    }

    pub(super) fn prolong_end(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::prolong_end(args.effect_timer()?, args.effect()?).into()
    }

    pub(super) fn sweep_in(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::sweep_in(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn slide_in(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::slide_in(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn slide_out(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::slide_out(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn with_duration(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::with_duration(
            args.duration()?,
            args.effect()?
        ).into()
    }

    pub(super) fn timed_never_complete(args: &mut InputArgs) -> Result<Effect, ScriptError> {
        fx::timed_never_complete(args.duration()?, args.effect()?).into()
    }
}

impl From<Effect> for Result<Effect, ScriptError> {
    fn from(effect: Effect) -> Self {
        Ok(effect)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use super::*;
    use crate::Interpolation::QuadOut;
    use crate::{Duration, Interpolation, Motion, Shader};
    use ratatui::style::{Color, Style};
    use Interpolation::Linear;

    #[test]
    fn happy_path_no_bound_vars() {
        let input = r#"fx::sweep_in(
                Motion::LeftToRight,
                10,
                0,
                Color::from_u32(0x1d2021),
                (1000, QuadOut)
            )"#;

        let ctx = ScriptContext::new();
        let effect = ctx.execute(ScriptEnv::new(), input)
            .expect("effect to be compiled");

        assert_eq!(effect.name(), "sweep_in");
        assert_eq!(effect.timer(), Some(EffectTimer::from_ms(1000, QuadOut)));
    }

    #[test]
    fn happy_path_with_bound_vars() {
        let env = ScriptEnv::new()
            .bind("motion", Motion::LeftToRight)
            .bind("c", Color::from_u32(0x1d2021));

        let input = r#"fx::sweep_in(
                motion,
                10,
                0,
                c,
                (1000, QuadOut)
            )"#;


        let ctx = ScriptContext::new();
        let effect = ctx.execute(env, input)
            .expect("effect to be compiled");

        assert_eq!(effect.name(), "sweep_in");
        assert_eq!(effect.timer(), Some(EffectTimer::from_ms(1000, QuadOut)));
        println!("{:?}", effect);
    }

    #[test]
    fn error_unknown_effect() {
        let input = r#"fx::nonexistent()"#;
        let ctx = ScriptContext::new();
        let err = ctx.execute(ScriptEnv::new(), input).unwrap_err();
        assert!(matches!(err, ScriptError::UnknownEffect { .. }));
    }

    #[test]
    fn error_invalid_argument() {
        let input = r#"fx::sweep_in("wrong", 10, 0, Color::from_u32(0x1d2021), 1000)"#;
        let ctx = ScriptContext::new();
        let err = ctx.execute(ScriptEnv::new(), input).unwrap_err();
        assert!(matches!(err, ScriptError::WrongArgumentType {
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

        let ctx = ScriptContext::new();
        let err = ctx.execute(ScriptEnv::new(), input).unwrap_err();
        assert!(matches!(err, ScriptError::TooManyArguments { .. }), "{:?}", err);
    }

    #[test]
    fn test_coalesce_compiler() {
        let context = ScriptContext::new();
        let env = ScriptEnv::new();
        let exprs = vec![
            Expr::Timer(EffectTimer::from_ms(500, Linear))
        ];
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::coalesce(&mut args).unwrap();
        assert_eq!(effect.name(), "coalesce");
        assert_eq!(effect.timer(), Some(EffectTimer::from_ms(500, Linear).reversed()));
    }

    #[test]
    fn test_coalesce_from_compiler() {
        let context = ScriptContext::new();
        let style = Style::default().fg(Color::Red);
        let exprs = vec![
            Expr::Style(style),
            Expr::Timer(EffectTimer::from_ms(500, Linear))
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::coalesce_from(&mut args).unwrap();
        assert_eq!(effect.name(), "coalesce_from");
    }

    #[test]
    fn test_fade_to_fg_compiler() {
        let context = ScriptContext::new();
        let exprs = vec![
            Expr::Color(Color::Red),
            Expr::Timer(EffectTimer::from_ms(500, Linear))
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::fade_to_fg(&mut args).unwrap();
        assert_eq!(effect.name(), "fade_to");
    }

    #[test]
    fn test_fade_from_fg_compiler() {
        let context = ScriptContext::new();
        let exprs = vec![
            Expr::Color(Color::Red),
            Expr::Timer(EffectTimer::from_ms(500, Linear))
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::fade_from_fg(&mut args).unwrap();
        assert_eq!(effect.name(), "fade_from");
    }

    #[test]
    fn test_fade_to_compiler() {
        let context = ScriptContext::new();
        let exprs = vec![
            Expr::Color(Color::Red),
            Expr::Color(Color::Blue),
            Expr::Timer(EffectTimer::from_ms(500, Linear))
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::fade_to(&mut args).unwrap();
        assert_eq!(effect.name(), "fade_to");
    }

    #[test]
    fn test_dissolve_to_compiler() {
        let context = ScriptContext::new();
        let style = Style::default().fg(Color::Red);
        let exprs = vec![
            Expr::Style(style),
            Expr::Timer(EffectTimer::from_ms(500, Linear))
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::dissolve_to(&mut args).unwrap();
        assert_eq!(effect.name(), "dissolve_to");
    }

    #[test]
    fn test_fade_from_compiler() {
        let context = ScriptContext::new();
        let exprs = vec![
            Expr::Color(Color::Red),
            Expr::Color(Color::Blue),
            Expr::Timer(EffectTimer::from_ms(500, Linear))
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::fade_from(&mut args).unwrap();
        assert_eq!(effect.name(), "fade_from");
    }

    #[test]
    fn test_sweep_out_compiler() {
        let context = ScriptContext::new();
        let exprs = vec![
            Expr::Motion(Motion::LeftToRight),
            Expr::U32(10),
            Expr::U32(0),
            Expr::Color(Color::Red),
            Expr::Timer(EffectTimer::from_ms(500, Linear))
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::sweep_out(&mut args).unwrap();
        assert_eq!(effect.name(), "sweep_out");
    }

    #[test]
    fn test_sleep_compiler() {
        let context = ScriptContext::new();
        let exprs = vec![
            Expr::Timer(EffectTimer::from_ms(500, Linear))
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::sleep(&mut args).unwrap();
        assert_eq!(effect.name(), "sleep");
    }

    #[test]
    fn test_sweep_in_compiler() {
        let context = ScriptContext::new();
        let exprs = vec![
            Expr::Motion(Motion::LeftToRight),
            Expr::U32(10),
            Expr::U32(0),
            Expr::Color(Color::Red),
            Expr::Timer(EffectTimer::from_ms(500, Linear))
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::sweep_in(&mut args).unwrap();
        assert_eq!(effect.name(), "sweep_in");
    }

    #[test]
    fn test_slide_in_compiler() {
        let context = ScriptContext::new();
        let exprs = vec![
            Expr::Motion(Motion::LeftToRight),
            Expr::U32(10),
            Expr::U32(0),
            Expr::Color(Color::Red),
            Expr::Timer(EffectTimer::from_ms(500, Linear))
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::slide_in(&mut args).unwrap();
        assert_eq!(effect.name(), "slide_in");
    }

    #[test]
    fn test_slide_out_compiler() {
        let context = ScriptContext::new();
        let exprs = vec![
            Expr::Motion(Motion::LeftToRight),
            Expr::U32(10),
            Expr::U32(0),
            Expr::Color(Color::Red),
            Expr::Timer(EffectTimer::from_ms(500, Linear))
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::slide_out(&mut args).unwrap();
        assert_eq!(effect.name(), "slide_out");
    }

    #[test]
    fn test_with_duration_compiler() {
        let context = ScriptContext::new();
        let exprs = vec![
            Expr::Duration(Duration::from_millis(1000)),
            Expr::Fx {
                name: "dissolve".to_string(),
                arguments: vec![Expr::Timer(EffectTimer::from_ms(500, Linear))]
            }
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::with_duration(&mut args).unwrap();
        assert_eq!(effect.name(), "with_duration");
    }

    #[test]
    fn test_timed_never_complete_compiler() {
        let context = ScriptContext::new();
        let exprs = vec![
            Expr::Duration(Duration::from_millis(1000)),
            Expr::Fx {
                name: "dissolve".to_string(),
                arguments: vec![Expr::Timer(EffectTimer::from_ms(500, Linear))]
            }
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::timed_never_complete(&mut args).unwrap();
        assert_eq!(effect.name(), "with_duration");
    }

    #[test]
    fn test_delay_compiler() {
        let context = ScriptContext::new();
        let exprs = vec![
            Expr::Timer(EffectTimer::from_ms(500, Linear)),
            Expr::Fx {
                name: "dissolve".to_string(),
                arguments: vec![Expr::Timer(EffectTimer::from_ms(500, Linear))]
            }
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::delay(&mut args).unwrap();
        assert_eq!(effect.name(), "sequence");
    }

    #[test]
    fn test_prolong_start_compiler() {
        let context = ScriptContext::new();
        let exprs = vec![
            Expr::Timer(EffectTimer::from_ms(500, Linear)),
            Expr::Fx {
                name: "dissolve".to_string(),
                arguments: vec![Expr::Timer(EffectTimer::from_ms(500, Linear))]
            }
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::prolong_start(&mut args).unwrap();
        assert_eq!(effect.name(), "prolong_start");
    }

    #[test]
    fn test_prolong_end_compiler() {
        let context = ScriptContext::new();
        let exprs = vec![
            Expr::Timer(EffectTimer::from_ms(500, Linear)),
            Expr::Fx {
                name: "dissolve".to_string(),
                arguments: vec![Expr::Timer(EffectTimer::from_ms(500, Linear))]
            }
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        let effect = compilers::prolong_end(&mut args).unwrap();
        assert_eq!(effect.name(), "prolong_end");
    }

    // Error cases
    #[test]
    fn test_compiler_missing_arguments() {
        let context = ScriptContext::new();
        let exprs = vec![];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        assert!(compilers::fade_to_fg(&mut args).is_err());
    }

    #[test]
    fn test_compiler_wrong_argument_type() {
        let context = ScriptContext::new();
        let exprs = vec![
            Expr::String("wrong".to_string()),
            Expr::Timer(EffectTimer::from_ms(500, Linear))
        ];
        let env = ScriptEnv::new();
        let mut args = InputArgs::new(VecDeque::from(exprs), &context, &env);
        assert!(compilers::fade_to_fg(&mut args).is_err());
    }
}
