use ratatui::layout::{Margin, Rect};
use ratatui::prelude::{Color, Style};
use crate::{Duration, EffectTimer, Motion};

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
    Fx { name: String, arguments: Vec<Expr> }
}
