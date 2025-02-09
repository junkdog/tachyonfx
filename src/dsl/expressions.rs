use ratatui::layout::{Margin, Rect};
use ratatui::prelude::{Color, Style};
use crate::{Duration, EffectTimer, Motion};
use crate::fx::RepeatMode;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Expr {
    Color(Color),
    Style(Style),
    String(String),
    U32(u32), // can also repr EffectTimer and Duration
    F32(f32),
    Duration(Duration),
    Timer(EffectTimer),
    Motion(Motion),
    Rect(Rect),
    Margin(Margin),
    ArrayRef(Vec<Expr>),
    Var(String),
    RepeatMode(RepeatMode),
    Fx { name: String, arguments: Vec<Expr> }
}

impl Expr {
    /// Returns a string representation of the expression's type
    /// Used for error messages
    pub fn type_name(&self) -> &'static str {
        match self {
            Expr::Color(_)      => "color",
            Expr::Style(_)      => "style",
            Expr::String(_)     => "string",
            Expr::U32(_)        => "u32",
            Expr::F32(_)        => "f32",
            Expr::Duration(_)   => "duration",
            Expr::Timer(_)      => "timer",
            Expr::Motion(_)     => "motion",
            Expr::Rect(_)       => "rect",
            Expr::Margin(_)     => "margin",
            Expr::ArrayRef(_)   => "array",
            Expr::Var(_)        => "variable",
            Expr::RepeatMode(_) => "repeat_mode",
            Expr::Fx { .. }     => "effect",
        }
    }
}