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
    fn new(
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
       Some(consume_tick())
    }).register("ping_pong", |args| {
        Some(ping_pong(args.effect()?))
    }).register("repeating", |args| {
        Some(repeating(args.effect()?))
    }).register("dissovle", |args| {
        Some(dissolve(args.effect_timer()?))
    }).register("dissolve_to", |args| {
        Some(dissolve_to(
            args.style()?,
            args.effect_timer()?
        ))
    }).register("fade_from", |args| {
        Some(fade_from(
            args.color()?,
            args.color()?,
            args.effect_timer()?
        ))
    }).register("sweep_out", |args| {
        Some(sweep_out(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ))
    }).register("sweep_in", |args| {
        Some(sweep_in(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ))
    }).register("slide_in", |args| {
        Some(slide_in(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ))
    }).register("slide_out", |args| {
        Some(slide_in(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ))
    })
}