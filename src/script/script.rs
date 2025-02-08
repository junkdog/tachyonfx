use crate::fx::{consume_tick, dissolve, ping_pong, repeating};
use crate::script::args::{type_name_of, InputArgs};
use crate::script::env::ScriptEnv;
use crate::script::parser::{parse_expr, Expr};
use crate::{Effect, EffectTimer};
use std::any::Any;
use crate::script::ScriptError;

struct EffectCompiler {
    name: &'static str,
    compiler: Box<dyn Fn(&mut InputArgs) -> Result<Effect, ScriptError>>,
}



pub struct ScriptContext {
    compilers: Vec<EffectCompiler>,
}

impl EffectCompiler {
    pub(crate) fn new(
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

    pub fn execute(
        &self,
        env: ScriptEnv,
        input: &str,
    ) -> Result<Effect, ScriptError> {
        parse_expr(input)
            .ok_or(ScriptError::ParseError(input.to_string()))
            .and_then(|expr| self.compile(&env, expr))
    }

    pub(super) fn compile(
        &self,
        env: &ScriptEnv,
        input: Expr
    ) -> Result<Effect, ScriptError> {
        match input {
            Expr::Fx { name, parameters } => self.compilers
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

    pub fn register(
        self,
        name: &'static str,
        compiler: impl Fn(&mut InputArgs) -> Result<Effect, ScriptError> + 'static
    ) -> Self {
        let mut this = self;
        this.compilers.push(EffectCompiler::new(name, compiler));
        this
    }
}

fn register_default_compilers(context: ScriptContext) -> ScriptContext {
    context
        .register("consume_tick", |args| consume_tick().into())
        .register("ping_pong",    |args| ping_pong(args.effect()?).into())
        .register("repeating",    |args| repeating(args.effect()?).into())
        .register("dissolve",     |args| dissolve(args.effect_timer()?).into())
        .register("dissolve_to",  compilers::dissolve_to)
        .register("fade_from",    compilers::fade_from)
        .register("sweep_out",    compilers::sweep_out)
        .register("sweep_in",     compilers::sweep_in)
        .register("slide_in",     compilers::slide_in)
        .register("slide_out",    compilers::slide_out)
}

mod compilers {
    use crate::script::script::InputArgs;
    use crate::{fx, Effect};
    use crate::script::ScriptError;

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
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;
    use super::*;
    use crate::Interpolation::QuadOut;
    use crate::{Motion, Shader};

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
}

impl From<Effect> for Result<Effect, ScriptError> {
    fn from(effect: Effect) -> Self {
        Ok(effect)
    }
}