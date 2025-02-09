use std::cell::Cell;
use std::fmt;
use std::fmt::Formatter;
use ratatui::layout::{Margin, Rect};
use ratatui::prelude::{Color, Style};
use crate::{CellFilter, Duration, EffectTimer, Interpolation, Motion};
use crate::fx::RepeatMode;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Expr {
    Literal(Value),
    Var(String),
    ArrayRef(Vec<Expr>),
    CellFilter { filter_type: &'static str, arguments: Vec<Expr> },
    Call {
        function: FnCall,  // e.g. ["Duration", "from_millis"]
        args: Vec<Expr>
    },
    Fx {
        name: String,
        arguments: Vec<Expr>
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FnCall {
    ColorFromU32,
    DurationFromMillis,
    DurationFromSeconds,
    EffectTimerNew,
    EffectTimerFromMs,
    EffectTimerFromSeconds,
    RectNew,
    RectStruct,
    MarginNew,
    MarginStruct,
    RepeatModeDuration,
    RepeatModeTimes,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Value {
    CellFilter(CellFilter),
    Color(Color),
    Style(Style),
    String(String),
    U16(u16),
    U32(u32),
    F32(f32),
    Duration(Duration),
    Timer(EffectTimer),
    Motion(Motion),
    Rect(Rect),
    Margin(Margin),
    RepeatMode(RepeatMode),
    Interpolation(Interpolation),
}

impl Expr {
    /// Returns a string representation of the expression's type
    /// Used for error messages
    pub fn type_name(&self) -> &'static str {
        match self {
            Expr::Var(_)            => "variable",
            Expr::Fx { .. }         => "effect",
            Expr::Literal(_)        => "literal",
            Expr::Call { .. }       => "function_call",
            Expr::ArrayRef(_)       => "array_ref",
            Expr::CellFilter { .. } => "cell_filter",
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Literal(v) => write!(f, "{}", v),
            Expr::Var(s) => write!(f, "{}", s),
            Expr::ArrayRef(v) => write!(f, "{:?}", v),
            Expr::CellFilter { filter_type, arguments } => {
                write!(f, "{}({:?})", filter_type, arguments)
            }
            Expr::Call { function, args } => {
                write!(f, "{:?}({:?})", function, args)
            }
            Expr::Fx { name, arguments } => {
                write!(f, "{}({:?})", name, arguments)
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Value::CellFilter(cf) => write!(f, "{:?}", cf),
            Value::Color(c) => write!(f, "{:?}", c),
            Value::Style(s) => write!(f, "{:?}", s),
            Value::String(s) => write!(f, "{:?}", s),
            Value::U16(u) => write!(f, "{:?}", u),
            Value::U32(u) => write!(f, "{:?}", u),
            Value::F32(v) => write!(f, "{:?}", v),
            Value::Duration(d) => write!(f, "{:?}", d),
            Value::Timer(t) => write!(f, "{:?}", t),
            Value::Motion(m) => write!(f, "{:?}", m),
            Value::Rect(r) => write!(f, "{:?}", r),
            Value::Margin(m) => write!(f, "{:?}", m),
            Value::RepeatMode(r) => write!(f, "{:?}", r),
            Value::Interpolation(i) => write!(f, "{:?}", i),
        }
    }
}