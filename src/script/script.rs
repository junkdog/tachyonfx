use std::any::Any;
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style};
use crate::{Duration, Effect, EffectTimer, Motion};
use crate::fx::{consume_tick, dissolve, dissolve_to, fade_from, ping_pong, repeating, slide_in, sweep_in, sweep_out};
use crate::script::parser::{FxArg};


struct Deserializer {
    name: &'static str,
    deserialize: Box<dyn Fn(&mut InputArgs) -> Option<Effect>>,
}

pub struct InputArgs<'a> {
    args: VecDeque<FxArg>,
    vars: &'a BTreeMap<&'static str, Box<dyn Any>>,
}


#[derive(Default)]
pub struct ScriptContext {
    deserializers: Vec<Deserializer>,
}

pub struct ScriptEnv {
    bound_variables: BTreeMap<&'static str, Box<dyn Any>>,
}

impl<'a> InputArgs<'a> {
    pub(super) fn new(
        args: VecDeque<FxArg>,
        vars: &'a BTreeMap<&'static str, Box<dyn Any>>
    ) -> Self {
        Self { args, vars }
    }

    pub fn duration(&mut self) -> Option<Duration> {
        match self.next()? {
            FxArg::Duration(d) => Some(d),
            FxArg::U32(ms)     => Some(Duration::from_millis(ms as _)),
            _                  => None,
        }
    }

    pub fn effect_timer(&mut self) -> Option<EffectTimer> {
        match self.next()? {
            FxArg::Timer(t) => Some(t),
            FxArg::U32(ms)  => Some(ms.into()),
            _               => None,
        }
    }

    pub fn read_u16(&mut self) -> Option<u16> {
        self.read_u32().map(|u| u as _)
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        match self.next()? {
            FxArg::U32(u) => Some(u),
            _             => None,
        }
    }

    pub fn read_f32(&mut self) -> Option<f32> {
        match self.next()? {
            FxArg::F32(f) => Some(f),
            _             => None,
        }
    }

    pub fn string(&mut self) -> Option<String> {
        match self.next()? {
            FxArg::String(s) => Some(s),
            _                => None,
        }
    }

    pub fn effect(&mut self) -> Option<Effect> {
        match self.next()? {
            FxArg::Fx { name, parameters } => {
                // todo: recursive deserialization?
                None
            },
            _ => None,
        }
    }

    pub fn color(&mut self) -> Option<Color> {
        match self.next()? {
            FxArg::Color(c) => Some(c),
            _               => None,
        }
    }

    pub fn style(&mut self) -> Option<Style> {
        match self.next()? {
            FxArg::Style(s) => Some(s),
            _               => None,
        }
    }

    pub fn motion(&mut self) -> Option<Motion> {
        match self.next()? {
            FxArg::Motion(m) => Some(m),
            _                => None,
        }
    }

    pub fn margin(&mut self) -> Option<Margin> {
        match self.next()? {
            FxArg::Margin(m) => Some(m),
            _                => None,
        }
    }

    pub fn rect(&mut self) -> Option<Rect> {
        match self.next()? {
            FxArg::Rect(r) => Some(r),
            _              => None,
        }
    }

    fn next(&mut self) -> Option<FxArg> {
        self.args.pop_front()
    }
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

impl Deserializer {
    pub(crate) fn new(
        name: &'static str,
        deserialize: impl Fn(&mut InputArgs) -> Option<Effect> + 'static
    ) -> Self {
        Self {
            name,
            deserialize: Box::new(deserialize),
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

    fn deserialize(
        &self,
        env: &mut ScriptEnv,
        input: FxArg
    ) -> Option<Effect> {
        match input {
            FxArg::Fx { name, parameters } => self.deserializers
                .iter()
                .find(|d| d.name == name)
                .and_then(|d| {
                    let mut args = InputArgs::new(parameters.into(), &env.bound_variables);
                    let effect = (d.deserialize)(&mut args);
                    debug_assert!(args.args.is_empty(), "unused arguments: {:?}", args.args);
                    effect
                }),
            _ => None
        }
    }

    pub fn register(
        self,
        name: &'static str,
        deserialize: impl Fn(&mut InputArgs) -> Option<Effect> + 'static
    ) -> Self {
        let mut this = self;
        this.deserializers.push(Deserializer::new(name, deserialize));
        this
    }
}

fn register_default_deserializers(ctx: ScriptContext) -> ScriptContext {
    ctx.register("consume_tick", |args| {
       consume_tick().into()
    }).register("ping_pong", |args| {
        ping_pong(args.effect()?).into()
    }).register("repeating", |args| {
        repeating(args.effect()?).into()
    }).register("dissovle", |args| {
        dissolve(args.effect_timer()?).into()
    }).register("dissolve_to", |args| {
        dissolve_to(
            args.style()?,
            args.effect_timer()?
        ).into()
    }).register("fade_from", |args| {
        fade_from(
            args.color()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }).register("sweep_out", |args| {
        sweep_out(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }).register("sweep_in", |args| {
        sweep_in(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }).register("slide_in", |args| {
        slide_in(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }).register("slide_out", |args| {
        slide_in(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use crate::Interpolation;

    fn empty_env() -> BTreeMap<&'static str, Box<dyn Any>> {
        BTreeMap::new()
    }

    #[test]
    fn test_duration_parsing() {
        let binding = empty_env();
        let mut args = InputArgs::new(
            vec![
                FxArg::Duration(Duration::from_millis(500)),
                FxArg::U32(1000),
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
                FxArg::Timer(EffectTimer::from_ms(500, Interpolation::Linear)),
                FxArg::U32(1000),
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
                FxArg::U32(42),
                FxArg::F32(3.14),
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
                FxArg::String("hello".to_string()),
                FxArg::U32(42), // Wrong type
                FxArg::String("world".to_string()),
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
                FxArg::Color(Color::Red),
                FxArg::Color(Color::Blue),
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
                FxArg::Style(style),
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
                FxArg::Motion(Motion::LeftToRight),
                FxArg::Motion(Motion::UpToDown),
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
                FxArg::Margin(margin),
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
                FxArg::Rect(rect),
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
                FxArg::Fx {
                    name: "test".to_string(),
                    parameters: vec![FxArg::U32(500)]
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
                FxArg::U32(500),
                FxArg::Motion(Motion::LeftToRight),
                FxArg::Color(Color::Blue),
                FxArg::Timer(EffectTimer::from_ms(1000, Interpolation::Linear)),
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
                FxArg::U32(65535), // Max u16
                FxArg::U32(65536), // Too large for u16
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