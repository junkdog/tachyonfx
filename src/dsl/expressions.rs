use crate::fx::RepeatMode;
use crate::{CellFilter, Duration, EffectTimer, Interpolation, Motion};
use ratatui::layout::{Margin, Rect};
use ratatui::prelude::{Color, Style};

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
    Sequence(Vec<Expr>),
    Parallel(Vec<Expr>),
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
            Expr::Sequence(_)       => "sequence",
            Expr::Parallel(_)       => "parallel",
        }
    }
}