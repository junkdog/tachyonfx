use crate::fx::{consume_tick, dissolve, ping_pong, repeating};
use crate::script::parser::{parse_expr, Expr};
use crate::{Duration, Effect, EffectTimer, Motion};
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style};
use std::any::Any;
use std::collections::{BTreeMap, VecDeque};
use crate::script::args::InputArgs;

struct EffectCompiler {
    name: &'static str,
    compiler: Box<dyn Fn(&mut InputArgs) -> Option<Effect>>,
}



pub struct ScriptContext {
    compilers: Vec<EffectCompiler>,
}

pub struct ScriptEnv {
    bound_variables: BTreeMap<&'static str, Box<dyn Any>>,
}


impl ScriptEnv {
    pub(crate) fn new() -> Self {
        Self {
            bound_variables: BTreeMap::new(),
        }
    }

    pub fn bind_variable<T: 'static>(self, name: &'static str, value: T) -> Self {
        let mut this = self;
        this.bound_variables.insert(name, Box::new(value));
        this
    }

    pub fn get_variable<T: 'static>(&self, name: &'static str) -> Option<&T> {
        self.bound_variables.get(name).and_then(|v| v.downcast_ref())
    }
}

impl EffectCompiler {
    pub(crate) fn new(
        name: &'static str,
        compiler: impl Fn(&mut InputArgs) -> Option<Effect> + 'static
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
        env: &mut ScriptEnv,
        input: &str,
    ) -> Option<Effect> {
        parse_expr(input)
            .and_then(|expr| self.compile(env, expr))
    }

    fn compile(
        &self,
        env: &mut ScriptEnv,
        input: Expr
    ) -> Option<Effect> {
        match input {
            Expr::Fx { name, parameters } => self.compilers
                .iter()
                .find(|d| d.name == name)
                .and_then(|d| {
                    let mut args = InputArgs::new(parameters.into(), &env.bound_variables);
                    let effect = (d.compiler)(&mut args);
                    debug_assert!(args.args().is_empty(), "unused arguments: {:?}", args.args());
                    effect
                }),
            _ => None
        }
    }

    pub fn register(
        self,
        name: &'static str,
        compiler: impl Fn(&mut InputArgs) -> Option<Effect> + 'static
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

    pub(super) fn dissolve_to(args: &mut InputArgs) -> Option<Effect> {
        fx::dissolve_to(
            args.style()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn fade_from(args: &mut InputArgs) -> Option<Effect> {
        fx::fade_from(
            args.color()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn sweep_out(args: &mut InputArgs) -> Option<Effect> {
        fx::sweep_out(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn sweep_in(args: &mut InputArgs) -> Option<Effect> {
        fx::sweep_in(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn slide_in(args: &mut InputArgs) -> Option<Effect> {
        fx::slide_in(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn slide_out(args: &mut InputArgs) -> Option<Effect> {
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
    use crate::Interpolation::QuadOut;
    use crate::Shader;
    use super::*;

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
        let effect = ctx.execute(&mut ScriptEnv::new(), input)
            .expect("effect to be compiled");
        
        assert_eq!(effect.name(), "sweep_in");
        assert_eq!(effect.timer(), Some(EffectTimer::from_ms(1000, QuadOut)));
    }
}