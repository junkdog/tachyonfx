use std::any::Any;
use std::collections::{BTreeMap, VecDeque};
use ratatui::layout::{Margin, Rect};
use ratatui::prelude::{Color, Style};
use crate::{Duration, Effect, EffectTimer, Motion};
use crate::script::parser::Expr;

pub struct InputArgs<'a> {
    args: VecDeque<Expr>,
    vars: &'a BTreeMap<&'static str, Box<dyn Any>>,
}

impl<'a> InputArgs<'a> {
    pub(super) fn new(
        args: VecDeque<Expr>,
        vars: &'a BTreeMap<&'static str, Box<dyn Any>>
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
            _                 => None,
        }
    }

    pub fn effect_timer(&mut self) -> Option<EffectTimer> {
        match self.next()? {
            Expr::Timer(t) => Some(t),
            Expr::U32(ms)  => Some(ms.into()),
            _              => None,
        }
    }

    pub fn read_u16(&mut self) -> Option<u16> {
        self.read_u32().map(|u| u as _)
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        match self.next()? {
            Expr::U32(u) => Some(u),
            _            => None,
        }
    }

    pub fn read_f32(&mut self) -> Option<f32> {
        match self.next()? {
            Expr::F32(f) => Some(f),
            _            => None,
        }
    }

    pub fn string(&mut self) -> Option<String> {
        match self.next()? {
            Expr::String(s) => Some(s),
            _               => None,
        }
    }

    pub fn effect(&mut self) -> Option<Effect> {
        match self.next()? {
            Expr::Fx { name, parameters } => {
                // todo: recursive deserialization?
                None
            },
            _ => None,
        }
    }

    pub fn color(&mut self) -> Option<Color> {
        match self.next()? {
            Expr::Color(c) => Some(c),
            _              => None,
        }
    }

    pub fn style(&mut self) -> Option<Style> {
        match self.next()? {
            Expr::Style(s) => Some(s),
            _              => None,
        }
    }

    pub fn motion(&mut self) -> Option<Motion> {
        match self.next()? {
            Expr::Motion(m) => Some(m),
            _               => None,
        }
    }

    pub fn margin(&mut self) -> Option<Margin> {
        match self.next()? {
            Expr::Margin(m) => Some(m),
            _               => None,
        }
    }

    pub fn rect(&mut self) -> Option<Rect> {
        match self.next()? {
            Expr::Rect(r) => Some(r),
            _             => None,
        }
    }

    fn next(&mut self) -> Option<Expr> {
        self.args.pop_front()
    }
}
