use compact_str::{format_compact, CompactString, CompactStringExt, ToCompactString};
use crate::fx::RepeatMode;
use crate::{CellFilter, Duration, EffectTimer, Interpolation, Motion};
use ratatui::layout::{Constraint, Direction, Margin, Rect};
use ratatui::prelude::{Color, Modifier, Style};
use crate::dsl::DslFormat;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct FnCallInfo {
    pub name: CompactString,
    pub args: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Expr {
    Literal(Value),
    Var { name: CompactString, self_fns: Vec<FnCallInfo> },
    LetBinding {
        name: CompactString,
        let_expr: Box<Expr>,
    },
    ArrayRef(Vec<Expr>),
    Array(Vec<Expr>),
    CellFilter { filter_type: &'static str, arguments: Vec<Expr> },
    FnCall { call: FnCallInfo, self_fns: Vec<FnCallInfo> }, // e.g. foo_bar(area)
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
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Value {
    CellFilter(CellFilter),
    Color(Color),
    Constraint(Constraint),
    Direction(Direction),
    Style(Style),
    String(CompactString),
    I32(i32),
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
            Expr::Var { .. }         => "variable",
            Expr::Fx { .. }          => "effect",
            Expr::Literal(v)         => v.type_name(),
            Expr::ArrayRef(_)        => "array_ref",
            Expr::Array(_)           => "array_ref",
            Expr::CellFilter { .. }  => "cell_filter",
            Expr::Sequence { .. }    => "sequence",
            Expr::Parallel { .. }    => "parallel",
            Expr::Style(_)           => "style",
            Expr::OptionSome(_)      => "some",
            Expr::FnCall { .. }      => "fn_call",
            Expr::Layout { .. }      => "layout",
            Expr::LetBinding { .. }  => "let_binding",
        }
    }

    // this is a ugly mess; but stays so until the tokenizaton/parser rework
    pub(super) fn format(&self, indent: usize, indent_arg: bool) -> CompactString {
        let indent_str = " ".repeat(indent);
        let prefix = if indent_arg { &indent_str } else { "" };

        // Determine if an expression is a "simple" value that can be rendered inline
        let is_simple_expr = |expr: &Expr| -> bool {
            match expr {
                Expr::Literal(Value::U32(_)) |
                Expr::Literal(Value::I32(_)) |
                Expr::Literal(Value::F32(_)) |
                Expr::Literal(Value::String(_)) |
                Expr::Literal(Value::Interpolation(_)) |
                Expr::Literal(Value::Motion(_)) => true,
                Expr::Var { self_fns, .. } if self_fns.len() < 2 => true,
                Expr::Fx { arguments, self_fns, .. } if arguments.len() < 2 && self_fns.is_empty() => true,
                // also consider function calls with no args or simple args to be simple
                Expr::FnCall { call, self_fns } =>
                    self_fns.is_empty() && call.args.len() < 2,
                _ => false,
            }
        };

        // Format arguments with appropriate indentation
        let formatted_args = |args: &[Expr]| {
            // If just one argument and it's simple, render it inline
            if args.len() == 1 && is_simple_expr(&args[0]) {
                args[0].format(indent, false)
            } else if args.is_empty() {
                "".to_compact_string()
            } else {
                args.iter()
                    .map(|e| e.format(indent + 4, args.len() > 1))
                    .collect::<Vec<_>>()
                    .join_compact(",\n")
            }
        };

        let chained_fns = |self_fns: &[FnCallInfo]| self_fns.iter()
            .map(|fn_call| {
                let args = formatted_args(&fn_call.args);
                format_compact!("\n{prefix}.{}({args})", fn_call.name)
            })
            .collect::<Vec<_>>()
            .join_compact("");

        match self {
            Expr::Literal(value) => format_compact!("{}{}", prefix, value.format()),
            Expr::Var { name, self_fns } =>
                format_compact!("{}{}{}", prefix, name, chained_fns(self_fns)),
            Expr::ArrayRef(exprs) => {
                let inner = formatted_args(exprs);
                format_compact!("{}&[\n{}\n{}]", prefix, inner, indent_str)
            },
            Expr::Array(exprs) => {
                let inner = formatted_args(exprs);
                format_compact!("{}[\n{}\n{}]", prefix, inner, indent_str)
            },
            Expr::Fx { name, arguments, self_fns } => {
                let effect = if arguments.is_empty() {
                    format_compact!("{}fx::{}()", prefix, name)
                } else if arguments.len() == 1 {
                    // check if it's a simple expression or another effect that should be rendered inline
                    let arg = &arguments[0];
                    if is_simple_expr(arg) || matches!(arg, Expr::Fx { .. } | Expr::FnCall { .. }) {
                        let arg_str = arg.format(indent, false);
                        format_compact!("{prefix}fx::{name}({})", arg_str.trim())
                    } else {
                        let args = formatted_args(arguments);
                        format_compact!("{}fx::{}(\n{}\n{})", prefix, name, args, indent_str)
                    }
                } else {
                    let args = formatted_args(arguments);
                    format_compact!("{}fx::{}(\n{}\n{})", prefix, name, args, indent_str)
                };

                format_compact!("{effect}{}", chained_fns(self_fns))
            },
            Expr::FnCall { call: FnCallInfo { name, args }, self_fns } => {
                let fn_call = if args.is_empty() {
                    format_compact!("{}{}()", prefix, name)
                } else if args.len() == 1 && is_simple_expr(&args[0]) {
                    // for simple single arguments, keep them on the same line
                    format_compact!("{}{}({})", prefix, name, args[0].format(indent, false).trim())
                } else {
                    // for multiple or complex arguments, use line breaks with indentation
                    let args_str = formatted_args(args);
                    format_compact!("{}{}(\n{}\n{})", prefix, name, args_str, indent_str)
                };

                // Append chained function calls
                let chains = self_fns.iter()
                    .map(|fn_call| {
                        let args = formatted_args(&fn_call.args);
                        format_compact!("\n{}.{}({})", indent_str, fn_call.name, args)
                    })
                    .collect::<Vec<_>>()
                    .join_compact("");

                format_compact!("{}{}", fn_call, chains)
            },
            Expr::LetBinding { name, let_expr} => format_compact!(
                "{indent_str}let {name} = {value}",
                name = name,
                value = let_expr.format(indent, false),
            ),
            Expr::Sequence { effects, self_fns } => format_compact!(
                "{indent_str}fx::sequence(&[\n{}\n{indent_str}]){}",
                formatted_args(effects),
                chained_fns(self_fns),
            ),
            Expr::Parallel { effects, self_fns } => format_compact!(
                "{indent_str}fx::parallel(&[\n{}\n{indent_str}]){}",
                formatted_args(effects),
                chained_fns(self_fns),
            ),
            Expr::CellFilter { filter_type, arguments } => {
                let args = formatted_args(arguments);
                format_compact!("{}CellFilter::{filter_type}({args})", indent_str)
            },
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
            Expr::OptionSome(v) => format_compact!("{}Some({})", indent_str, v.format(indent, false)),
            Expr::Layout { .. } => "layout(todo)".to_compact_string()
        }
    }
}

impl Value {
    pub(super) fn format(&self) -> CompactString {
        match self {
            Value::Color(c)         => c.dsl_format(),
            Value::Duration(d)      => d.dsl_format(),
            Value::Motion(m)        => m.dsl_format(),
            Value::String(s)        => format_compact!("\"{}\"", s.replace('"', "\\\"")),
            Value::U32(n)           => n.to_compact_string(),
            Value::F32(f)           => f.to_compact_string(),
            Value::I32(i)           => i.to_compact_string(),
            Value::CellFilter(c)    => c.dsl_format(),
            Value::Style(s)         => s.dsl_format(),
            Value::Timer(t)         => t.dsl_format(),
            Value::Rect(r)          => r.dsl_format(),
            Value::Margin(m)        => m.dsl_format(),
            Value::RepeatMode(r)    => r.dsl_format(),
            Value::Interpolation(i) => i.dsl_format(),
            Value::OptionNone       => "None".to_compact_string(),
            Value::Modifier(m)      => m.dsl_format(),
            Value::Constraint(c)    => c.dsl_format(),
            Value::Direction(dir)   => dir.dsl_format(),
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Value::CellFilter(_) => "cell_filter",
            Value::Color(_)       => "color",
            Value::Duration(_)    => "duration",
            Value::Motion(_)      => "motion",
            Value::String(_)      => "string",
            Value::U32(_)         => "u32",
            Value::F32(_)         => "f32",
            Value::I32(_)         => "i32",
            Value::Style(_)       => "style",
            Value::Timer(_)       => "timer",
            Value::Rect(_)        => "rect",
            Value::Margin(_)      => "margin",
            Value::RepeatMode(_)  => "repeat_mode",
            Value::Interpolation(_)=> "interpolation",
            Value::OptionNone     => "option",
            Value::Modifier(_)     => "modifier",
            Value::Constraint(_)  => "constraint",
            Value::Direction(_)   => "direction",
        }
    }
}