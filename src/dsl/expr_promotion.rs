use ratatui::layout::Direction;
use ratatui::prelude::Modifier;
use ratatui::style::Color;
use crate::dsl::expressions::{Expr, ExprSpan, FnCallInfo, Value};
use crate::{CellFilter, Interpolation, Motion};

/// Attempts to convert qualified identifiers like `Motion::LeftToRight` or
/// `Interpolation::Linear` into their corresponding literal values.
///
/// Supported types for promotion:
/// - Motion enum variants
/// - Direction enum variants
/// - CellFilter enum variants
/// - Modifier enum variants
/// - Interpolation enum variants
///
/// # Arguments
/// * `expr` - The expression to potentially promote
///
/// # Returns
/// The promoted expression if a match was found, or the original expression unchanged
pub(super) fn maybe_promote<'a>(expr: Expr) -> Expr {
    match &expr {
        Expr::QualifiedMember(s, span)        => promote(s, span),
        Expr::FnCall { call, self_fns, span } => promote(&call.name, span)
            .map(|f| f.self_fns(self_fns.clone())),
        Expr::Var { name, self_fns, span }    => promote(name, span)
            .map(|f| f.self_fns(self_fns.clone())),
        _                                     => None
    }.unwrap_or(expr)
}

fn promote(text: &str, span: &ExprSpan) -> Option<Expr> {
    motion(text)
        .or_else(|| direction(text))
        .or_else(|| cell_filter(text))
        .or_else(|| modifier(text))
        .or_else(|| interpolation(text))
        .or_else(|| color(text))
        .map(|v| Expr::Literal(v, *span))
}

fn motion(text: &str) -> Option<Value> {
    Some(match text.strip_prefix("Motion::").unwrap_or(text) {
        "LeftToRight" => Motion::LeftToRight,
        "RightToLeft" => Motion::RightToLeft,
        "UpToDown"    => Motion::UpToDown,
        "DownToUp"    => Motion::DownToUp,
        _             => None?,
    }).map(Value::Motion)
}

fn cell_filter(text: &str) -> Option<Value> {
    match text.strip_prefix("CellFilter::").unwrap_or(text) {
        "All"        => Some(CellFilter::All),
        "Text"       => Some(CellFilter::Text),
        _            => None?,
    }.map(Value::CellFilter)
}

fn direction(text: &str) -> Option<Value> {
    match text.strip_prefix("Direction::").unwrap_or(text) {
        "Horizontal" => Some(Direction::Horizontal),
        "Vertical"   => Some(Direction::Vertical),
        _            => None,
    }.map(Value::Direction)
}

fn modifier(text: &str) -> Option<Value> {
    Some(match text.strip_prefix("Modifier::").unwrap_or(text) {
        "BOLD"        => Modifier::BOLD,
        "DIM"         => Modifier::DIM,
        "ITALIC"      => Modifier::ITALIC,
        "UNDERLINED"  => Modifier::UNDERLINED,
        "SLOW_BLINK"  => Modifier::SLOW_BLINK,
        "RAPID_BLINK" => Modifier::RAPID_BLINK,
        "REVERSED"    => Modifier::REVERSED,
        "HIDDEN"      => Modifier::HIDDEN,
        "CROSSED_OUT" => Modifier::CROSSED_OUT,
        _             => None?,
    }).map(Value::Modifier)
}

fn interpolation(text: &str) -> Option<Value> {
    Some(match text.strip_prefix("Interpolation::").unwrap_or(text) {
        "BackIn"       => Interpolation::BackIn,
        "BackOut"      => Interpolation::BackOut,
        "BackInOut"    => Interpolation::BackInOut,

        "BounceIn"     => Interpolation::BounceIn,
        "BounceOut"    => Interpolation::BounceOut,
        "BounceInOut"  => Interpolation::BounceInOut,

        "CircIn"       => Interpolation::CircIn,
        "CircOut"      => Interpolation::CircOut,
        "CircInOut"    => Interpolation::CircInOut,

        "CubicIn"      => Interpolation::CubicIn,
        "CubicOut"     => Interpolation::CubicOut,
        "CubicInOut"   => Interpolation::CubicInOut,

        "ElasticIn"    => Interpolation::ElasticIn,
        "ElasticOut"   => Interpolation::ElasticOut,
        "ElasticInOut" => Interpolation::ElasticInOut,

        "ExpoIn"       => Interpolation::ExpoIn,
        "ExpoOut"      => Interpolation::ExpoOut,
        "ExpoInOut"    => Interpolation::ExpoInOut,

        "Linear"       => Interpolation::Linear,

        "QuadIn"       => Interpolation::QuadIn,
        "QuadOut"      => Interpolation::QuadOut,
        "QuadInOut"    => Interpolation::QuadInOut,

        "QuartIn"      => Interpolation::QuartIn,
        "QuartOut"     => Interpolation::QuartOut,
        "QuartInOut"   => Interpolation::QuartInOut,

        "QuintIn"      => Interpolation::QuintIn,
        "QuintOut"     => Interpolation::QuintOut,
        "QuintInOut"   => Interpolation::QuintInOut,

        "Reverse"      => Interpolation::Reverse,

        "SineIn"       => Interpolation::SineIn,
        "SineOut"      => Interpolation::SineOut,
        "SineInOut"    => Interpolation::SineInOut,

        _             => None?,
    }).map(Value::Interpolation)
}

fn color(text: &str) -> Option<Value> {
    Some(match text.strip_prefix("Color::").unwrap_or(text) {
        "Reset"        => Color::Reset,
        "Black"        => Color::Black,
        "Red"          => Color::Red,
        "Green"        => Color::Green,
        "Yellow"       => Color::Yellow,
        "Blue"         => Color::Blue,
        "Magenta"      => Color::Magenta,
        "Cyan"         => Color::Cyan,
        "Gray"         => Color::Gray,
        "DarkGray"     => Color::DarkGray,
        "LightRed"     => Color::LightRed,
        "LightGreen"   => Color::LightGreen,
        "LightYellow"  => Color::LightYellow,
        "LightBlue"    => Color::LightBlue,
        "LightMagenta" => Color::LightMagenta,
        "LightCyan"    => Color::LightCyan,
        "White"        => Color::White,
        _              => None?,
    }).map(Value::Color)
}

impl Expr {
    fn self_fns(self, self_fns: Vec<FnCallInfo>) -> Expr {
        match self {
            Expr::FnCall { call, span, .. } => Expr::FnCall { call, self_fns, span },
            _                               => self
        }
    }
}