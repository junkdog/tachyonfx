use crate::fx::RepeatMode;
use crate::{CellFilter, Duration, EffectTimer, Interpolation, Motion};
use ratatui::layout::{Margin, Rect};
use ratatui::prelude::{Color, Modifier, Style};
use crate::color_ext::ToRgbComponents;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Expr {
    Literal(Value),
    Var(String),
    ArrayRef(Vec<Expr>),
    Array(Vec<Expr>),
    CellFilter { filter_type: &'static str, arguments: Vec<Expr> },
    Call {
        function: FnCall,  // e.g. ["Duration", "from_millis"]
        args: Vec<Expr>
    },
    OptionSome(Box<Expr>),
    Sequence(Vec<Expr>),
    Parallel(Vec<Expr>),
    Style(Vec<StyleMethod>),
    Fx {
        name: String,
        arguments: Vec<Expr>,
        // cell_filter: Option<CellFilter>,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Value {
    CellFilter(CellFilter),
    Color(Color),
    Style(Style),
    String(String),
    U32(u32),
    F32(f32),
    None,
    Duration(Duration),
    Timer(EffectTimer),
    Motion(Motion),
    Rect(Rect),
    Margin(Margin),
    RepeatMode(RepeatMode),
    Interpolation(Interpolation),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum FnCall {
    ColorRgb,
    ColorIndexed,
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

// Helper enum for method types
#[derive(Debug, Clone, PartialEq)]
pub(super) enum StyleMethod {
    Fg(Expr),
    Bg(Expr),
    AddModifier(Modifier),
    SubModifier(Modifier),
}

pub(super) fn compile_style(methods: Vec<StyleMethod>) -> Style {
    methods.into_iter().fold(Style::new(), |style, method| {
        match method {
            StyleMethod::Fg(Expr::Literal(Value::Color(color))) => style.fg(color),
            StyleMethod::Bg(Expr::Literal(Value::Color(color))) => style.bg(color),
            StyleMethod::AddModifier(modifier) => style.add_modifier(modifier),
            _ => style
        }
    })
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
            Expr::Array(_)          => "array_ref",
            Expr::CellFilter { .. } => "cell_filter",
            Expr::Sequence(_)       => "sequence",
            Expr::Parallel(_)       => "parallel",
            Expr::Style(_)          => "style",
            Expr::OptionSome(_)     => "some",
        }
    }

    pub(super) fn format(&self, indent: usize) -> String {
        let indent_str = " ".repeat(indent);

        match self {
            Expr::Literal(value) => format!("{}{}", indent_str, value.format()),
            Expr::Var(name) => format!("{}{}", indent_str, name),
            Expr::ArrayRef(exprs) => {
                let inner = exprs.iter()
                    .map(|e| e.format(indent + 4))
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("{}&[\n{}\n{}]", indent_str, inner, indent_str)
            },
            Expr::Array(exprs) => {
                let inner = exprs.iter()
                    .map(|e| e.format(indent + 4))
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("{}[\n{}\n{}]", indent_str, inner, indent_str)
            },
            Expr::Fx { name, arguments } => {
                if arguments.is_empty() {
                    format!("{}fx::{}()", indent_str, name)
                } else if arguments.len() == 1 {
                    format!("{}fx::{}({})", indent_str, name, arguments[0].format(indent).trim())
                } else {
                    let args = arguments.iter()
                        .map(|e| e.format(indent + 4))
                        .collect::<Vec<_>>()
                        .join(",\n");

                    format!("{}fx::{}(\n{}\n{})", indent_str, name, args, indent_str)
                }
            },
            Expr::Call { function, args } => {

                let formatted_args = args.iter()
                    .map(|e| e.format(if args.len() == 1 { 0 } else { indent + 4 }))
                    .collect::<Vec<_>>()
                    .join(",\n");

                let (prefix, suffix) = match function {
                    FnCall::ColorRgb            => ("Color::rgb(", ")"),
                    FnCall::ColorIndexed        => ("Color::indexed(", ")"),
                    FnCall::ColorFromU32        => ("Color::from_u32(", ")"),
                    FnCall::DurationFromMillis  => ("Duration::from_millis(", ")"),
                    FnCall::DurationFromSeconds => ("Duration::from_secs(", ")"),
                    FnCall::EffectTimerNew      => ("EffectTimer::new(", ")"),
                    FnCall::EffectTimerFromMs   => ("EffectTimer::from_ms(", ")"),
                    FnCall::RectNew             => ("Rect::new(", ")"),
                    FnCall::RectStruct          => ("Rect {\n", "\n}"),
                    FnCall::MarginNew           => ("Margin::new(", ")"),
                    FnCall::MarginStruct        => ("Margin {\n", "\n}"),
                    FnCall::RepeatModeDuration  => ("RepeatMode::Duration(", ")"),
                    FnCall::RepeatModeTimes     => ("RepeatMode::Times(", ")"),
                };

                if args.len() <= 1 {
                    format!("{}{}{}{}", indent_str, prefix, formatted_args, suffix)
                } else {
                    format!("{}{}\n{}\n{}{}",
                        indent_str, prefix,
                        formatted_args,
                        indent_str, suffix)
                }
            },
            Expr::Sequence(exprs) => {
                let inner = exprs.iter()
                    .map(|e| e.format(indent + 4))
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("{}fx::sequence(&[\n{}\n{}])",
                    indent_str, inner, indent_str)
            },
            Expr::Parallel(exprs) => {
                let inner = exprs.iter()
                    .map(|e| e.format(indent + 4))
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("{}fx::parallel(&[\n{}\n{}])",
                    indent_str, inner, indent_str)
            },
            Expr::CellFilter { .. } => format!("{}// TODO: format cell filter", indent_str),
            Expr::Style(methods) => {
                let inner = methods.iter()
                    .map(|m| match m {
                        StyleMethod::Fg(expr) => format!("{}fg({})", indent_str, expr.format(indent + 4)),
                        StyleMethod::Bg(expr) => format!("{}bg({})", indent_str, expr.format(indent + 4)),
                        StyleMethod::AddModifier(modifier) => format!("{}add_modifier({:?})", indent_str, modifier),
                        StyleMethod::SubModifier(modifier) => format!("{}sub_modifier({:?})", indent_str, modifier),
                    })
                    .collect::<Vec<_>>()
                    .join("\n{indent_str}");

                format!("{}Style::new(){}", indent_str, inner)
            }
            Expr::OptionSome(v) => format!("{}Some({})", indent_str, v.format(0)),
        }
    }
}

impl Value {
    pub(super) fn format(&self) -> String {
        match self {
            Value::Color(c) => {
                let (r, g, b) = c.to_rgb();
                format!("Color::from_u32(0x{:02x}{:02x}{:02x})", r, g, b)
            }
            Value::Duration(d) =>
                format!("Duration::from_millis({})", d.as_millis()),
            Value::Motion(m) =>
                format!("{m:?}"),
            Value::String(s) =>
                format!("\"{}\"", s.replace('"', "\\\"")),
            Value::U32(n) =>
                n.to_string(),
            Value::F32(f) =>
                f.to_string(),
            Value::CellFilter(c) =>
                c.format(),
            Value::Style(_) =>
                todo!("format style"),
            Value::Timer(t) =>
                format!("EffectTimer::from_millis({}, {:?})", t.duration().as_millis(), t.interpolation()),
            Value::Rect(r) =>
                format!("Rect::new({}, {}, {}, {})", r.x, r.y, r.width, r.height),
            Value::Margin(m) =>
                format!("Margin::new({}, {})", m.horizontal, m.vertical),
            Value::RepeatMode(RepeatMode::Duration(d)) =>
                format!("RepeatMode::Duration(Duration::from_millis({}))", d.as_millis()),
            Value::RepeatMode(RepeatMode::Times(n)) =>
                format!("RepeatMode::Times({})", n),
            Value::RepeatMode(RepeatMode::Forever) =>
                "RepeatMode::Forever".to_string(),
            Value::Interpolation(i) =>
                format!("{i:?}"),
            Value::None => "None".to_string(),
        }
    }
}
