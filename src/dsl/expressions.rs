use compact_str::{format_compact, CompactString, CompactStringExt, ToCompactString};
use crate::fx::RepeatMode;
use crate::{CellFilter, Duration, EffectTimer, Interpolation, Motion};
use ratatui::layout::{Constraint, Direction, Margin, Rect};
use ratatui::prelude::{Color, Modifier, Style};
use crate::color_ext::ToRgbComponents;
use crate::dsl::DslFormat;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct FnCallInfo {
    pub name: CompactString,
    pub args: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Expr {
    Literal(Value),
    Var(CompactString),
    ArrayRef(Vec<Expr>),
    Array(Vec<Expr>),
    CellFilter { filter_type: &'static str, arguments: Vec<Expr> },
    FnCall(FnCallInfo), // e.g. foo_bar(area)
    OptionSome(Box<Expr>),
    Layout { expr: Box<Expr>, self_fns: Vec<FnCallInfo> },
    Sequence {
        effects: Vec<Expr>,
        self_fns: Vec<FnCallInfo>
    },
    Parallel {
        effects: Vec<Expr>,
        self_fns: Vec<FnCallInfo>
    },
    Style(Vec<FnCallInfo>),
    Fx {
        name: CompactString,
        arguments: Vec<Expr>,
        self_fns: Vec<FnCallInfo>,
    },
    InvalidExpr { remaining: String },
    ParserError(String),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Value {
    CellFilter(CellFilter),
    Color(Color),
    Constraint(Constraint),
    Direction(Direction),
    Style(Style),
    String(CompactString),
    U32(u32),
    F32(f32),
    OptionNone,
    Duration(Duration),
    Timer(EffectTimer),
    Modifier(Modifier),
    Motion(Motion),
    Rect(Rect),
    Margin(Margin),
    RepeatMode(RepeatMode),
    Interpolation(Interpolation),
}

impl FnCallInfo {
    pub fn new(
        name: impl Into<CompactString>,
        args: Vec<Expr>
    ) -> Self {
        Self { name: name.into(), args }
    }
}

impl From<(&str, Vec<Expr>)> for FnCallInfo {
    fn from((name, args): (&str, Vec<Expr>)) -> Self {
        Self { name: name.into(), args }
    }
}

impl Expr {
    /// Returns a string representation of the expression's type
    /// Used for error messages
    pub fn type_name(&self) -> &'static str {
        match self {
            Expr::Var(_)             => "variable",
            Expr::Fx { .. }          => "effect",
            Expr::Literal(_)         => "literal",
            Expr::ArrayRef(_)        => "array_ref",
            Expr::Array(_)           => "array_ref",
            Expr::CellFilter { .. }  => "cell_filter",
            Expr::Sequence { .. }    => "sequence",
            Expr::Parallel { .. }    => "parallel",
            Expr::Style(_)           => "style",
            Expr::OptionSome(_)      => "some",
            Expr::FnCall { .. }      => "fn_call",
            Expr::InvalidExpr { .. } => "invalid",
            Expr::ParserError(_)     => "parser_error",
            Expr::Layout { .. }      => "layout"
        }
    }

    pub(super) fn format(&self, indent: usize) -> CompactString {
        let indent_str = " ".repeat(indent);

        let formatted_args = |args: &[Expr]| args.iter()
            .map(|e| e.format(if args.len() == 1 { 0 } else { indent + 4 }))
            .collect::<Vec<_>>()
            .join_compact(",\n");

        let chained_fns = |self_fns: &[FnCallInfo]| self_fns.iter()
            .map(|fn_call| {
                let args = formatted_args(&fn_call.args);
                format_compact!("\n{indent_str}.{}({args})", fn_call.name)
            })
            .collect::<Vec<_>>()
            .join_compact("");

        match self {
            Expr::Literal(value) => format_compact!("{}{}", indent_str, value.format()),
            Expr::Var(name) => format_compact!("{}{}", indent_str, name),
            Expr::ArrayRef(exprs) => {
                let inner = formatted_args(exprs);
                format_compact!("{}&[\n{}\n{}]", indent_str, inner, indent_str)
            },
            Expr::Array(exprs) => {
                let inner = formatted_args(exprs);
                format_compact!("{}[\n{}\n{}]", indent_str, inner, indent_str)
            },
            Expr::Fx { name, arguments, self_fns } => {
                let effect = if arguments.is_empty() {
                    format_compact!("{}fx::{}()", indent_str, name)
                } else if arguments.len() == 1 {
                    format_compact!("{}fx::{}({})", indent_str, name, arguments[0].format(indent).trim())
                } else {
                    let args = formatted_args(arguments);
                    format_compact!("{}fx::{}(\n{}\n{})", indent_str, name, args, indent_str)
                };

                format_compact!("{effect}{}", chained_fns(self_fns))
            },
            Expr::FnCall(FnCallInfo { name, args }) => {
                let formatted_args = formatted_args(args);

                if args.len() <= 1 {
                    format_compact!("{}{}({})", indent_str, name, formatted_args)
                } else {
                    format_compact!("{}{}(\n{}\n{})", indent_str, name, formatted_args, indent_str)
                }
            },
            Expr::Sequence { effects, self_fns } => {
                format_compact!(
                    "{indent_str}fx::sequence(&[\n{}\n{indent_str}]){}",
                    formatted_args(effects),
                    chained_fns(self_fns),
                )
            },
            Expr::Parallel { effects, self_fns } => {
                format_compact!(
                    "{indent_str}fx::parallel(&[\n{}\n{indent_str}]){}",
                    formatted_args(effects),
                    chained_fns(self_fns),
                )
            },
            Expr::CellFilter { .. } => format_compact!("{}// TODO: format cell filter", indent_str),
            Expr::Style(methods) => {
                let inner = methods.iter()
                    .map(|f| {
                        let args = formatted_args(&f.args);
                        format!("\n{indent_str}.{}({args})", f.name)
                    })
                    .collect::<Vec<_>>()
                    .join_compact("\n{indent_str}");

                format_compact!("{}Style::new(){}", indent_str, inner)
            }
            Expr::OptionSome(v) => format_compact!("{}Some({})", indent_str, v.format(0)),
            Expr::InvalidExpr { remaining: input } => format_compact!("{}// Invalid expression: {}", indent_str, input),
            Expr::ParserError(message) => format_compact!("{}// Parser error: {}", indent_str, message), // ?
            Expr::Layout { .. } => "layout(todo)".to_compact_string()
        }
    }
}

impl Value {
    pub(super) fn format(&self) -> CompactString {
        match self {
            Value::Color(c) => {
                let (r, g, b) = c.to_rgb();
                format_compact!("Color::from_u32(0x{:02x}{:02x}{:02x})", r, g, b)
            }
            Value::Duration(d) =>
                format_compact!("Duration::from_millis({})", d.as_millis()),
            Value::Motion(m) =>
                format_compact!("{m:?}"),
            Value::String(s) =>
                format_compact!("\"{}\"", s.replace('"', "\\\"")),
            Value::U32(n) =>
                n.to_compact_string(),
            Value::F32(f) =>
                f.to_compact_string(),
            Value::CellFilter(c) =>
                c.format(),
            Value::Style(_) =>
                todo!("format style"),
            Value::Timer(t) =>
                format_compact!("EffectTimer::from_millis({}, {:?})", t.duration().as_millis(), t.interpolation()),
            Value::Rect(r) =>
                format_compact!("Rect::new({}, {}, {}, {})", r.x, r.y, r.width, r.height),
            Value::Margin(m) =>
                format_compact!("Margin::new({}, {})", m.horizontal, m.vertical),
            Value::RepeatMode(RepeatMode::Duration(d)) =>
                format_compact!("RepeatMode::Duration(Duration::from_millis({}))", d.as_millis()),
            Value::RepeatMode(RepeatMode::Times(n)) =>
                format_compact!("RepeatMode::Times({})", n),
            Value::RepeatMode(RepeatMode::Forever) =>
                "RepeatMode::Forever".to_compact_string(),
            Value::Interpolation(i) =>
                format_compact!("{i:?}"),
            Value::OptionNone => "None".to_compact_string(),
            Value::Modifier(m) => m.dsl_format(),
            Value::Constraint(c) => c.to_compact_string(),
            Value::Direction(dir) => dir.to_compact_string(),
        }
    }
}
