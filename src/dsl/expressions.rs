use crate::fx::RepeatMode;
use crate::{CellFilter, Duration, EffectTimer, Interpolation, Motion};
use ratatui::layout::{Constraint, Margin, Rect};
use ratatui::prelude::{Color, Modifier, Style};
use crate::color_ext::ToRgbComponents;
use crate::dsl::DslFormat;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct FnCallInfo {
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Expr {
    Literal(Value),
    Var(String),
    ArrayRef(Vec<Expr>),
    Array(Vec<Expr>),
    CellFilter { filter_type: &'static str, arguments: Vec<Expr> },
    FnCall(FnCallInfo), // e.g. foo_bar(area)
    OptionSome(Box<Expr>),
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
        name: String,
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
    Style(Style),
    String(String),
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

// todo: rething; doens't support resolving vars
pub(super) fn compile_style(methods: Vec<FnCallInfo>) -> Style {
    methods.into_iter().fold(Style::new(), |style, method| {
        match method.name.as_str() {
            "fg" => match method.args[0] {
                Expr::Literal(Value::Color(color)) => style.fg(color),
                _ => style
            },
            "bg" => match method.args[0] {
                Expr::Literal(Value::Color(color)) => style.bg(color),
                _ => style
            },
            "add_modifier" => match method.args[0] {
                Expr::Literal(Value::Modifier(modifier)) => style.add_modifier(modifier),
                _ => style
            },
            _ => style
        }
    })
}

impl FnCallInfo {
    pub fn new(
        name: impl Into<String>,
        args: Vec<Expr>
    ) -> Self {
        Self { name: name.into(), args }
    }
}

impl From<(String, Vec<Expr>)> for FnCallInfo {
    fn from((name, args): (String, Vec<Expr>)) -> Self {
        Self { name, args }
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
        }
    }

    pub(super) fn format(&self, indent: usize) -> String {
        let indent_str = " ".repeat(indent);

        let formatted_args = |args: &[Expr]| args.iter()
            .map(|e| e.format(if args.len() == 1 { 0 } else { indent + 4 }))
            .collect::<Vec<_>>()
            .join(",\n");

        let chained_fns = |self_fns: &[FnCallInfo]| self_fns.iter()
            .map(|fn_call| {
                let args = formatted_args(&fn_call.args);
                format!("\n{indent_str}.{}({args})", fn_call.name)
            })
            .collect::<Vec<_>>()
            .join("");

        match self {
            Expr::Literal(value) => format!("{}{}", indent_str, value.format()),
            Expr::Var(name) => format!("{}{}", indent_str, name),
            Expr::ArrayRef(exprs) => {
                let inner = formatted_args(exprs);
                format!("{}&[\n{}\n{}]", indent_str, inner, indent_str)
            },
            Expr::Array(exprs) => {
                let inner = formatted_args(exprs);
                format!("{}[\n{}\n{}]", indent_str, inner, indent_str)
            },
            Expr::Fx { name, arguments, self_fns } => {
                let effect = if arguments.is_empty() {
                    format!("{}fx::{}()", indent_str, name)
                } else if arguments.len() == 1 {
                    format!("{}fx::{}({})", indent_str, name, arguments[0].format(indent).trim())
                } else {
                    let args = formatted_args(arguments);
                    format!("{}fx::{}(\n{}\n{})", indent_str, name, args, indent_str)
                };

                format!("{effect}{}", chained_fns(self_fns))
            },
            Expr::FnCall(FnCallInfo { name, args }) => {
                let formatted_args = formatted_args(args);

                if args.len() <= 1 {
                    format!("{}{}({})", indent_str, name, formatted_args)
                } else {
                    format!("{}{}(\n{}\n{})", indent_str, name, formatted_args, indent_str)
                }
            },
            Expr::Sequence { effects, self_fns } => {
                format!(
                    "{indent_str}fx::sequence(&[\n{}\n{indent_str}]){}",
                    formatted_args(effects),
                    chained_fns(self_fns),
                )
            },
            Expr::Parallel { effects, self_fns } => {
                format!(
                    "{indent_str}fx::parallel(&[\n{}\n{indent_str}]){}",
                    formatted_args(effects),
                    chained_fns(self_fns),
                )
            },
            Expr::CellFilter { .. } => format!("{}// TODO: format cell filter", indent_str),
            Expr::Style(methods) => {
                let inner = methods.iter()
                    .map(|f| {
                        let args = formatted_args(&f.args);
                        format!("\n{indent_str}.{}({args})", f.name)
                    })
                    .collect::<Vec<_>>()
                    .join("\n{indent_str}");

                format!("{}Style::new(){}", indent_str, inner)
            }
            Expr::OptionSome(v) => format!("{}Some({})", indent_str, v.format(0)),
            Expr::InvalidExpr { remaining: input } => format!("{}// Invalid expression: {}", indent_str, input),
            Expr::ParserError(message) => format!("{}// Parser error: {}", indent_str, message), // ?
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
            Value::OptionNone => "None".to_string(),
            Value::Modifier(m) => m.dsl_format(),
            Value::Constraint(c) => c.to_string(),
        }
    }
}
