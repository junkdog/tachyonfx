use crate::fx::{consume_tick, dissolve, ping_pong, repeating};
use crate::script::parser::Expr;
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



#[derive(Default)]
pub struct ScriptContext {
    deserializers: Vec<EffectCompiler>,
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
        Self::default()
    }

    pub fn execute(
        &self,
        env: &mut ScriptEnv,
        input: &str,
    ) -> Option<Effect> {
        unimplemented!("parse input")
    }

    fn compile(
        &self,
        env: &mut ScriptEnv,
        input: Expr
    ) -> Option<Effect> {
        match input {
            Expr::Fx { name, parameters } => self.deserializers
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
        this.deserializers.push(EffectCompiler::new(name, compiler));
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
    use super::*;
    use crate::Interpolation;
    use std::collections::VecDeque;

    fn empty_env() -> BTreeMap<&'static str, Box<dyn Any>> {
        BTreeMap::new()
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