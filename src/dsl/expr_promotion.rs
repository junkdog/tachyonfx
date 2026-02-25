use ratatui_core::{
    layout::{Direction, Flex},
    style::{Color, Modifier},
};

use crate::{
    dsl::expressions::{Expr, ExprSpan, FnCallInfo, Value},
    fx::RepeatMode,
    CellFilter, ColorSpace, Interpolation, Motion,
};

/// Attempts to convert qualified identifiers like `Motion::LeftToRight` or
/// `Interpolation::Linear` into their corresponding literal values.
///
/// Supported types for promotion:
/// - Motion enum variants
/// - Direction enum variants
/// - Flex enum variants
/// - ExpandDirection enum variants
/// - ColorSpace enum variants
/// - CellFilter enum variants
/// - Modifier enum variants
/// - Interpolation enum variants
/// - Color enum variants
/// - RepeatMode enum variants
/// - None
///
/// # Arguments
/// * `expr` - The expression to potentially promote
///
/// # Returns
/// The promoted expression if a match was found, or the original expression unchanged
pub(super) fn maybe_promote(expr: Expr) -> Expr {
    match &expr {
        Expr::QualifiedMember { name, self_fns, span } => {
            promote(name, *span).map(|f| f.self_fns(self_fns.clone()))
        },
        Expr::Var { name, self_fns, span } => {
            promote(name, *span).map(|f| f.self_fns(self_fns.clone()))
        },
        _ => None,
    }
    .unwrap_or(expr)
}

fn promote(text: &str, span: ExprSpan) -> Option<Expr> {
    motion(text)
        .or_else(|| direction(text))
        .or_else(|| flex(text))
        .or_else(|| expand_direction(text))
        .or_else(|| cell_filter(text))
        .or_else(|| modifier(text))
        .or_else(|| color_space(text))
        .or_else(|| interpolation(text))
        .or_else(|| color(text))
        .or_else(|| repeat_mode(text))
        .or_else(|| evolve_symbol_set(text))
        .or_else(|| none(text))
        .map(|v| Expr::Literal(v, span))
}

fn motion(text: &str) -> Option<Value> {
    Some(Value::Motion(match text.trim_start_matches("Motion::") {
        "LeftToRight" => Motion::LeftToRight,
        "RightToLeft" => Motion::RightToLeft,
        "UpToDown" => Motion::UpToDown,
        "DownToUp" => Motion::DownToUp,
        _ => None?,
    }))
}

fn color_space(text: &str) -> Option<Value> {
    Some(Value::ColorSpace(
        match text.trim_start_matches("ColorSpace::") {
            "Rgb" => ColorSpace::Rgb,
            "Hsl" => ColorSpace::Hsl,
            "Hsv" => ColorSpace::Hsv,
            _ => None?,
        },
    ))
}

fn cell_filter(text: &str) -> Option<Value> {
    Some(Value::CellFilter(
        match text.trim_start_matches("CellFilter::") {
            "All" => CellFilter::All,
            "Text" => CellFilter::Text,
            "NonEmpty" => CellFilter::NonEmpty,
            _ => None?,
        },
    ))
}

fn direction(text: &str) -> Option<Value> {
    Some(Value::Direction(
        match text.trim_start_matches("Direction::") {
            "Horizontal" => Direction::Horizontal,
            "Vertical" => Direction::Vertical,
            _ => None?,
        },
    ))
}

fn flex(text: &str) -> Option<Value> {
    Some(Value::Flex(match text.trim_start_matches("Flex::") {
        "Legacy" => Flex::Legacy,
        "Start" => Flex::Start,
        "End" => Flex::End,
        "Center" => Flex::Center,
        "SpaceBetween" => Flex::SpaceBetween,
        "SpaceAround" => Flex::SpaceAround,
        "SpaceEvenly" => Flex::SpaceEvenly,
        _ => None?,
    }))
}

fn expand_direction(text: &str) -> Option<Value> {
    Some(Value::ExpandDirection(
        match text.trim_start_matches("ExpandDirection::") {
            "Horizontal" => crate::fx::ExpandDirection::Horizontal,
            "Vertical" => crate::fx::ExpandDirection::Vertical,
            _ => None?,
        },
    ))
}

fn modifier(text: &str) -> Option<Value> {
    Some(Value::Modifier(
        match text.trim_start_matches("Modifier::") {
            "BOLD" => Modifier::BOLD,
            "DIM" => Modifier::DIM,
            "ITALIC" => Modifier::ITALIC,
            "UNDERLINED" => Modifier::UNDERLINED,
            "SLOW_BLINK" => Modifier::SLOW_BLINK,
            "RAPID_BLINK" => Modifier::RAPID_BLINK,
            "REVERSED" => Modifier::REVERSED,
            "HIDDEN" => Modifier::HIDDEN,
            "CROSSED_OUT" => Modifier::CROSSED_OUT,
            _ => None?,
        },
    ))
}

fn interpolation(text: &str) -> Option<Value> {
    Some(Value::Interpolation(
        match text.trim_start_matches("Interpolation::") {
            "BackIn" => Interpolation::BackIn,
            "BackOut" => Interpolation::BackOut,
            "BackInOut" => Interpolation::BackInOut,

            "BounceIn" => Interpolation::BounceIn,
            "BounceOut" => Interpolation::BounceOut,
            "BounceInOut" => Interpolation::BounceInOut,

            "CircIn" => Interpolation::CircIn,
            "CircOut" => Interpolation::CircOut,
            "CircInOut" => Interpolation::CircInOut,

            "CubicIn" => Interpolation::CubicIn,
            "CubicOut" => Interpolation::CubicOut,
            "CubicInOut" => Interpolation::CubicInOut,

            "ElasticIn" => Interpolation::ElasticIn,
            "ElasticOut" => Interpolation::ElasticOut,
            "ElasticInOut" => Interpolation::ElasticInOut,

            "ExpoIn" => Interpolation::ExpoIn,
            "ExpoOut" => Interpolation::ExpoOut,
            "ExpoInOut" => Interpolation::ExpoInOut,

            "Linear" => Interpolation::Linear,

            "QuadIn" => Interpolation::QuadIn,
            "QuadOut" => Interpolation::QuadOut,
            "QuadInOut" => Interpolation::QuadInOut,

            "QuartIn" => Interpolation::QuartIn,
            "QuartOut" => Interpolation::QuartOut,
            "QuartInOut" => Interpolation::QuartInOut,

            "QuintIn" => Interpolation::QuintIn,
            "QuintOut" => Interpolation::QuintOut,
            "QuintInOut" => Interpolation::QuintInOut,

            "Reverse" => Interpolation::Reverse,

            "SmoothStep" => Interpolation::SmoothStep,
            "Spring" => Interpolation::Spring,

            "SineIn" => Interpolation::SineIn,
            "SineOut" => Interpolation::SineOut,
            "SineInOut" => Interpolation::SineInOut,

            _ => None?,
        },
    ))
}

fn color(text: &str) -> Option<Value> {
    Some(Value::Color(match text.trim_start_matches("Color::") {
        "Reset" => Color::Reset,
        "Black" => Color::Black,
        "Red" => Color::Red,
        "Green" => Color::Green,
        "Yellow" => Color::Yellow,
        "Blue" => Color::Blue,
        "Magenta" => Color::Magenta,
        "Cyan" => Color::Cyan,
        "Gray" => Color::Gray,
        "DarkGray" => Color::DarkGray,
        "LightRed" => Color::LightRed,
        "LightGreen" => Color::LightGreen,
        "LightYellow" => Color::LightYellow,
        "LightBlue" => Color::LightBlue,
        "LightMagenta" => Color::LightMagenta,
        "LightCyan" => Color::LightCyan,
        "White" => Color::White,
        _ => None?,
    }))
}

fn repeat_mode(text: &str) -> Option<Value> {
    matches!(text.trim_start_matches("RepeatMode::"), "Forever")
        .then(|| RepeatMode::Forever)
        .map(Value::RepeatMode)
}

fn evolve_symbol_set(text: &str) -> Option<Value> {
    use crate::fx::EvolveSymbolSet::*;
    Some(Value::EvolveSymbolSet(
        match text.trim_start_matches("EvolveSymbolSet::") {
            "BlocksHorizontal" => BlocksHorizontal,
            "BlocksVertical" => BlocksVertical,
            "CircleFill" => CircleFill,
            "Circles" => Circles,
            "Quadrants" => Quadrants,
            "Shaded" => Shaded,
            "Squares" => Squares,
            _ => None?,
        },
    ))
}

fn none(text: &str) -> Option<Value> {
    matches!(text, "None").then(|| Value::OptionNone)
}

impl Expr {
    fn self_fns(self, self_fns: Vec<FnCallInfo>) -> Expr {
        match self {
            Expr::Var { name, span, .. } => Expr::Var { name, self_fns, span },
            Expr::QualifiedMember { name, span, .. } => {
                Expr::QualifiedMember { name, self_fns, span }
            },
            _ => self,
        }
    }
}
