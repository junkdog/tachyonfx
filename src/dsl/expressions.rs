use crate::dsl::DslFormat;
use crate::fx::RepeatMode;
use crate::{CellFilter, ColorSpace, Interpolation, Motion};
use compact_str::{format_compact, CompactString, ToCompactString};
use ratatui::layout::Direction;
use ratatui::prelude::{Color, Modifier};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct FnCallInfo {
    pub name: CompactString,
    pub args: Vec<Expr>,
    pub span: ExprSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Expr {
    Literal(Value, ExprSpan),
    Var { name: CompactString, self_fns: Vec<FnCallInfo>, span: ExprSpan },
    LetBinding {
        name: CompactString,
        let_expr: Box<Expr>,
        span: ExprSpan,
    },
    ArrayRef(Vec<Expr>, ExprSpan),
    Array(Vec<Expr>, ExprSpan),
    FnCall { call: FnCallInfo, self_fns: Vec<FnCallInfo> },
    QualifiedMember(CompactString, ExprSpan), // enums, struct fields
    OptionSome(Box<Expr>, ExprSpan),
    Sequence {
        effects: Vec<Expr>,
        self_fns: Vec<FnCallInfo>,
        span: ExprSpan,
    },
    Parallel {
        effects: Vec<Expr>,
        self_fns: Vec<FnCallInfo>,
        span: ExprSpan,
    },
    StructInit {
        name: CompactString,
        fields: Vec<(CompactString, Expr)>,
        span: ExprSpan,
    },
    Tuple(Vec<Expr>, ExprSpan),
    Macro { name: CompactString, args: Vec<Expr>, span: ExprSpan },
    Delimiter { symbol: char, span: ExprSpan }, // discarded after validation
    SyntaxError { message: CompactString, span: ExprSpan },
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Value {
    CellFilter(CellFilter),
    Color(Color),
    Direction(Direction),
    String(CompactString),
    Bool(bool),
    I32(i32),
    U32(u32),
    F32(f32),
    ColorSpace(ColorSpace),
    OptionNone,
    Modifier(Modifier),
    Motion(Motion),
    RepeatMode(RepeatMode),
    Interpolation(Interpolation),
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct ExprSpan {
    pub start: u32,
    pub end: u32,
}

impl ExprSpan {
    pub(super) const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub(super) const fn len(&self) -> u32 {
        self.end - self.start
    }
}

impl FnCallInfo {
    pub fn new(
        name: impl Into<CompactString>,
        args: Vec<Expr>,
        span: ExprSpan
    ) -> Self {
        Self { name: name.into(), args, span }
    }
}

impl From<(&str, Vec<Expr>)> for FnCallInfo {
    fn from((name, args): (&str, Vec<Expr>)) -> Self {
        let (start, end) = (
            args.first().map(|a| a.span().start).unwrap_or(0),
            args.last().map(|a| a.span().end).unwrap_or(0)
        );

        Self { name: name.into(), args, span: ExprSpan::new(start, end) }
    }
}

impl Expr {
    pub(super) fn span(&self) -> ExprSpan {
        *match self {
            Expr::Literal(_, span) => span,
            Expr::Var { span, .. } => span,
            Expr::LetBinding { span, .. } => span,
            Expr::ArrayRef(_, span) => span,
            Expr::Array(_, span) => span,
            Expr::FnCall { call, .. } => &call.span,
            Expr::QualifiedMember(_, span) => span,
            Expr::OptionSome(_, span) => span,
            Expr::Sequence { span, .. } => span,
            Expr::Parallel { span, .. } => span,
            Expr::StructInit { span, .. } => span,
            Expr::Tuple(_, span) => span,
            Expr::Macro { span, .. } => span,
            Expr::Delimiter { span, .. } => span,
            Expr::SyntaxError { span, .. } => span
        }
    }

    /// Returns a string representation of the expression's type
    /// Used for error messages
    pub fn type_name(&self) -> &'static str {
        match self {
            Expr::Var { .. }            => "variable",
            Expr::Literal(v, _)         => v.type_name(),
            Expr::ArrayRef(_, _)        => "array_ref",
            Expr::Array(_, _)           => "array_ref",
            Expr::Sequence { .. }       => "sequence",
            Expr::Parallel { .. }       => "parallel",
            Expr::OptionSome(_, _)      => "some",
            Expr::FnCall { .. }         => "fn_call",
            Expr::LetBinding { .. }     => "let_binding",
            Expr::QualifiedMember(_, _) => "qualified_name",
            Expr::StructInit { .. }     => "struct",
            Expr::Tuple(_, _)           => "tuple",
            Expr::Macro { .. }          => "macro",
            Expr::Delimiter { .. }      => "delimiter",
            Expr::SyntaxError { .. }    => "syntax_error",
        }
    }
}

impl std::fmt::Display for ExprSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}..{})", self.start, self.end)
    }
}

impl Value {
    pub(super) fn format(&self) -> CompactString {
        match self {
            Value::Color(c)         => c.dsl_format(),
            Value::Motion(m)        => m.dsl_format(),
            Value::String(s)        => format_compact!("\"{}\"", s.replace('"', "\\\"")),
            Value::U32(n)           => n.to_compact_string(),
            Value::F32(f)           => f.to_compact_string(),
            Value::I32(i)           => i.to_compact_string(),
            Value::CellFilter(c)    => c.dsl_format(),
            Value::RepeatMode(r)    => r.dsl_format(),
            Value::Interpolation(i) => i.dsl_format(),
            Value::OptionNone       => "None".to_compact_string(),
            Value::Modifier(m)      => m.dsl_format(),
            Value::Direction(dir)   => dir.dsl_format(),
            Value::ColorSpace(c)    => c.dsl_format(),
            Value::Bool(b)          => b.dsl_format(),
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Value::Bool(_)          => "bool",
            Value::CellFilter(_)    => "cell_filter",
            Value::Color(_)         => "color",
            Value::Motion(_)        => "motion",
            Value::String(_)        => "string",
            Value::U32(_)           => "u32",
            Value::F32(_)           => "f32",
            Value::I32(_)           => "i32",
            Value::RepeatMode(_)    => "repeat_mode",
            Value::Interpolation(_) => "interpolation",
            Value::OptionNone       => "option",
            Value::Modifier(_)      => "modifier",
            Value::Direction(_)     => "direction",
            Value::ColorSpace(_)    => "color_space",
        }
    }
}