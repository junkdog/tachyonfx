use crate::dsl::dsl::EffectDsl;
use crate::dsl::environment::DslEnv;
use crate::dsl::expressions::{Expr, ExprSpan, FnCallInfo, Value};
use crate::dsl::method_chains::ChainableMethods;
use crate::dsl::DslError;
use crate::fx::RepeatMode;
use crate::{CellFilter, ColorSpace, Duration, Effect, EffectTimer, Interpolation, Motion};
use compact_str::{CompactString, ToCompactString};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Offset, Rect};
use ratatui::prelude::{Color, Style};
use ratatui::style::Modifier;
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::fmt::Formatter;

/// A helper struct for parsing arguments when implementing custom effect compilers.
///
/// `Arguments` is primarily used when registering custom effects with [`EffectDsl`].
/// It provides methods to safely extract and validate typed values from DSL expressions.
///
/// # Example
///
/// ```
/// use tachyonfx::dsl::{EffectDsl, DslError};
/// use tachyonfx::{Effect, Duration, fx};
///
/// // re-registering `sweep_in` under the name `sweep_in_dup`, this
/// // would typically be supplanted by a custom effect implementation.
/// let dsl = EffectDsl::new()
///     .register("sweep_in_dup", |args| {
///         Ok(fx::sweep_in(
///             args.motion()?,
///             args.read_u16()?,
///             args.read_u16()?,
///             args.color()?,
///             args.effect_timer()?
///         ))
///     });
/// ```
#[derive(Debug)]
pub struct Arguments<'dsl> {
    args: VecDeque<Expr>,
    span: ExprSpan,
    vars: &'dsl DslEnv,
    context: &'dsl EffectDsl,
    initial_arg_count: usize,
}

impl<'dsl> Arguments<'dsl> {
    pub(super) fn new(
        args: VecDeque<Expr>,
        context: &'dsl EffectDsl,
        vars: &'dsl DslEnv,
        fallback_span: ExprSpan,
    ) -> Self {
        let initial_arg_count = args.len();
        let mut span = args.front().map_or_else(|| fallback_span, |e| e.span());
        span.end = args.back().map_or(span.end, |e| e.span().end);
        Self { args, span, vars, context, initial_arg_count }
    }

    pub(super) fn remaining_args(&self) -> &VecDeque<Expr> {
        &self.args
    }

    pub(super) fn remaining_arg_count(&self) -> usize {
        self.args.len()
    }

    /// Consumes the next argument and returns a [`Duration`].
    pub fn duration(&mut self) -> Result<Duration, DslError> {
        match self.next("duration")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, .. } => Ok(match name.as_str() {
                "Duration::from_millis" => {
                    let ms = self.extract_nested(args, Arguments::read_u32, span)?;
                    Duration::from_millis(ms as _)
                },
                "Duration::from_secs_f32" => {
                    let seconds = self.extract_nested(args, Arguments::read_f32, span)?;
                    Duration::from_secs_f32(seconds)
                },
                _ => self.expected_type("duration", name, span)?,
            }),
            Expr::Literal(Value::U32(ms), _)     => Ok(Duration::from_millis(ms as _)),
            Expr::Literal(v, span)               => self.expected_type("duration", v.format(), span),
            Expr::Var { name, span, .. }         => self.bound_var(name, span),
            e                                    => self.expected_type_expr("duration", e),
        }
    }

    /// Consumes the next argument and returns an [`EffectTimer`].
    pub fn effect_timer(&mut self) -> Result<EffectTimer, DslError> {
        match self.next("timer")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, .. } => Ok(match name.as_str() {
                "EffectTimer::from_ms" => {
                    let mut inner_args = self.nested_args(args, 2, span)?;
                    let ms = inner_args.read_u32()?;
                    let interpolation = inner_args.interpolation()?;
                    EffectTimer::from_ms(ms, interpolation)
                },
                "EffectTimer::new" => {
                    let mut inner_args = self.nested_args(args, 2, span)?;
                    let duration = inner_args.duration()?;
                    let interpolation = inner_args.interpolation()?;
                    EffectTimer::new(duration, interpolation)
                },
                _ => self.expected_type("timer", name, span)?,
            }),
            Expr::Literal(Value::U32(ms), _)   => Ok(ms.into()),
            Expr::Tuple(exprs, span) => {
                let mut args = self.nested_args(exprs, 2, span)?;
                let duration = args.duration()?;
                let interpolation = args.interpolation()?;
                Ok(EffectTimer::new(duration, interpolation))
            },
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e                            => self.expected_type_expr("timer", e),
        }
    }

    /// Consumes the next argument and returns a [`Color`].
    pub fn cell_filter(&mut self) -> Result<CellFilter, DslError> {
        match self.next("cell_filter")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, .. } => {
                let filter_type = name.trim_start_matches("CellFilter::");
                let mut inner_args = Arguments::new(args.into(), self.context, self.vars, span);

                match filter_type {
                    "Area"       => Ok(CellFilter::Area(inner_args.rect()?)),
                    "FgColor"    => Ok(CellFilter::FgColor(inner_args.color()?)),
                    "BgColor"    => Ok(CellFilter::BgColor(inner_args.color()?)),
                    "Inner"      => Ok(CellFilter::Inner(inner_args.margin()?)),
                    "Outer"      => Ok(CellFilter::Outer(inner_args.margin()?)),
                    "AllOf"      => Ok(CellFilter::AllOf(inner_args.array(Arguments::cell_filter)?)),
                    "AnyOf"      => Ok(CellFilter::AnyOf(inner_args.array(Arguments::cell_filter)?)),
                    "NoneOf"     => Ok(CellFilter::NoneOf(inner_args.array(Arguments::cell_filter)?)),
                    "Not"        => Ok(CellFilter::Not(inner_args.boxed(Arguments::cell_filter, span)?)),
                    "Layout"     => Ok(CellFilter::Layout(inner_args.layout()?, inner_args.read_u16()?)),
                    "PositionFn" => Ok(CellFilter::PositionFn(inner_args.any_var()?)),
                    "EvalCell"   => Ok(CellFilter::EvalCell(inner_args.any_var()?)),
                    e            => Err(DslError::UnknownCellFilter {
                        name: e.to_compact_string(),
                        location: span,
                    })?,
                }
            }
            Expr::Literal(Value::CellFilter(f), _) => Ok(f),
            Expr::Var { name, span, .. }           => self.bound_var(name, span),
            e                                      => self.expected_type_expr("cell_filter", e),
        }
    }

    pub fn color_space(&mut self) -> Result<ColorSpace, DslError> {
        match self.next("color_space")? {
            Expr::Literal(Value::ColorSpace(c), _) => Ok(c),
            Expr::Var { name, span, .. }           => self.bound_var(name, span),
            e                                      => self.expected_type_expr("color_space", e),
        }
    }

    /// Consumes the next argument and returns a `T`.
    pub fn any_var<T: Clone + 'static>(
        &mut self,
    ) -> Result<T, DslError> {
        match self.next("var")? {
            Expr::Var { name, span, .. } => self.vars.bound_global(name, span),
            e                            => self.expected_type_expr("var", e),
        }
    }

    /// Consumes the next argument and returns a [`Constraint`].
    pub fn constraint(&mut self) -> Result<Constraint, DslError> {
        use Constraint::*;

        match self.next("constraint")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, .. } =>
                Ok(match name.trim_start_matches("Constraint::") {
                    "Min"        => Min(self.extract_nested(args, Arguments::read_u16, span)?),
                    "Max"        => Max(self.extract_nested(args, Arguments::read_u16, span)?),
                    "Length"     => Length(self.extract_nested(args, Arguments::read_u16, span)?),
                    "Percentage" => Percentage(self.extract_nested(args, Arguments::read_u16, span)?),
                    "Fill"       => Fill(self.extract_nested(args, Arguments::read_u16, span)?),
                    "Ratio" => {
                        let mut inner_args = self.nested_args(args, 2, span)?;
                        let a = inner_args.read_u32()?;
                        let b = inner_args.read_u32()?;
                        Ratio(a, b)
                    },
                    _ => self.expected_type("constraint", name, span)?,
                }),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("constraint", e),
        }
    }

    /// Consumes the next argument and returns a [`Direction`].
    pub fn direction(&mut self) -> Result<Direction, DslError> {
        match self.next("direction")? {
            Expr::Literal(Value::Direction(d), _) => Ok(d),
            Expr::Var { name, span, .. }          => self.bound_var(name, span),
            e => self.expected_type_expr("direction", e),
        }
    }

    /// Consumes the next argument and returns a [`Layout`].
    pub fn layout(&mut self) -> Result<Layout, DslError> {
        match self.next("layout")? {
            Expr::FnCall { call, self_fns } => {
                let base_layout = match call.name.as_str() {
                    "Layout::horizontal" => {
                        let constraints = self.extract_nested(
                            call.args, |a| a.array(Arguments::constraint), call.span)?;
                        Ok(Layout::horizontal(constraints))
                    },
                    "Layout::vertical" => {
                        let constraints = self.extract_nested(
                            call.args, |a| a.array(Arguments::constraint), call.span)?;
                        Ok(Layout::vertical(constraints))
                    },
                    "Layout::new" => {
                        let mut inner_args = self.nested_args(call.args, 2, call.span)?;
                        let direction = inner_args.direction()?;
                        let constraints = inner_args.array(Arguments::constraint)?;
                        Ok(Layout::new(direction, constraints))
                    },
                    _ => self.expected_type("layout", call.name.to_compact_string(), call.span),
                }?;

                // Apply method chains
                base_layout.fold_fns(self_fns, self.context, self.vars)
            },

            Expr::Var { name, self_fns, span } => self.bound_var::<Layout>(name, span)?
                .fold_fns(self_fns, self.context, self.vars),

            e => self.expected_type_expr("layout", e),
        }
    }

    /// Consumes the next argument and returns a [`Interpolation`].
    pub fn interpolation(&mut self) -> Result<Interpolation, DslError> {
        match self.next("interpolation")? {
            Expr::Literal(Value::Interpolation(i), _) => Ok(i),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e                            => self.expected_type_expr("interpolation", e),
        }
    }
    
    /// Consumes the next argument and returns a `bool`.
    pub fn read_bool(&mut self) -> Result<bool, DslError> {
        match self.next("bool")? {
            Expr::Literal(Value::Bool(v), _) => Ok(v),
            Expr::Var { name, span, .. }     => self.bound_var(name, span),
            e                                => self.expected_type_expr("bool", e),
        }
    }

    /// Consumes the next argument and returns a `u8`.
    pub fn read_u8(&mut self) -> Result<u8, DslError> {
        let span = self.peek().map(|expr| expr.span());
        u8::try_from(self.read_u32()?)
            .map_err(|_| DslError::CastOverflow {
                location: span.unwrap(),
                from: "u32",
                to: "u8",
            })
    }

    /// Consumes the next argument and returns a `u16`.
    pub fn read_u16(&mut self) -> Result<u16, DslError> {
        let span = self.peek().map(|expr| expr.span());
        u16::try_from(self.read_u32()?)
            .map_err(|_| DslError::CastOverflow {
                location: span.unwrap(),
                from: "u32",
                to: "u16",
            })
    }

    /// Consumes the next argument and returns a `u32`.
    pub fn read_u32(&mut self) -> Result<u32, DslError> {
        match self.next("u32")? {
            Expr::Literal(Value::U32(u), _) => Ok(u),
            Expr::Var { name, span, .. }    => self.bound_var(name, span),
            e                               => self.expected_type_expr("u32", e),
        }
    }
    
    pub fn read_i32(&mut self) -> Result<i32, DslError> {
        match self.next("i32")? {
            Expr::Literal(Value::I32(i), _) => Ok(i),
            Expr::Literal(Value::U32(i), _) => Ok(i as _),
            Expr::Var { name, span, .. }    => self.bound_var(name, span),
            e                               => self.expected_type_expr("i32", e),
        }
    }

    /// Consumes the next argument and returns a `f32`.
    pub fn read_into_f32(&mut self) -> Result<f32, DslError> {
        match self.next("f32")? {
            Expr::Literal(Value::F32(f), _) => Ok(f),
            Expr::Literal(Value::U32(v), _) => Ok(v as f32),
            Expr::Var { name, span, .. }    => self.bound_var(name, span),
            e                               => self.expected_type_expr("f32", e),
        }
    }

    /// Consumes the next argument and returns a `f32`.
    pub fn read_f32(&mut self) -> Result<f32, DslError> {
        match self.next("f32")? {
            Expr::Literal(Value::F32(f), _) => Ok(f),
            Expr::Var { name, span, .. }    => self.bound_var(name, span),
            e                               => self.expected_type_expr("f32", e),
        }
    }

    /// Consumes the next argument and returns a [`CompactString`].
    pub fn string(&mut self) -> Result<CompactString, DslError> {
        match self.next("string")? {
            Expr::Literal(Value::String(s), _) => Ok(s),
            Expr::Var { name, span, .. }       => self.bound_var(name, span),
            e                                  => self.expected_type_expr("string", e),
        }
    }

    /// Consumes the next argument and returns an `Option<T>`.
    #[allow(private_bounds)]
    pub fn option<T: Clone + FromDslExpr + 'static>(
        &mut self,
        inner: impl Fn(&mut Self) -> Result<T, DslError>
    ) -> Result<Option<T>, DslError> {
        match self.next("option")? {
            Expr::Literal(Value::OptionNone, _) => Ok(None),
            Expr::OptionSome(expr, span)        => {
                let mut args = self.nested_args(vec![*expr], 1, span)?;
                inner(&mut args).map(Some)
            },
            Expr::Var { name, span, .. }         => self.bound_var(name, span),
            e                      => self.expected_type_expr("option", e),
        }
    }

    /// Consumes the next argument and returns an [`Effect`].
    pub fn effect(&mut self) -> Result<Effect, DslError> {
        match self.next("effect")? {
            Expr::FnCall { call, self_fns } => {
                // Check if it's an effect constructor with "fx::" prefix
                let fx_name = call.name.strip_prefix("fx::").unwrap_or(&call.name);

                    // This is a dedicated effect constructor
                    let fx_expr = Expr::FnCall {
                        call: FnCallInfo {
                            name: fx_name.to_compact_string(),
                            args: call.args,
                            span: call.span,
                        },
                        self_fns,
                    };
                    self.compile_effect(fx_expr)
            },
            Expr::Sequence { effects, self_fns, span } =>
                self.compile_effect(Expr::Sequence { effects, self_fns, span }),
            Expr::Parallel { effects, self_fns, span } =>
                self.compile_effect(Expr::Parallel { effects, self_fns, span }),

            Expr::Var { name, self_fns, span } => self.bound_var::<Effect>(name, span)?
                .fold_fns(self_fns, self.context, self.vars),

            e => self.expected_type_expr("effect", e),
        }
    }

    /// Consumes the next argument and returns a [`Color`].
    pub fn color(&mut self) -> Result<Color, DslError> {
        match self.next("color")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, .. } => Ok(match name.as_str() {
                "Color::Rgb" => {
                    let mut inner_args = self.nested_args(args, 3, span)?;
                    let r = inner_args.read_u8()?;
                    let g = inner_args.read_u8()?;
                    let b = inner_args.read_u8()?;
                    Color::Rgb(r, g, b)
                },
                "Color::from_u32" => {
                    Color::from_u32(self.extract_nested(args, Arguments::read_u32, span)?)
                }
                "Color::Indexed" => {
                    Color::Indexed(self.extract_nested(args, Arguments::read_u8, span)?)
                }
                _ => self.expected_type("color", name, span)?,
            }),
            Expr::Literal(Value::Color(c), _) => Ok(c),
            Expr::Var { name, span, .. }      => self.bound_var(name, span),
            e                                 => self.expected_type_expr("color", e),
        }
    }

    /// Consumes the next argument and returns a [`Modifier`].
    pub fn modifier(&mut self) -> Result<Modifier, DslError> {
        match self.next("modifier")? {
            Expr::Literal(Value::Modifier(m), _) => Ok(m),
            Expr::Var { name, span, .. }         => self.bound_var(name, span),
            e                                    => self.expected_type_expr("modifier", e),
        }
    }

    /// Consumes the next argument and returns a [`Style`].
    pub fn style(&mut self) -> Result<Style, DslError> {
        match self.next("style")? {
            Expr::FnCall { call, self_fns } => {
                if call.name == "Style::new" || call.name == "Style::default" {
                    Style::new().fold_fns(self_fns, self.context, self.vars)
                } else {
                    self.expected_type("style", call.name.to_compact_string(), call.span)?
                }
            },
            Expr::Var { name, self_fns, span } => self.bound_var::<Style>(name, span)?
                .fold_fns(self_fns, self.context, self.vars),
            e                              => self.expected_type_expr("style", e),
        }
    }

    /// Consumes the next argument and returns a [`Motion`].
    pub fn motion(&mut self) -> Result<Motion, DslError> {
        match self.next("motion")? {
            Expr::Literal(Value::Motion(m), _) => Ok(m),
            Expr::Var { name, span, .. }       => self.bound_var(name, span),
            e                                  => self.expected_type_expr("motion", e),
        }
    }

    /// Consumes the next argument and returns a [`RepeatMode`].
    pub fn repeat_mode(&mut self) -> Result<RepeatMode, DslError> {
        match self.next("repeat_mode")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, .. } => Ok(match name.as_str() {
                "RepeatMode::Forever" => RepeatMode::Forever,
                "RepeatMode::Times"   => RepeatMode::Times(self.extract_nested(args, Arguments::read_u32, span)?),
                "RepeatMode::Duration"=> RepeatMode::Duration(self.extract_nested(args, Arguments::duration, span)?),
                _                     => self.expected_type("repeat_mode", name, span)?,
            }),
            Expr::Literal(Value::RepeatMode(m), _) => Ok(m),
            Expr::Var { name, span, .. }           => self.bound_var(name, span),
            e                                      => self.expected_type_expr("repeat_mode", e),
        }
    }

    /// Consumes the next argument and returns a [`Margin`].
    pub fn margin(&mut self) -> Result<Margin, DslError> {
        match self.next("margin")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, .. } if name == "Margin::new" => {
                let mut inner_args = self.nested_args(args, 2, span)?;
                Ok(Margin::new(inner_args.read_u16()?, inner_args.read_u16()?))
            },
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e                            => self.expected_type_expr("margin", e),
        }
    }

    /// Consumes the next argument and returns a [`Rect`].
    pub fn rect(&mut self) -> Result<Rect, DslError> {
        match self.next("rect")? {
            Expr::FnCall { call, self_fns } => match call.name.as_str() {
                "Rect::new" => {
                    let mut inner_args = self.nested_args(call.args, 4, call.span)?;
                    let x = inner_args.read_u16()?;
                    let y = inner_args.read_u16()?;
                    let width = inner_args.read_u16()?;
                    let height = inner_args.read_u16()?;

                    Ok(Rect::new(x, y, width, height)
                        .fold_fns(self_fns, self.context, self.vars)?)
                },
                e => Err(DslError::UnknownFunction {
                    name: e.to_compact_string(),
                    location: call.span,
                }),
            },
            Expr::StructInit { name, fields, span } => {
                if name == "Rect" {
                    let fields = struct_fields("Rect", &["x", "y", "width", "height"], fields)
                        .map_err(|e| e.with_span(span))?;
                    Ok(Rect {
                        x: self.extract_field("x", &fields, Arguments::read_u16, span)?,
                        y: self.extract_field("y", &fields, Arguments::read_u16, span)?,
                        width: self.extract_field("width", &fields, Arguments::read_u16, span)?,
                        height: self.extract_field("height", &fields, Arguments::read_u16, span)?,
                    })
                } else {
                    Err(DslError::UnknownStruct {
                        name: name.to_compact_string(),
                        location: span,
                    })
                }
            },
            Expr::Var { name, self_fns, span } => self.bound_var::<Rect>(name, span)?
                .fold_fns(self_fns, self.context, self.vars),

            e => self.expected_type_expr("rect", e),
        }
    }

    /// Consumes the next argument and returns an `Offset` tuple.
    pub fn offset(&mut self) -> Result<Offset, DslError> {
        match self.next("offset")? {
            Expr::StructInit { name, fields, span } => {
                if name == "Offset" {
                    let fields = struct_fields("Offset", &["x", "y"], fields)?;
                    Ok(Offset {
                        x: self.extract_field("x", &fields, Arguments::read_i32, span)?,
                        y: self.extract_field("y", &fields, Arguments::read_i32, span)?,
                    })
                } else {
                    Err(DslError::UnknownStruct {
                        name: name.to_compact_string(),
                        location: span,
                    })
                }
            }
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e                            => self.expected_type_expr("offset", e),
        }
    }

    /// Consumes the next argument and returns a `Vec<T>`.
    #[allow(private_bounds)]
    pub fn array<T: Clone + FromDslExpr + 'static>(
        &mut self,
        inner: impl Fn(&mut Self) -> Result<T, DslError>
    ) -> Result<Vec<T>, DslError> {
        match self.next("array")? {
            Expr::Array(exprs, span)        => self.map_exprs(exprs, inner, span),
            Expr::ArrayRef(exprs, span)     => self.map_exprs(exprs, inner, span),
            Expr::Macro { name, args, span } if name == "vec" => self.map_exprs(args, inner, span),
            Expr::Var { name, span, .. }    => self.bound_var(name, span),
            e                               => self.expected_type_expr("array", e),
        }
    }

    pub fn boxed<T: Clone + FromDslExpr + 'static>(
        &mut self,
        inner: impl Fn(&mut Self) -> Result<T, DslError>,
        span: ExprSpan,
    ) -> Result<Box<T>, DslError> {
        match self.next("box")? {
            Expr::FnCall { call: FnCallInfo { name, args, .. }, .. } if name == "Box::new" => {
                let mut inner_args = self.nested_args(args, 1, span)?;
                inner(&mut inner_args).map(Box::new)
            },
            e => self.expected_type_expr("box", e),
        }
    }

    pub(super) fn original_arg_count(&self) -> usize {
        self.initial_arg_count
    }

    pub(super) fn span(&self) -> ExprSpan {
        self.span
    }

    fn map_exprs<T: Clone>(
        &mut self,
        exprs: Vec<Expr>,
        inner: impl Fn(&mut Self) -> Result<T, DslError>,
        span: ExprSpan,
    ) -> Result<Vec<T>, DslError> {
        let mut args = self.all_inner_args(exprs, span);
        (0..args.initial_arg_count)
            .map(|_| inner(&mut args)).collect()
    }

    fn compile_effect(&self, expr: Expr) -> Result<Effect, DslError> {
        self.context.compile(self.vars, [expr].into())
    }

    fn bound_var<T: Clone + FromDslExpr + 'static>(
        &self,
        name: impl Into<CompactString>,
        span: ExprSpan,
    ) -> Result<T, DslError> {
        self.vars.bound_var(self.context, name, span)
    }

    fn next(&mut self, type_name: &'static str) -> Result<Expr, DslError> {
        self.args.pop_front()
            .ok_or(DslError::MissingArgument {
                position: self.initial_arg_count - self.args.len() + 1,
                name: type_name,
                location: ExprSpan::new(
                    self.span.start + self.span.len().saturating_sub(1),
                    self.span.end
                ),
            })
            .and_then(|arg| if let Expr::SyntaxError { message, span } = arg {
                Err(DslError::SyntaxError { message, location: span })
            } else {
                Ok(arg)
            })
    }

    fn peek(&self) -> Option<&Expr> {
        self.args.front()
    }

    fn expected_type<T>(
        &self,
        expected: &'static str,
        actual: CompactString,
        span: ExprSpan
    ) -> Result<T, DslError>  {
        Err(DslError::WrongArgumentType {
            location: span,
            expected,
            actual
        })
    }

    fn expected_type_expr<T>(
        &self,
        expected: &'static str,
        actual: Expr,
    ) -> Result<T, DslError>  {
        self.expected_type(expected, actual.type_name().to_compact_string(), actual.span())
    }

    fn nested_args(
        &mut self,
        exprs: Vec<Expr>,
        required_arg_count: usize,
        span: ExprSpan,
    ) -> Result<Self, DslError> {
        if exprs.len() != required_arg_count {
            let start = exprs.iter().map(|e| e.span().start).min().unwrap_or_default();
            let end = exprs.iter().map(|e| e.span().end).max().unwrap_or_default();
            return Err(DslError::InvalidArgumentLength {
                expected: required_arg_count,
                actual: exprs.len(),
                location: ExprSpan::new(start, end),
            });
        }

        Ok(self.all_inner_args(exprs, span))
    }

    pub(super) fn extract_nested<T>(
        &mut self,
        exprs: Vec<Expr>,
        inner: impl Fn(&mut Self) -> Result<T, DslError>,
        span: ExprSpan,
    ) -> Result<T, DslError> {
        let mut args = self.nested_args(exprs, 1, span)?;
        inner(&mut args)
    }

    pub(super) fn extract_field<T>(
        &mut self,
        key: &'static str,
        exprs: &BTreeMap<&'static str, Expr>,
        inner: impl FnOnce(&mut Self) -> Result<T, DslError>,
        span: ExprSpan,
    ) -> Result<T, DslError> {
        let field_expr = exprs.get(key).expect("key to already be validated").clone();
        let mut args = self.nested_args(vec![field_expr], 1, span)?;
        inner(&mut args)
    }

    fn all_inner_args(&mut self, exprs: Vec<Expr>, span: ExprSpan) -> Self {
        Self::new(exprs.into(), self.context, self.vars, span)
    }
}

fn struct_fields(
    struct_name: &'static str,
    required: &[&'static str],
    fields: Vec<(CompactString, Expr)>
) -> Result<BTreeMap<&'static str, Expr>, DslError> {
    let mut field_values = BTreeMap::new();

    // todo: validate that all fields are used
    for field_name in required {

        let field_expr = fields.iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, expr)| expr.clone());

        match field_expr {
            Some(expr) => {
                field_values.insert(*field_name, expr);
            },
            None => Err(DslError::MissingField {
                field: field_name,
                struct_name: struct_name.into(),
                location: ExprSpan::default(), // span updated by the caller
            })?,
        }
    }

    Ok(field_values)
}


impl DslError {
    pub(super) fn with_span(self, span: ExprSpan) -> Self {
        match self {
            DslError::CastOverflow { to, from, .. } => {
                DslError::CastOverflow { to, from, location: span }
            }
            DslError::InvalidArgumentLength { expected, actual, .. } => {
                DslError::InvalidArgumentLength { location: span, expected, actual }
            }
            DslError::InvalidExpression {expected, actual, .. } => {
                DslError::InvalidExpression { location: span, expected, actual }
            }
            DslError::MissingArgument { position, name, .. } => {
                DslError::MissingArgument { position, name, location: span }
            }
            DslError::MissingField { struct_name, field, .. } => {
                DslError::MissingField { struct_name, field, location: span }
            }
            DslError::NoSuchVariable { name, expected, .. } => {
                DslError::NoSuchVariable { name, expected, location: span }
            }
            DslError::UnknownArgument { name, .. } => {
                DslError::UnknownArgument { name, location: span }
            }
            DslError::TooManyArguments { name, count, .. } => {
                DslError::TooManyArguments { name, count, location: span }
            }
            DslError::UnknownField { struct_name, field, valid_fields, .. } => {
                DslError::UnknownField { struct_name, field, valid_fields, location: span, }
            }
            DslError::UnknownFunction { name, .. } => {
                DslError::UnknownFunction { name, location: span }
            }
            DslError::UnknownStruct { name, .. } => {
                DslError::UnknownStruct { name, location: span }
            }
            DslError::WrongArgumentType { expected, actual, .. } => {
                DslError::WrongArgumentType { location: span, expected, actual }
            }
            DslError::TokenizationError { .. } => {
                DslError::TokenizationError { location: span }
            }
            DslError::TokenParseError { .. } => {
                DslError::TokenParseError { location: span }
            }
            DslError::OhNoError => DslError::OhNoError,
            DslError::UnknownEffect { name, .. } => DslError::UnknownEffect { name, location: span },
            // DslError::EffectExpressionNotSupported(name)
            // DslError::UnsupportedEffect { name, .. }
            DslError::ArrayLengthMismatch { expected, actual, .. } => {
                DslError::ArrayLengthMismatch { location: span, expected, actual }
            }
            DslError::UnknownCellFilter { name, .. } => {
                DslError::UnknownCellFilter { name, location: span }
            }
            _ => self,
        }
    }

    pub(super) fn span(&self) -> Option<ExprSpan> {
        Some(match self {
            DslError::ArrayLengthMismatch { location, .. } => *location,
            DslError::BracketMismatch { location, .. } => *location,
            DslError::CastOverflow { location, .. } => *location,
            DslError::InvalidArgumentLength { location, .. } => *location,
            DslError::InvalidExpression { location, .. } => *location,
            DslError::MissingArgument { location, .. } => *location,
            DslError::MissingSemicolon { location, .. } => *location,
            DslError::MissingComma { location, .. } => *location,
            DslError::MissingField { location, .. } => *location,
            DslError::NoSuchVariable { location, .. } => *location,
            DslError::SyntaxError { location, .. } => *location,
            DslError::TokenParseError { location, .. } => *location,
            DslError::TokenizationError { location, .. } => *location,
            DslError::TooManyArguments { location, .. } => *location,
            DslError::UnknownArgument { location, .. } => *location,
            DslError::UnknownCellFilter { location, .. } => *location,
            DslError::UnknownEffect { location, .. } => *location,
            DslError::UnknownField { location, .. } => *location,
            DslError::UnknownFunction { location, .. } => *location,
            DslError::UnknownStruct { location, .. } => *location,
            DslError::WrongArgumentType { location, .. } => *location,

            DslError::EffectExpressionNotSupported { .. } => None?,
            DslError::OhNoError => None?,
            DslError::UnsupportedEffect { .. } => None?,
        })
    }
}

impl fmt::Display for Arguments<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Arguments({})", self.args
            .iter()
            .map(|e| e.type_name())
            .collect::<Vec<_>>()
            .join(", ")
        )
    }
}

/// An internal trait for types that can be compiled from let
/// expressions in the DSL.
pub trait FromDslExpr where Self: Sized {
    /// Attempts to compile a value of type `Self` from a let expression.
    ///
    /// # Arguments
    ///
    /// * `args` - The argument parser containing the expression to convert
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - The successfully compiled value
    /// * `Err(DslError)` - If the compilation fails or the expression type doesn't match
    fn from_expr(
        args: &mut Arguments<'_>,
    ) -> Result<Self, DslError>;
}

impl<T: Clone + FromDslExpr + 'static> FromDslExpr for Option<T> {
    fn from_expr(args: &mut Arguments<'_>) -> Result<Self, DslError> {
        args.option(FromDslExpr::from_expr)
    }
}

impl<T: Clone + FromDslExpr + 'static> FromDslExpr for Vec<T> {
    fn from_expr(args: &mut Arguments<'_>) -> Result<Self, DslError> {
        args.array(FromDslExpr::from_expr)
    }
}

impl<const N: usize> FromDslExpr for [f32; N] {
    fn from_expr(args: &mut Arguments<'_>) -> Result<Self, DslError> {
        args.array(FromDslExpr::from_expr)
            .map(|v| {
                let mut arr = [0.0; N];
                arr.copy_from_slice(&v);
                arr
            })
    }
}

macro_rules! impl_from_args {
    ($type:ty, $method:ident) => {
        impl FromDslExpr for $type {
            fn from_expr(args: &mut Arguments<'_>) -> Result<Self, DslError> {
                args.$method()
            }
        }
    };
}

// Basic numeric types
impl_from_args!(bool, read_bool);
impl_from_args!(u8,   read_u8);
impl_from_args!(u16,  read_u16);
impl_from_args!(u32,  read_u32);
impl_from_args!(i32,  read_i32);
impl_from_args!(f32,  read_f32);

// String types
impl_from_args!(CompactString, string);

// Color/Style related
impl_from_args!(Color, color);
impl_from_args!(Style, style);
impl_from_args!(Modifier, modifier);

// Layout related
impl_from_args!(Direction, direction);
impl_from_args!(Layout, layout);
impl_from_args!(Constraint, constraint);
impl_from_args!(Margin, margin);
impl_from_args!(Rect, rect);
impl_from_args!(Offset, offset);

// Effect related
impl_from_args!(Effect, effect);
impl_from_args!(Duration, duration);
impl_from_args!(EffectTimer, effect_timer);
impl_from_args!(Interpolation, interpolation);
impl_from_args!(Motion, motion);
impl_from_args!(RepeatMode, repeat_mode);
impl_from_args!(CellFilter, cell_filter);
impl_from_args!(ColorSpace, color_space);


#[cfg(test)]
mod tests {
    use crate::dsl::arguments::Arguments;
    use crate::dsl::dsl::EffectDsl;
    use crate::dsl::environment::DslEnv;
    use crate::dsl::expressions::{Expr, ExprSpan, FnCallInfo, Value};
    use crate::dsl::token_parsers::parse_ast;
    use crate::dsl::tokenizer::{sanitize_tokens, tokenize};
    use crate::dsl::DslError;
    use crate::{CellFilter, Motion};
    use compact_str::ToCompactString;
    use ratatui::layout::{Margin, Offset, Rect};
    use ratatui::prelude::Color;
    use std::collections::VecDeque;
    use std::fmt::Debug;
    use crate::dsl::token_verification::verify_tokens;

    fn prepare_test<'a>(args: impl Into<VecDeque<Expr>>) -> Arguments<'a> {
        // leaking, but it's fine for tests as it reduces boilerplate
        let dsl = Box::leak(Box::new(EffectDsl::new()));
        let env = Box::leak(Box::new(DslEnv::new()));

        Arguments::new(args.into(), dsl, env, ExprSpan::default())
    }

    fn assert_result<'a, T: Debug>(
        input: &str,
        expected: T,
        f: impl Fn(&mut Arguments<'a>) -> Result<T, DslError>
    ) {
        // leaking, but it's fine for tests as it reduces boilerplate
        let dsl = Box::leak(Box::new(EffectDsl::new()));
        let env = Box::leak(Box::new(DslEnv::new()));

        let args = parse_expr(input);
        let mut args = Arguments::new([args].into(), dsl, env, ExprSpan::default());
        let result = f(&mut args)
            .expect("value from arguments");

        assert_eq!(
            format!("{result:#?}"),
            format!("{expected:#?}")
        );
    }

    #[test]
    fn test_numeric_parsing() {
        let span = ExprSpan::new(0, 0);
        let mut args = prepare_test(vec![
            Expr::Literal(Value::U32(42), span),
            Expr::Literal(Value::F32(3.14), span),
        ]);

        assert_eq!(args.read_u32(), Ok(42));
        assert_eq!(args.read_f32(), Ok(3.14));
        assert_eq!(args.read_u32(), Err(DslError::MissingArgument {
            position: 3,
            name: "u32",
            location: span,
        }));
    }

    #[test]
    fn test_array_parsing() {
        let span = ExprSpan::new(0, 0);
        // test a
        let mut args = prepare_test(vec![
            Expr::ArrayRef(vec![
                Expr::Literal(Value::F32(10.0), span),
                Expr::Literal(Value::F32(3.14), span),
            ], span),
        ]);

        let floats = args.array(Arguments::read_f32).unwrap();
        assert_eq!(floats, vec![10.0, 3.14]);

        // test b
        let mut args = prepare_test(vec![
            Expr::ArrayRef(vec![
                Expr::Literal(Value::String("a".into()), span),
                Expr::Literal(Value::String("b".into()), span),
                Expr::Literal(Value::String("c".into()), span),
            ], span),
        ]);

        let strings = args.array(Arguments::string).unwrap();
        assert_eq!(strings, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_option_parsing() {
        let span = ExprSpan::new(0, 0);
        let mut args = prepare_test(vec![
            Expr::OptionSome(Box::new(
                Expr::Array(vec![
                    Expr::Literal(Value::U32(1), span),
                    Expr::Literal(Value::U32(2), span),
                    Expr::Literal(Value::U32(3), span),
                ], span)
            ), span),
        ]);

        let inner_arg = args.option(|args| args.array(Arguments::read_u32)).unwrap();
        assert_eq!(inner_arg, Some(vec![1, 2, 3]));

        let mut args = prepare_test(vec![Expr::Literal(Value::OptionNone, span)]);
        let inner_arg = args.option(Arguments::read_u32).unwrap();
        assert_eq!(inner_arg, None);
    }

    fn parse_expr(input: &str) -> Expr {
        tokenize(input)
            .map(sanitize_tokens)
            .and_then(verify_tokens)
            .and_then(parse_ast)
            .unwrap()
            .last()
            .unwrap()
            .clone()
    }

    #[test]
    fn test_string_parsing() {
        let span = ExprSpan::new(0, 0);
        let mut args = prepare_test(vec![
            Expr::Literal(Value::String("hello".to_compact_string()), span),
            Expr::Literal(Value::U32(42), span), // Wrong type
            Expr::Literal(Value::String("world".to_compact_string()), span),
        ]);

        assert_eq!(args.string(), Ok("hello".to_compact_string()));
        assert_eq!(args.string(), Err(DslError::WrongArgumentType {
            location: span,
            expected: "string",
            actual: "u32".into()
        }));
        assert_eq!(args.string(), Ok("world".to_compact_string()));
    }

    #[test]
    fn test_color_parsing() {
        let span = ExprSpan::new(0, 0);
        let mut args = prepare_test(vec![
            Expr::Literal(Value::Color(Color::Red), span),
            Expr::Literal(Value::Color(Color::Blue), span),
        ]);

        assert_eq!(args.color(), Ok(Color::Red));
        assert_eq!(args.color(), Ok(Color::Blue));
        assert_eq!(args.color(), Err(DslError::MissingArgument {
            position: 3,
            name: "color",
            location: span,
        }));
    }

    #[test]
    fn test_motion_parsing() {
        let span = ExprSpan::new(0, 0);
        let mut args = prepare_test(vec![
            Expr::Literal(Value::Motion(Motion::LeftToRight), span),
            Expr::Literal(Value::Motion(Motion::UpToDown), span),
        ]);

        assert_eq!(args.motion(), Ok(Motion::LeftToRight));
        assert_eq!(args.motion(), Ok(Motion::UpToDown));
        assert_eq!(args.motion(), Err(DslError::MissingArgument {
            position: 3,
            name: "motion",
            location: span,
        }));
    }

    #[test]
    fn test_rect_method_chaining() {
        let expected = Rect::new(0, 0, 10, 10)
            .inner(Margin::new(1, 1))
            .clamp(Rect::new(5, 5, 10, 10))
            .intersection(Rect::new(0, 0, 5, 5))
            .union(Rect::new(5, 5, 15, 7))
            .offset(Offset { x: 20, y: 30 });

        let input = r#"Rect::new(0, 0, 10, 10)
            .inner(Margin::new(1, 1))
            .clamp(Rect::new(5, 5, 10, 10))
            .intersection(Rect::new(0, 0, 5, 5))
            .union(Rect::new(5, 5, 15, 7))
            .offset(Offset { x: 20, y: 30 })
        "#;

        // fixme: parse structs with fields (Offset, Rect, etc)
        assert_result(input, expected, Arguments::rect);
    }

    #[test]
    fn test_effect_parsing() {
        let span = ExprSpan::new(0, 0);
        let test_args = vec![Expr::Literal(Value::U32(500), span)];
        let mut args = prepare_test(vec![
            Expr::FnCall {
                call: FnCallInfo {
                    name: "fx::test".to_compact_string(),
                    args: test_args,
                    span
                },
                self_fns: vec![],
            },
        ]);

        let result = args.effect();
        assert!(result.is_err());
        assert_eq!(
            result.expect_err("expected error"),
            DslError::UnknownEffect {
                name: "test".to_compact_string(),
                location: span
            }
        );
    }

    #[test]
    fn test_cell_filter_parsing() {
        let span = ExprSpan::new(0, 0);
        // Test with a CellFilter constructor function call
        let mut args = prepare_test(vec![
            Expr::FnCall {
                call: FnCallInfo {
                    name: "CellFilter::FgColor".to_compact_string(),
                    args: vec![Expr::Literal(Value::Color(Color::Red), span)],
                    span,
                },
                self_fns: vec![],
            },
        ]);

        let result = args.cell_filter().unwrap();
        assert!(matches!(result, CellFilter::FgColor(Color::Red)));
    }
    
    #[test]
    fn test_cell_filter_allof_with_vec_macro() {
        let span = ExprSpan::new(0, 0);
        
        // Create a vec![] macro expression with two filters
        let text_filter = Expr::Literal(Value::CellFilter(CellFilter::Text), span);
        let fg_filter = Expr::FnCall {
            call: FnCallInfo {
                name: "CellFilter::FgColor".to_compact_string(),
                args: vec![Expr::Literal(Value::Color(Color::Red), span)],
                span
            },
            self_fns: vec![],
        };
        
        // Test with CellFilter::AllOf using vec![] macro
        let mut args = prepare_test(vec![
            Expr::FnCall {
                call: FnCallInfo {
                    name: "CellFilter::AllOf".to_compact_string(),
                    args: vec![
                        Expr::Macro {
                            name: "vec".into(),
                            args: vec![text_filter, fg_filter],
                            span
                        }
                    ],
                    span,
                },
                self_fns: vec![],
            },
        ]);

        let result = args.cell_filter().unwrap();
        if let CellFilter::AllOf(filters) = result {
            assert_eq!(filters.len(), 2);
            assert!(matches!(filters[0], CellFilter::Text));
            assert!(matches!(filters[1], CellFilter::FgColor(Color::Red)));
        } else {
            panic!("Expected CellFilter::AllOf, got {:?}", result);
        }
    }

    #[test]
    fn test_style_constructor_parsing() {
        let span = ExprSpan::new(0, 0);
        // Test with Style constructor
        let mut args = prepare_test(vec![
            Expr::FnCall {
                call: FnCallInfo {
                    name: "Style::new".to_compact_string(),
                    args: vec![],
                    span
                },
                self_fns: vec![
                    FnCallInfo {
                        name: "fg".to_compact_string(),
                        args: vec![Expr::Literal(Value::Color(Color::Red), span)],
                        span
                    }
                ],
            },
        ]);

        let result = args.style().unwrap();
        assert_eq!(result.fg, Some(Color::Red));
    }

    #[test]
    fn test_mixed_arguments() {
        let span = ExprSpan::new(0, 0);
        let mut args = prepare_test(vec![
            Expr::Literal(Value::U32(500), span),
            Expr::Literal(Value::Motion(Motion::LeftToRight), span),
            Expr::Literal(Value::Color(Color::Blue), span),
        ]);

        assert_eq!(args.read_u32(), Ok(500));
        assert_eq!(args.motion(), Ok(Motion::LeftToRight));
        assert_eq!(args.color(), Ok(Color::Blue));
        assert_eq!(args.read_u32(), Err(DslError::MissingArgument {
            position: 4,
            name: "u32",
            location: span,
        }));
    }

    #[test]
    fn test_u16_conversion() {
        let span = ExprSpan::new(0, 0);
        let mut args = prepare_test(vec![
            Expr::Literal(Value::U32(65535), span), // Max u16
            Expr::Literal(Value::U32(65536), span), // Too large for u16
        ]);

        assert_eq!(args.read_u16(), Ok(65535));
        assert_eq!(args.read_u16(), Err(DslError::CastOverflow {
            location: span,
            from: "u32",
            to: "u16",
        })); // Truncated
    }

    #[test]
    fn test_empty_args() {
        let mut args = prepare_test([]);

        let missing = |idx, name| Err(DslError::MissingArgument {
            position: idx,
            name,
            location: ExprSpan::default(),
        });

        assert_eq!(args.duration(), missing(1, "duration"));
    }
}