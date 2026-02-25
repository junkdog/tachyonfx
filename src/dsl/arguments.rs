#![allow(clippy::std_instead_of_alloc, clippy::std_instead_of_core)]

use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    fmt::Formatter,
};

use compact_str::{CompactString, ToCompactString};
use ratatui_core::{
    layout::{Constraint, Direction, Layout, Margin, Offset, Rect, Size},
    style::{Color, Modifier, Style},
};

use crate::{
    dsl::{
        dsl::EffectDsl,
        environment::DslEnv,
        expressions::{Expr, ExprSpan, FnCallInfo, Value},
        method_chains::ChainableMethods,
        DslError,
    },
    fx::{EvolveSymbolSet, ExpandDirection, RepeatMode},
    pattern::AnyPattern,
    wave::{Modulator, Oscillator, WaveLayer},
    CellFilter, ColorSpace, Duration, Effect, EffectTimer, Interpolation, Motion, RefRect,
    SimpleRng,
};

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
        let mut span = args
            .front()
            .map_or_else(|| fallback_span, Expr::span);
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
            Expr::Literal(Value::U32(ms), _) => Ok(Duration::from_millis(ms as _)),
            Expr::Literal(v, span) => self.expected_type("duration", v.format(), span),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("duration", e),
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
            Expr::Literal(Value::U32(ms), _) => Ok(ms.into()),
            Expr::Tuple(exprs, span) => {
                let mut args = self.nested_args(exprs, 2, span)?;
                let duration = args.duration()?;
                let interpolation = args.interpolation()?;
                Ok(EffectTimer::new(duration, interpolation))
            },
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("timer", e),
        }
    }

    /// Consumes the next argument and returns a [`Color`].
    pub fn cell_filter(&mut self) -> Result<CellFilter, DslError> {
        match self.next("cell_filter")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, self_fns } => {
                let filter_type = name.trim_start_matches("CellFilter::");
                let mut inner_args = Arguments::new(args.into(), self.context, self.vars, span);

                // All and Text are as literals
                match filter_type {
                    "Area" => CellFilter::Area(inner_args.rect()?),
                    "RefArea" => CellFilter::RefArea(inner_args.ref_rect()?),
                    "FgColor" => CellFilter::FgColor(inner_args.color()?),
                    "BgColor" => CellFilter::BgColor(inner_args.color()?),
                    "Inner" => CellFilter::Inner(inner_args.margin()?),
                    "Outer" => CellFilter::Outer(inner_args.margin()?),
                    "AllOf" => CellFilter::AllOf(inner_args.array(Arguments::cell_filter)?),
                    "AnyOf" => CellFilter::AnyOf(inner_args.array(Arguments::cell_filter)?),
                    "NoneOf" => CellFilter::NoneOf(inner_args.array(Arguments::cell_filter)?),
                    "Not" => CellFilter::Not(inner_args.boxed(Arguments::cell_filter, span)?),
                    "Static" => CellFilter::Static(inner_args.boxed(Arguments::cell_filter, span)?),
                    "Layout" => CellFilter::Layout(inner_args.layout()?, inner_args.read_u16()?),
                    "PositionFn" => CellFilter::PositionFn(inner_args.any_var()?),
                    "EvalCell" => CellFilter::EvalCell(inner_args.any_var()?),
                    e => Err(DslError::UnknownCellFilter {
                        name: e.to_compact_string(),
                        location: span,
                    })?,
                }
                .fold_fns(self_fns, self.context, self.vars)
            },
            Expr::Literal(Value::CellFilter(f), _) => Ok(f), // fixme: no self_fns on literals?
            Expr::Var { name, span, self_fns } => self
                .bound_var::<CellFilter>(name, span)?
                .fold_fns(self_fns, self.context, self.vars),
            e => self.expected_type_expr("cell_filter", e),
        }
    }

    pub fn color_space(&mut self) -> Result<ColorSpace, DslError> {
        match self.next("color_space")? {
            Expr::Literal(Value::ColorSpace(c), _) => Ok(c),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("color_space", e),
        }
    }

    /// Consumes the next argument and returns a `T`.
    pub fn any_var<T: Clone + 'static>(&mut self) -> Result<T, DslError> {
        match self.next("var")? {
            Expr::Var { name, span, .. } => self.vars.bound_global(name, span),
            e => self.expected_type_expr("var", e),
        }
    }

    /// Consumes the next argument and returns a [`Constraint`].
    pub fn constraint(&mut self) -> Result<Constraint, DslError> {
        use Constraint::*;

        match self.next("constraint")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, .. } => {
                Ok(match name.trim_start_matches("Constraint::") {
                    "Min" => Min(self.extract_nested(args, Arguments::read_u16, span)?),
                    "Max" => Max(self.extract_nested(args, Arguments::read_u16, span)?),
                    "Length" => Length(self.extract_nested(args, Arguments::read_u16, span)?),
                    "Percentage" => {
                        Percentage(self.extract_nested(args, Arguments::read_u16, span)?)
                    },
                    "Fill" => Fill(self.extract_nested(args, Arguments::read_u16, span)?),
                    "Ratio" => {
                        let mut inner_args = self.nested_args(args, 2, span)?;
                        let a = inner_args.read_u32()?;
                        let b = inner_args.read_u32()?;
                        Ratio(a, b)
                    },
                    _ => self.expected_type("constraint", name, span)?,
                })
            },
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("constraint", e),
        }
    }

    /// Consumes the next argument and returns a [`Direction`].
    pub fn direction(&mut self) -> Result<Direction, DslError> {
        match self.next("direction")? {
            Expr::Literal(Value::Direction(d), _) => Ok(d),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("direction", e),
        }
    }

    /// Consumes the next argument and returns a [`Flex`].
    pub fn flex(&mut self) -> Result<ratatui_core::layout::Flex, DslError> {
        match self.next("flex")? {
            Expr::Literal(Value::Flex(f), _) => Ok(f),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("flex", e),
        }
    }

    /// Consumes the next argument and returns a [`Layout`].
    pub fn layout(&mut self) -> Result<Layout, DslError> {
        match self.next("layout")? {
            Expr::FnCall { call, self_fns } => {
                let base_layout = match call.name.as_str() {
                    "Layout::default" => {
                        self.verify_no_nested_args(call.args, call.span)?;
                        Ok(Layout::default())
                    },
                    "Layout::horizontal" => {
                        let constraints = self.extract_nested(
                            call.args,
                            |a| a.array(Arguments::constraint),
                            call.span,
                        )?;
                        Ok(Layout::horizontal(constraints))
                    },
                    "Layout::vertical" => {
                        let constraints = self.extract_nested(
                            call.args,
                            |a| a.array(Arguments::constraint),
                            call.span,
                        )?;
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

            Expr::Var { name, self_fns, span } => {
                self.bound_var::<Layout>(name, span)?
                    .fold_fns(self_fns, self.context, self.vars)
            },

            e => self.expected_type_expr("layout", e),
        }
    }

    /// Consumes the next argument and returns a [`Interpolation`].
    pub fn interpolation(&mut self) -> Result<Interpolation, DslError> {
        match self.next("interpolation")? {
            Expr::Literal(Value::Interpolation(i), _) => Ok(i),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("interpolation", e),
        }
    }

    /// Consumes the next argument and returns a `bool`.
    pub fn read_bool(&mut self) -> Result<bool, DslError> {
        match self.next("bool")? {
            Expr::Literal(Value::Bool(v), _) => Ok(v),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("bool", e),
        }
    }

    /// Consumes the next argument and returns a `u8`.
    pub fn read_u8(&mut self) -> Result<u8, DslError> {
        let span = self.peek().map(Expr::span);
        u8::try_from(self.read_u32()?).map_err(|_| DslError::CastOverflow {
            location: span.unwrap(),
            from: "u32",
            to: "u8",
        })
    }

    /// Consumes the next argument and returns a `u16`.
    pub fn read_u16(&mut self) -> Result<u16, DslError> {
        let span = self.peek().map(Expr::span);
        u16::try_from(self.read_u32()?).map_err(|_| DslError::CastOverflow {
            location: span.unwrap(),
            from: "u32",
            to: "u16",
        })
    }

    /// Consumes the next argument and returns a `u32`.
    pub fn read_u32(&mut self) -> Result<u32, DslError> {
        match self.next("u32")? {
            Expr::Literal(Value::U32(u), _) => Ok(u),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("u32", e),
        }
    }

    pub fn read_i32(&mut self) -> Result<i32, DslError> {
        match self.next("i32")? {
            Expr::Literal(Value::I32(i), _) => Ok(i),
            Expr::Literal(Value::U32(i), _) => Ok(i as _),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("i32", e),
        }
    }

    /// Consumes the next argument and returns a `f32`.
    pub fn read_into_f32(&mut self) -> Result<f32, DslError> {
        match self.next("f32")? {
            Expr::Literal(Value::F32(f), _) => Ok(f),
            Expr::Literal(Value::U32(v), _) => Ok(v as f32),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("f32", e),
        }
    }

    /// Consumes the next argument and returns a `f32`.
    pub fn read_f32(&mut self) -> Result<f32, DslError> {
        match self.next("f32")? {
            Expr::Literal(Value::F32(f), _) => Ok(f),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("f32", e),
        }
    }

    /// Consumes the next argument and returns a [`CompactString`].
    pub fn string(&mut self) -> Result<CompactString, DslError> {
        match self.next("string")? {
            Expr::Literal(Value::String(s), _) => Ok(s),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("string", e),
        }
    }

    /// Consumes the next argument and returns an `Option<T>`.
    #[allow(private_bounds)]
    pub fn option<T: Clone + FromDslExpr + 'static>(
        &mut self,
        inner: impl Fn(&mut Self) -> Result<T, DslError>,
    ) -> Result<Option<T>, DslError> {
        match self.next("option")? {
            Expr::Literal(Value::OptionNone, _) => Ok(None),
            Expr::OptionSome(expr, span) => {
                let mut args = self.nested_args(vec![*expr], 1, span)?;
                inner(&mut args).map(Some)
            },
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("option", e),
        }
    }

    /// Consumes the next argument and returns an [`Effect`].
    pub fn effect(&mut self) -> Result<Effect, DslError> {
        match self.next("effect")? {
            Expr::FnCall { call, self_fns } => {
                // Check if it's an effect constructor with "fx::" prefix
                let fx_name = call
                    .name
                    .strip_prefix("fx::")
                    .unwrap_or(&call.name);

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
            Expr::Sequence { effects, self_fns, span } => {
                self.compile_effect(Expr::Sequence { effects, self_fns, span })
            },
            Expr::Parallel { effects, self_fns, span } => {
                self.compile_effect(Expr::Parallel { effects, self_fns, span })
            },

            Expr::Var { name, self_fns, span } => {
                self.bound_var::<Effect>(name, span)?
                    .fold_fns(self_fns, self.context, self.vars)
            },

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
                },
                "Color::Indexed" => {
                    Color::Indexed(self.extract_nested(args, Arguments::read_u8, span)?)
                },
                _ => self.expected_type("color", name, span)?,
            }),
            Expr::Literal(Value::Color(c), _) => Ok(c),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("color", e),
        }
    }

    /// Consumes the next argument and returns a [`Modifier`].
    pub fn modifier(&mut self) -> Result<Modifier, DslError> {
        match self.next("modifier")? {
            Expr::Literal(Value::Modifier(m), _) => Ok(m),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("modifier", e),
        }
    }

    /// Consumes the next argument and returns a [`Style`].
    pub fn style(&mut self) -> Result<Style, DslError> {
        match self.next("style")? {
            Expr::FnCall { call, self_fns } => {
                if call.name == "Style::new" || call.name == "Style::default" {
                    self.verify_no_nested_args(call.args, call.span)?;
                    Style::new().fold_fns(self_fns, self.context, self.vars)
                } else {
                    self.expected_type("style", call.name.to_compact_string(), call.span)?
                }
            },
            Expr::Var { name, self_fns, span } => {
                self.bound_var::<Style>(name, span)?
                    .fold_fns(self_fns, self.context, self.vars)
            },
            e => self.expected_type_expr("style", e),
        }
    }

    /// Consumes the next argument and returns a [`Motion`].
    pub fn motion(&mut self) -> Result<Motion, DslError> {
        match self.next("motion")? {
            Expr::Literal(Value::Motion(m), _) => Ok(m),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("motion", e),
        }
    }

    /// Consumes the next argument and returns an [`ExpandDirection`].
    pub fn expand_direction(&mut self) -> Result<ExpandDirection, DslError> {
        match self.next("expand_direction")? {
            Expr::Literal(Value::ExpandDirection(d), _) => Ok(d),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("expand_direction", e),
        }
    }

    pub fn evolve_symbol_set(&mut self) -> Result<EvolveSymbolSet, DslError> {
        match self.next("evolve_symbol_set")? {
            Expr::Literal(Value::EvolveSymbolSet(s), _) => Ok(s),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("evolve_symbol_set", e),
        }
    }

    /// Consumes the next argument and returns a [`RepeatMode`].
    pub fn repeat_mode(&mut self) -> Result<RepeatMode, DslError> {
        match self.next("repeat_mode")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, .. } => Ok(match name.as_str() {
                "RepeatMode::Times" => {
                    RepeatMode::Times(self.extract_nested(args, Arguments::read_u32, span)?)
                },
                "RepeatMode::Duration" => {
                    RepeatMode::Duration(self.extract_nested(args, Arguments::duration, span)?)
                },
                _ => self.expected_type("repeat_mode", name, span)?,
            }),
            Expr::Literal(Value::RepeatMode(m), _) => Ok(m),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("repeat_mode", e),
        }
    }

    /// Consumes the next argument and returns a [`SimpleRng`].
    pub fn simple_rng(&mut self) -> Result<SimpleRng, DslError> {
        match self.next("simple_rng")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, .. } => Ok(match name.as_str() {
                "SimpleRng::new" => {
                    SimpleRng::new(self.extract_nested(args, Arguments::read_u32, span)?)
                },
                "SimpleRng::default" => {
                    self.verify_no_nested_args(args, span)?;
                    SimpleRng::default()
                },
                _ => self.expected_type("simple_rng", name, span)?,
            }),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("simple_rng", e),
        }
    }

    pub fn pattern(&mut self) -> Result<AnyPattern, DslError> {
        use crate::pattern::*;
        match self.next("pattern")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, self_fns } => {
                let pattern = match name.as_str() {
                    "CheckerboardPattern::default" => {
                        self.verify_no_nested_args(args, span)?;
                        AnyPattern::from(CheckerboardPattern::default())
                    },
                    "CheckerboardPattern::with_cell_size" => {
                        let cell_size = self.extract_nested(args, Arguments::read_u16, span)?;
                        AnyPattern::from(CheckerboardPattern::with_cell_size(cell_size))
                    },

                    "CoalescePattern::new" | "CoalescePattern::default" => {
                        self.verify_no_nested_args(args, span)?;
                        AnyPattern::from(CoalescePattern::default())
                    },
                    "CoalescePattern::from" => {
                        let rng = self.extract_nested(args, Arguments::simple_rng, span)?;
                        AnyPattern::from(CoalescePattern::from(rng))
                    },

                    "DiagonalPattern::top_left_to_bottom_right" => {
                        self.verify_no_nested_args(args, span)?;
                        AnyPattern::from(DiagonalPattern::top_left_to_bottom_right())
                    },
                    "DiagonalPattern::top_right_to_bottom_left" => {
                        self.verify_no_nested_args(args, span)?;
                        AnyPattern::from(DiagonalPattern::top_right_to_bottom_left())
                    },
                    "DiagonalPattern::bottom_left_to_top_right" => {
                        self.verify_no_nested_args(args, span)?;
                        AnyPattern::from(DiagonalPattern::bottom_left_to_top_right())
                    },
                    "DiagonalPattern::bottom_right_to_top_left" => {
                        self.verify_no_nested_args(args, span)?;
                        AnyPattern::from(DiagonalPattern::bottom_right_to_top_left())
                    },

                    "DissolvePattern::new" | "DissolvePattern::default" => {
                        self.verify_no_nested_args(args, span)?;
                        AnyPattern::from(DissolvePattern::default())
                    },

                    "DissolvePattern::from" => {
                        let rng = self.extract_nested(args, Arguments::simple_rng, span)?;
                        AnyPattern::from(DissolvePattern::from(rng))
                    },

                    "RadialPattern::center" => {
                        self.verify_no_nested_args(args, span)?;
                        AnyPattern::from(RadialPattern::center())
                    },
                    "RadialPattern::new" => {
                        let mut inner_args = self.nested_args(args, 2, span)?;
                        let center_x = inner_args.read_f32()?;
                        let center_y = inner_args.read_f32()?;
                        AnyPattern::from(RadialPattern::new(center_x, center_y))
                    },
                    "RadialPattern::with_transition" => {
                        let mut inner_args = self.nested_args(args, 2, span)?;
                        let center_xy =
                            inner_args.tuple_2(Arguments::read_f32, Arguments::read_f32)?;
                        let transition_width = inner_args.read_f32()?;
                        AnyPattern::from(RadialPattern::with_transition(
                            center_xy,
                            transition_width,
                        ))
                    },

                    "DiamondPattern::center" => {
                        self.verify_no_nested_args(args, span)?;
                        AnyPattern::from(DiamondPattern::center())
                    },
                    "DiamondPattern::new" => {
                        let mut inner_args = self.nested_args(args, 2, span)?;
                        let center_x = inner_args.read_f32()?;
                        let center_y = inner_args.read_f32()?;
                        AnyPattern::from(DiamondPattern::new(center_x, center_y))
                    },
                    "DiamondPattern::with_transition" => {
                        let mut inner_args = self.nested_args(args, 2, span)?;
                        let center_xy =
                            inner_args.tuple_2(Arguments::read_f32, Arguments::read_f32)?;
                        let transition_width = inner_args.read_f32()?;
                        AnyPattern::from(DiamondPattern::with_transition(
                            center_xy,
                            transition_width,
                        ))
                    },

                    "SpiralPattern::center" => {
                        self.verify_no_nested_args(args, span)?;
                        AnyPattern::from(SpiralPattern::center())
                    },
                    "SpiralPattern::new" => {
                        let mut inner_args = self.nested_args(args, 2, span)?;
                        let center_x = inner_args.read_f32()?;
                        let center_y = inner_args.read_f32()?;
                        AnyPattern::from(SpiralPattern::new(center_x, center_y))
                    },
                    "SpiralPattern::with_transition" => {
                        let mut inner_args = self.nested_args(args, 2, span)?;
                        let center_xy =
                            inner_args.tuple_2(Arguments::read_f32, Arguments::read_f32)?;
                        let transition_width = inner_args.read_f32()?;
                        AnyPattern::from(SpiralPattern::with_transition(
                            center_xy,
                            transition_width,
                        ))
                    },

                    "SweepPattern::left_to_right" => {
                        let width = self.extract_nested(args, Arguments::read_u16, span)?;
                        AnyPattern::from(SweepPattern::left_to_right(width))
                    },
                    "SweepPattern::right_to_left" => {
                        let width = self.extract_nested(args, Arguments::read_u16, span)?;
                        AnyPattern::from(SweepPattern::right_to_left(width))
                    },
                    "SweepPattern::up_to_down" => {
                        let width = self.extract_nested(args, Arguments::read_u16, span)?;
                        AnyPattern::from(SweepPattern::up_to_down(width))
                    },
                    "SweepPattern::down_to_up" => {
                        let width = self.extract_nested(args, Arguments::read_u16, span)?;
                        AnyPattern::from(SweepPattern::down_to_up(width))
                    },

                    "WavePattern::new" => {
                        let layer = self.extract_nested(args, Arguments::wave_layer, span)?;
                        AnyPattern::from(WavePattern::new(layer))
                    },

                    "CombinedPattern::multiply" => {
                        let mut inner_args = self.nested_args(args, 2, span)?;
                        let pattern_a = inner_args.pattern()?;
                        let pattern_b = inner_args.pattern()?;
                        AnyPattern::from(CombinedPattern::multiply(pattern_a, pattern_b))
                    },
                    "CombinedPattern::max" => {
                        let mut inner_args = self.nested_args(args, 2, span)?;
                        let pattern_a = inner_args.pattern()?;
                        let pattern_b = inner_args.pattern()?;
                        AnyPattern::from(CombinedPattern::max(pattern_a, pattern_b))
                    },
                    "CombinedPattern::min" => {
                        let mut inner_args = self.nested_args(args, 2, span)?;
                        let pattern_a = inner_args.pattern()?;
                        let pattern_b = inner_args.pattern()?;
                        AnyPattern::from(CombinedPattern::min(pattern_a, pattern_b))
                    },
                    "CombinedPattern::average" => {
                        let mut inner_args = self.nested_args(args, 2, span)?;
                        let pattern_a = inner_args.pattern()?;
                        let pattern_b = inner_args.pattern()?;
                        AnyPattern::from(CombinedPattern::average(pattern_a, pattern_b))
                    },

                    "InvertedPattern::new" => {
                        let inner = self.extract_nested(args, Arguments::pattern, span)?;
                        AnyPattern::from(InvertedPattern::new(inner))
                    },

                    "BlendPattern::new" => {
                        let mut inner_args = self.nested_args(args, 2, span)?;
                        let pattern_a = inner_args.pattern()?;
                        let pattern_b = inner_args.pattern()?;
                        AnyPattern::from(BlendPattern::new(pattern_a, pattern_b))
                    },

                    _ => self.expected_type("pattern", name, span)?,
                };

                // Handle method chaining for patterns
                pattern.fold_fns(self_fns, self.context, self.vars)
            },
            Expr::Var { name, span, self_fns } => self
                .bound_var::<AnyPattern>(name, span)?
                .fold_fns(self_fns, self.context, self.vars),

            e => self.expected_type_expr("pattern", e),
        }
    }

    /// Consumes the next argument and returns a [`Modulator`].
    pub fn modulator(&mut self) -> Result<Modulator, DslError> {
        match self.next("modulator")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, self_fns } => {
                let base = match name.as_str() {
                    "Modulator::sin" => {
                        let mut inner = self.nested_args(args, 3, span)?;
                        Modulator::sin(inner.read_f32()?, inner.read_f32()?, inner.read_f32()?)
                    },
                    "Modulator::cos" => {
                        let mut inner = self.nested_args(args, 3, span)?;
                        Modulator::cos(inner.read_f32()?, inner.read_f32()?, inner.read_f32()?)
                    },
                    "Modulator::triangle" => {
                        let mut inner = self.nested_args(args, 3, span)?;
                        Modulator::triangle(inner.read_f32()?, inner.read_f32()?, inner.read_f32()?)
                    },
                    "Modulator::sawtooth" => {
                        let mut inner = self.nested_args(args, 3, span)?;
                        Modulator::sawtooth(inner.read_f32()?, inner.read_f32()?, inner.read_f32()?)
                    },
                    _ => self.expected_type("modulator", name, span)?,
                };
                base.fold_fns(self_fns, self.context, self.vars)
            },
            Expr::Var { name, self_fns, span } => self
                .bound_var::<Modulator>(name, span)?
                .fold_fns(self_fns, self.context, self.vars),
            e => self.expected_type_expr("modulator", e),
        }
    }

    /// Consumes the next argument and returns an [`Oscillator`].
    pub fn oscillator(&mut self) -> Result<Oscillator, DslError> {
        match self.next("oscillator")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, self_fns } => {
                let base = match name.as_str() {
                    "Oscillator::sin" => {
                        let mut inner = self.nested_args(args, 3, span)?;
                        Oscillator::sin(inner.read_f32()?, inner.read_f32()?, inner.read_f32()?)
                    },
                    "Oscillator::cos" => {
                        let mut inner = self.nested_args(args, 3, span)?;
                        Oscillator::cos(inner.read_f32()?, inner.read_f32()?, inner.read_f32()?)
                    },
                    "Oscillator::triangle" => {
                        let mut inner = self.nested_args(args, 3, span)?;
                        Oscillator::triangle(
                            inner.read_f32()?,
                            inner.read_f32()?,
                            inner.read_f32()?,
                        )
                    },
                    "Oscillator::sawtooth" => {
                        let mut inner = self.nested_args(args, 3, span)?;
                        Oscillator::sawtooth(
                            inner.read_f32()?,
                            inner.read_f32()?,
                            inner.read_f32()?,
                        )
                    },
                    _ => self.expected_type("oscillator", name, span)?,
                };
                base.fold_fns(self_fns, self.context, self.vars)
            },
            Expr::Var { name, self_fns, span } => self
                .bound_var::<Oscillator>(name, span)?
                .fold_fns(self_fns, self.context, self.vars),
            e => self.expected_type_expr("oscillator", e),
        }
    }

    /// Consumes the next argument and returns a [`WaveLayer`].
    pub fn wave_layer(&mut self) -> Result<WaveLayer, DslError> {
        match self.next("wave_layer")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, self_fns } => {
                let base = match name.as_str() {
                    "WaveLayer::new" => {
                        let mut inner = self.nested_args(args, 1, span)?;
                        WaveLayer::new(inner.oscillator()?)
                    },
                    _ => self.expected_type("wave_layer", name, span)?,
                };
                base.fold_fns(self_fns, self.context, self.vars)
            },
            Expr::Var { name, self_fns, span } => self
                .bound_var::<WaveLayer>(name, span)?
                .fold_fns(self_fns, self.context, self.vars),
            e => self.expected_type_expr("wave_layer", e),
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
            e => self.expected_type_expr("margin", e),
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

                    Ok(Rect::new(x, y, width, height).fold_fns(
                        self_fns,
                        self.context,
                        self.vars,
                    )?)
                },
                e => Err(DslError::UnknownFunction {
                    name: e.to_compact_string(),
                    location: call.span,
                }),
            },
            Expr::StructInit { name, fields, span } => {
                if name == "Rect" {
                    let fields = struct_fields("Rect", &["x", "y", "width", "height"], &fields)
                        .map_err(|e| e.with_span(span))?;
                    Ok(Rect {
                        x: self.extract_field("x", &fields, Arguments::read_u16, span)?,
                        y: self.extract_field("y", &fields, Arguments::read_u16, span)?,
                        width: self.extract_field("width", &fields, Arguments::read_u16, span)?,
                        height: self.extract_field("height", &fields, Arguments::read_u16, span)?,
                    })
                } else {
                    Err(DslError::UnknownStruct { name: name.to_compact_string(), location: span })
                }
            },
            Expr::Var { name, self_fns, span } => {
                self.bound_var::<Rect>(name, span)?
                    .fold_fns(self_fns, self.context, self.vars)
            },

            e => self.expected_type_expr("rect", e),
        }
    }

    /// Consumes the next argument and returns a [`RefRect`].
    pub fn ref_rect(&mut self) -> Result<RefRect, DslError> {
        match self.next("ref_rect")? {
            Expr::FnCall { call, self_fns: _ } => match call.name.as_str() {
                "RefRect::new" => {
                    let mut inner_args = self.nested_args(call.args, 1, call.span)?;
                    let rect = inner_args.rect()?;
                    Ok(RefRect::new(rect))
                },
                "RefRect::default" => {
                    self.verify_no_nested_args(call.args, call.span)?;
                    Ok(RefRect::default())
                },
                e => Err(DslError::UnknownFunction {
                    name: e.to_compact_string(),
                    location: call.span,
                }),
            },
            Expr::Var { name, self_fns: _, span } => self.bound_var(name, span),

            e => self.expected_type_expr("ref_rect", e),
        }
    }

    /// Consumes the next argument and returns an `Offset` tuple.
    pub fn offset(&mut self) -> Result<Offset, DslError> {
        match self.next("offset")? {
            Expr::StructInit { name, fields, span } => {
                if name == "Offset" {
                    let fields = struct_fields("Offset", &["x", "y"], &fields)?;
                    Ok(Offset {
                        x: self.extract_field("x", &fields, Arguments::read_i32, span)?,
                        y: self.extract_field("y", &fields, Arguments::read_i32, span)?,
                    })
                } else {
                    Err(DslError::UnknownStruct { name: name.to_compact_string(), location: span })
                }
            },
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("offset", e),
        }
    }

    pub(super) fn tuple_2<A, B>(
        &mut self,
        inner_a: impl Fn(&mut Self) -> Result<A, DslError>,
        inner_b: impl Fn(&mut Self) -> Result<B, DslError>,
    ) -> Result<(A, B), DslError>
    where
        A: Clone + FromDslExpr + 'static,
        B: Clone + FromDslExpr + 'static,
    {
        match self.next("tuple_2")? {
            Expr::Tuple(exprs, span) => {
                let mut args = self.nested_args(exprs, 2, span)?;
                let a = inner_a(&mut args)?;
                let b = inner_b(&mut args)?;
                Ok((a, b))
            },

            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("tuple_2", e),
        }
    }

    /// Consumes the next argument and returns a `Size`.
    pub fn size(&mut self) -> Result<Size, DslError> {
        match self.next("size")? {
            Expr::FnCall { call: FnCallInfo { name, args, span }, .. } if name == "Size::new" => {
                let mut inner_args = self.nested_args(args, 2, span)?;
                let width = inner_args.read_u16()?;
                let height = inner_args.read_u16()?;
                Ok(Size::new(width, height))
            },
            Expr::StructInit { name, fields, span } => {
                if name == "Size" {
                    let fields = struct_fields("Size", &["width", "height"], &fields)?;
                    Ok(Size {
                        width: self.extract_field("width", &fields, Arguments::read_u16, span)?,
                        height: self.extract_field("height", &fields, Arguments::read_u16, span)?,
                    })
                } else {
                    Err(DslError::UnknownStruct { name: name.to_compact_string(), location: span })
                }
            },
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("size", e),
        }
    }

    /// Consumes the next argument and returns a `Vec<T>`.
    #[allow(private_bounds)]
    pub fn array<T: Clone + FromDslExpr + 'static>(
        &mut self,
        inner: impl Fn(&mut Self) -> Result<T, DslError>,
    ) -> Result<Vec<T>, DslError> {
        match self.next("array")? {
            Expr::Array(exprs, span) => self.map_exprs(exprs, inner, span),
            Expr::ArrayRef(exprs, span) => self.map_exprs(exprs, inner, span),
            Expr::Macro { name, args, span } if name == "vec" => self.map_exprs(args, inner, span),
            Expr::Var { name, span, .. } => self.bound_var(name, span),
            e => self.expected_type_expr("array", e),
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
        &self,
        exprs: Vec<Expr>,
        inner: impl Fn(&mut Self) -> Result<T, DslError>,
        span: ExprSpan,
    ) -> Result<Vec<T>, DslError> {
        let mut args = self.all_inner_args(exprs, span);
        (0..args.initial_arg_count)
            .map(|_| inner(&mut args))
            .collect()
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
        self.args
            .pop_front()
            .ok_or(DslError::MissingArgument {
                position: self.initial_arg_count - self.args.len() + 1,
                name: type_name,
                location: ExprSpan::new(
                    self.span.start + self.span.len().saturating_sub(1),
                    self.span.end,
                ),
            })
            .and_then(|arg| {
                if let Expr::SyntaxError { message, span } = arg {
                    Err(DslError::SyntaxError { message, location: span })
                } else {
                    Ok(arg)
                }
            })
    }

    pub(super) fn peek(&self) -> Option<&Expr> {
        self.args.front()
    }

    #[allow(clippy::unused_self)]
    fn expected_type<T>(
        &self,
        expected: &'static str,
        actual: CompactString,
        span: ExprSpan,
    ) -> Result<T, DslError> {
        Err(DslError::WrongArgumentType { location: span, expected, actual })
    }

    #[allow(clippy::needless_pass_by_value)] // 30+ call sites pass owned Expr
    fn expected_type_expr<T>(&self, expected: &'static str, actual: Expr) -> Result<T, DslError> {
        self.expected_type(
            expected,
            actual.type_name().to_compact_string(),
            actual.span(),
        )
    }

    fn verify_no_nested_args(&self, exprs: Vec<Expr>, span: ExprSpan) -> Result<Self, DslError> {
        self.nested_args(exprs, 0, span)
    }

    fn nested_args(
        &self,
        exprs: Vec<Expr>,
        required_arg_count: usize,
        span: ExprSpan,
    ) -> Result<Self, DslError> {
        if exprs.len() != required_arg_count {
            let start = exprs
                .iter()
                .map(|e| e.span().start)
                .min()
                .unwrap_or_default();
            let end = exprs
                .iter()
                .map(|e| e.span().end)
                .max()
                .unwrap_or_default();
            return Err(DslError::InvalidArgumentLength {
                expected: required_arg_count,
                actual: exprs.len(),
                location: ExprSpan::new(start, end),
            });
        }

        Ok(self.all_inner_args(exprs, span))
    }

    pub(super) fn extract_nested<T>(
        &self,
        exprs: Vec<Expr>,
        inner: impl Fn(&mut Self) -> Result<T, DslError>,
        span: ExprSpan,
    ) -> Result<T, DslError> {
        let mut args = self.nested_args(exprs, 1, span)?;
        inner(&mut args)
    }

    pub(super) fn extract_field<T>(
        &self,
        key: &'static str,
        exprs: &BTreeMap<&'static str, Expr>,
        inner: impl FnOnce(&mut Self) -> Result<T, DslError>,
        span: ExprSpan,
    ) -> Result<T, DslError> {
        let field_expr = exprs
            .get(key)
            .expect("key to already be validated")
            .clone();
        let mut args = self.nested_args(vec![field_expr], 1, span)?;
        inner(&mut args)
    }

    fn all_inner_args(&self, exprs: Vec<Expr>, span: ExprSpan) -> Self {
        Self::new(exprs.into(), self.context, self.vars, span)
    }
}

fn struct_fields(
    struct_name: &'static str,
    required: &[&'static str],
    fields: &[(CompactString, Expr)],
) -> Result<BTreeMap<&'static str, Expr>, DslError> {
    let mut field_values = BTreeMap::new();

    // todo: validate that all fields are used
    for field_name in required {
        let field_expr = fields
            .iter()
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
            },
            DslError::InvalidArgumentLength { expected, actual, .. } => {
                DslError::InvalidArgumentLength { location: span, expected, actual }
            },
            DslError::InvalidExpression { expected, actual, .. } => {
                DslError::InvalidExpression { location: span, expected, actual }
            },
            DslError::MissingArgument { position, name, .. } => {
                DslError::MissingArgument { position, name, location: span }
            },
            DslError::MissingField { struct_name, field, .. } => {
                DslError::MissingField { struct_name, field, location: span }
            },
            DslError::NoSuchVariable { name, expected, .. } => {
                DslError::NoSuchVariable { name, expected, location: span }
            },
            DslError::UnknownArgument { name, .. } => {
                DslError::UnknownArgument { name, location: span }
            },
            DslError::TooManyArguments { name, count, .. } => {
                DslError::TooManyArguments { name, count, location: span }
            },
            DslError::UnknownField { struct_name, field, valid_fields, .. } => {
                DslError::UnknownField { struct_name, field, valid_fields, location: span }
            },
            DslError::UnknownFunction { name, .. } => {
                DslError::UnknownFunction { name, location: span }
            },
            DslError::UnknownStruct { name, .. } => {
                DslError::UnknownStruct { name, location: span }
            },
            DslError::WrongArgumentType { expected, actual, .. } => {
                DslError::WrongArgumentType { location: span, expected, actual }
            },
            DslError::TokenizationError { .. } => DslError::TokenizationError { location: span },
            DslError::TokenParseError { .. } => DslError::TokenParseError { location: span },
            DslError::OhNoError => DslError::OhNoError,
            DslError::UnknownEffect { name, .. } => {
                DslError::UnknownEffect { name, location: span }
            },
            // DslError::EffectExpressionNotSupported(name)
            // DslError::UnsupportedEffect { name, .. }
            DslError::ArrayLengthMismatch { expected, actual, .. } => {
                DslError::ArrayLengthMismatch { location: span, expected, actual }
            },
            DslError::UnknownCellFilter { name, .. } => {
                DslError::UnknownCellFilter { name, location: span }
            },
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
        write!(
            f,
            "Arguments({})",
            self.args
                .iter()
                .map(Expr::type_name)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// An internal trait for types that can be compiled from let
/// expressions in the DSL.
pub trait FromDslExpr
where
    Self: Sized,
{
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
    fn from_expr(args: &mut Arguments<'_>) -> Result<Self, DslError>;
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
        args.array(FromDslExpr::from_expr).map(|v| {
            let mut arr = [0.0; N];
            arr.copy_from_slice(&v);
            arr
        })
    }
}

impl FromDslExpr for EvolveSymbolSet {
    fn from_expr(args: &mut Arguments<'_>) -> Result<Self, DslError> {
        args.evolve_symbol_set()
    }
}

impl<A, B> FromDslExpr for (A, B)
where
    A: Clone + FromDslExpr + 'static,
    B: Clone + FromDslExpr + 'static,
{
    fn from_expr(args: &mut Arguments<'_>) -> Result<Self, DslError> {
        args.tuple_2(A::from_expr, B::from_expr)
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
impl_from_args!(u8, read_u8);
impl_from_args!(u16, read_u16);
impl_from_args!(u32, read_u32);
impl_from_args!(i32, read_i32);
impl_from_args!(f32, read_f32);

// String types
impl_from_args!(CompactString, string);

// Color/Style related
impl_from_args!(Color, color);
impl_from_args!(Style, style);
impl_from_args!(Modifier, modifier);

// Layout related
impl_from_args!(Direction, direction);
impl_from_args!(ratatui_core::layout::Flex, flex);
impl_from_args!(Layout, layout);
impl_from_args!(Constraint, constraint);
impl_from_args!(Margin, margin);
impl_from_args!(Rect, rect);
impl_from_args!(RefRect, ref_rect);
impl_from_args!(Offset, offset);
impl_from_args!(Size, size);

// Effect related
impl_from_args!(Effect, effect);
impl_from_args!(Duration, duration);
impl_from_args!(EffectTimer, effect_timer);
impl_from_args!(Interpolation, interpolation);
impl_from_args!(Motion, motion);
impl_from_args!(crate::fx::ExpandDirection, expand_direction);
impl_from_args!(RepeatMode, repeat_mode);
impl_from_args!(CellFilter, cell_filter);
impl_from_args!(ColorSpace, color_space);

// Random types
impl_from_args!(SimpleRng, simple_rng);

// Pattern types
impl_from_args!(AnyPattern, pattern);

// Wave types
impl_from_args!(Modulator, modulator);
impl_from_args!(Oscillator, oscillator);
impl_from_args!(WaveLayer, wave_layer);

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, fmt::Debug};

    use compact_str::ToCompactString;
    use ratatui_core::{
        layout::{Margin, Offset, Rect, Size},
        style::Color,
    };

    use crate::{
        dsl::{
            arguments::Arguments,
            dsl::EffectDsl,
            environment::DslEnv,
            expressions::{Expr, ExprSpan, FnCallInfo, Value},
            token_parsers::parse_ast,
            token_verification::verify_tokens,
            tokenizer::{sanitize_tokens, tokenize},
            DslError,
        },
        CellFilter, Motion, RefRect, SimpleRng,
    };

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

    fn prepare_test<'a>(args: impl Into<VecDeque<Expr>>) -> Arguments<'a> {
        // leaking, but it's fine for tests as it reduces boilerplate
        let dsl = Box::leak(Box::new(EffectDsl::new()));
        let env = Box::leak(Box::new(DslEnv::new()));

        Arguments::new(args.into(), dsl, env, ExprSpan::default())
    }

    fn assert_result<'a, T: Debug>(
        input: &str,
        expected: T,
        f: impl Fn(&mut Arguments<'a>) -> Result<T, DslError>,
    ) {
        // leaking, but it's fine for tests as it reduces boilerplate
        let dsl = Box::leak(Box::new(EffectDsl::new()));
        let env = Box::leak(Box::new(DslEnv::new()));

        let args = parse_expr(input);
        let mut args = Arguments::new([args].into(), dsl, env, ExprSpan::default());
        let result = f(&mut args).expect("value from arguments");

        assert_eq!(format!("{result:#?}"), format!("{expected:#?}"));
    }

    #[test]
    fn test_simple_rng() {
        assert_result(r#"SimpleRng::new(12345)"#, SimpleRng::new(12345), |args| {
            args.simple_rng()
        });
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_numeric_parsing() {
        let span = ExprSpan::new(0, 0);
        let mut args = prepare_test(vec![
            Expr::Literal(Value::U32(42), span),
            Expr::Literal(Value::F32(3.14), span),
        ]);

        assert_eq!(args.read_u32(), Ok(42));
        assert_eq!(args.read_f32(), Ok(3.14));
        assert_eq!(
            args.read_u32(),
            Err(DslError::MissingArgument { position: 3, name: "u32", location: span })
        );
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_array_parsing() {
        let span = ExprSpan::new(0, 0);
        // test a
        let mut args = prepare_test(vec![Expr::ArrayRef(
            vec![Expr::Literal(Value::F32(10.0), span), Expr::Literal(Value::F32(3.14), span)],
            span,
        )]);

        let floats = args.array(Arguments::read_f32).unwrap();
        assert_eq!(floats, vec![10.0, 3.14]);

        // test b
        let mut args = prepare_test(vec![Expr::ArrayRef(
            vec![
                Expr::Literal(Value::String("a".into()), span),
                Expr::Literal(Value::String("b".into()), span),
                Expr::Literal(Value::String("c".into()), span),
            ],
            span,
        )]);

        let strings = args.array(Arguments::string).unwrap();
        assert_eq!(strings, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_option_parsing() {
        let span = ExprSpan::new(0, 0);
        let mut args = prepare_test(vec![Expr::OptionSome(
            Box::new(Expr::Array(
                vec![
                    Expr::Literal(Value::U32(1), span),
                    Expr::Literal(Value::U32(2), span),
                    Expr::Literal(Value::U32(3), span),
                ],
                span,
            )),
            span,
        )]);

        let inner_arg = args
            .option(|args| args.array(Arguments::read_u32))
            .unwrap();
        assert_eq!(inner_arg, Some(vec![1, 2, 3]));

        let mut args = prepare_test(vec![Expr::Literal(Value::OptionNone, span)]);
        let inner_arg = args.option(Arguments::read_u32).unwrap();
        assert_eq!(inner_arg, None);
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
        assert_eq!(
            args.string(),
            Err(DslError::WrongArgumentType {
                location: span,
                expected: "string",
                actual: "u32".into()
            })
        );
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
        assert_eq!(
            args.color(),
            Err(DslError::MissingArgument { position: 3, name: "color", location: span })
        );
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
        assert_eq!(
            args.motion(),
            Err(DslError::MissingArgument { position: 3, name: "motion", location: span })
        );
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
        let mut args = prepare_test(vec![Expr::FnCall {
            call: FnCallInfo {
                name: "fx::test".to_compact_string(),
                args: test_args,
                span,
            },
            self_fns: vec![],
        }]);

        let result = args.effect();
        assert!(result.is_err());
        assert_eq!(
            result.expect_err("expected error"),
            DslError::UnknownEffect { name: "test".to_compact_string(), location: span }
        );
    }

    #[test]
    fn test_cell_filter_parsing() {
        let span = ExprSpan::new(0, 0);
        // Test with a CellFilter constructor function call
        let mut args = prepare_test(vec![Expr::FnCall {
            call: FnCallInfo {
                name: "CellFilter::FgColor".to_compact_string(),
                args: vec![Expr::Literal(Value::Color(Color::Red), span)],
                span,
            },
            self_fns: vec![],
        }]);

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
                span,
            },
            self_fns: vec![],
        };

        // Test with CellFilter::AllOf using vec![] macro
        let mut args = prepare_test(vec![Expr::FnCall {
            call: FnCallInfo {
                name: "CellFilter::AllOf".to_compact_string(),
                args: vec![Expr::Macro {
                    name: "vec".into(),
                    args: vec![text_filter, fg_filter],
                    span,
                }],
                span,
            },
            self_fns: vec![],
        }]);

        let result = args.cell_filter().unwrap();
        if let CellFilter::AllOf(filters) = result {
            assert_eq!(filters.len(), 2);
            assert!(matches!(filters[0], CellFilter::Text));
            assert!(matches!(filters[1], CellFilter::FgColor(Color::Red)));
        } else {
            panic!("Expected CellFilter::AllOf, got {result:?}");
        }
    }

    #[test]
    fn test_style_constructor_parsing() {
        let span = ExprSpan::new(0, 0);
        // Test with Style constructor
        let mut args = prepare_test(vec![Expr::FnCall {
            call: FnCallInfo {
                name: "Style::new".to_compact_string(),
                args: vec![],
                span,
            },
            self_fns: vec![FnCallInfo {
                name: "fg".to_compact_string(),
                args: vec![Expr::Literal(Value::Color(Color::Red), span)],
                span,
            }],
        }]);

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
        assert_eq!(
            args.read_u32(),
            Err(DslError::MissingArgument { position: 4, name: "u32", location: span })
        );
    }

    #[test]
    fn test_u16_conversion() {
        let span = ExprSpan::new(0, 0);
        let mut args = prepare_test(vec![
            Expr::Literal(Value::U32(65535), span), // Max u16
            Expr::Literal(Value::U32(65536), span), // Too large for u16
        ]);

        assert_eq!(args.read_u16(), Ok(65535));
        assert_eq!(
            args.read_u16(),
            Err(DslError::CastOverflow { location: span, from: "u32", to: "u16" })
        ); // Truncated
    }

    #[test]
    fn test_empty_args() {
        let mut args = prepare_test([]);

        let missing = |idx, name| {
            Err(DslError::MissingArgument { position: idx, name, location: ExprSpan::default() })
        };

        assert_eq!(args.duration(), missing(1, "duration"));
    }

    #[test]
    fn test_ref_rect_constructors() {
        // Test RefRect::new with a simple rect
        let expected = RefRect::new(Rect::new(10, 20, 30, 40));
        assert_result(
            "RefRect::new(Rect::new(10, 20, 30, 40))",
            expected,
            Arguments::ref_rect,
        );

        // Test RefRect::default
        let expected = RefRect::default();
        assert_result("RefRect::default()", expected, Arguments::ref_rect);
    }

    #[test]
    fn test_ref_rect_with_chained_rect() {
        // Test RefRect::new with a chained rect
        let expected = RefRect::new(
            Rect::new(0, 0, 100, 50)
                .inner(Margin::new(5, 2))
                .intersection(Rect::new(10, 10, 80, 30)),
        );

        let input = r#"RefRect::new(
            Rect::new(0, 0, 100, 50)
                .inner(Margin::new(5, 2))
                .intersection(Rect::new(10, 10, 80, 30))
        )"#;

        assert_result(input, expected, Arguments::ref_rect);
    }

    #[test]
    fn test_cell_filter_ref_area() {
        // Test CellFilter::RefArea with RefRect::new
        let expected = CellFilter::RefArea(RefRect::new(Rect::new(5, 10, 20, 15)));
        assert_result(
            "CellFilter::RefArea(RefRect::new(Rect::new(5, 10, 20, 15)))",
            expected,
            Arguments::cell_filter,
        );

        // Test CellFilter::RefArea with RefRect::default
        let expected = CellFilter::RefArea(RefRect::default());
        assert_result(
            "CellFilter::RefArea(RefRect::default())",
            expected,
            Arguments::cell_filter,
        );
    }

    #[test]
    fn test_compound_cell_filter_with_ref_rect() {
        // Test RefRect in compound cell filters
        let ref_rect1 = RefRect::new(Rect::new(0, 0, 50, 25));
        let ref_rect2 = RefRect::new(Rect::new(25, 12, 50, 25));

        let expected = CellFilter::AllOf(vec![
            CellFilter::RefArea(ref_rect1),
            CellFilter::Not(Box::new(CellFilter::RefArea(ref_rect2))),
        ]);

        let input = r#"CellFilter::AllOf(vec![
            CellFilter::RefArea(RefRect::new(Rect::new(0, 0, 50, 25))),
            CellFilter::Not(Box::new(CellFilter::RefArea(RefRect::new(Rect::new(25, 12, 50, 25)))))
        ])"#;

        assert_result(input, expected, Arguments::cell_filter);
    }

    #[test]
    fn test_size_constructor_parsing() {
        // Test Size::new constructor
        let expected = Size::new(80, 24);
        assert_result("Size::new(80, 24)", expected, Arguments::size);
    }

    #[test]
    fn test_size_struct_init_parsing() {
        // Test Size struct initialization
        let expected = Size { width: 120, height: 40 };
        assert_result("Size { width: 120, height: 40 }", expected, Arguments::size);
    }

    #[test]
    fn test_offset_struct_init_parsing() {
        // Test Offset struct initialization
        let expected = Offset { x: 5, y: -3 };
        assert_result("Offset { x: 5, y: -3 }", expected, Arguments::offset);
    }

    #[test]
    fn test_cell_filter_method_chaining() {
        // Test CellFilter with method chaining
        let expected = CellFilter::FgColor(Color::Red)
            .negated()
            .into_static();

        let input = r#"CellFilter::FgColor(Color::Red)
            .negated()
            .into_static()"#;

        assert_result(input, expected, Arguments::cell_filter);
    }

    #[test]
    fn negative_test_arg_in_zero_arg_fn() {
        // Test that zero-argument functions reject arguments when provided
        let test_cases = vec![
            ("Style::default(42)", "style"),
            ("Style::new(42)", "style"),
            ("Layout::default(42)", "layout"),
            ("CheckerboardPattern::default(42)", "pattern"),
            ("CoalescePattern::new(42)", "pattern"),
            ("CoalescePattern::default(42)", "pattern"),
            ("DiagonalPattern::top_left_to_bottom_right(42)", "pattern"),
            ("DiagonalPattern::top_right_to_bottom_left(42)", "pattern"),
            ("DiagonalPattern::bottom_left_to_top_right(42)", "pattern"),
            ("DiagonalPattern::bottom_right_to_top_left(42)", "pattern"),
            ("DissolvePattern::new(42)", "pattern"),
            ("DissolvePattern::default(42)", "pattern"),
            ("RadialPattern::center(42)", "pattern"),
            ("RefRect::default(42)", "ref_rect"),
        ];

        for (input, method_name) in test_cases {
            let dsl = EffectDsl::new();
            let env = DslEnv::new();

            let expr = parse_expr(input);
            let mut args = Arguments::new([expr].into(), &dsl, &env, ExprSpan::default());

            let result = match method_name {
                "style" => args.style().map(|_| ()),
                "layout" => args.layout().map(|_| ()),
                "pattern" => args.pattern().map(|_| ()),
                "ref_rect" => args.ref_rect().map(|_| ()),
                _ => panic!("Unknown method: {method_name}"),
            };

            assert!(
                result.is_err(),
                "Expected error for input '{input}', but got Ok"
            );

            // Verify it's specifically an InvalidArgumentLength error
            if let Err(DslError::InvalidArgumentLength { expected, actual, .. }) = result {
                assert_eq!(expected, 0, "Expected 0 arguments for '{input}'");
                assert_eq!(actual, 1, "Got 1 argument for '{input}'");
            } else {
                panic!("Expected InvalidArgumentLength error for '{input}', got {result:?}");
            }
        }
    }

    #[test]
    fn test_pattern_method_chaining() {
        use crate::pattern::{
            AnyPattern, CheckerboardPattern, DiagonalPattern, RadialPattern, SweepPattern,
        };

        // RadialPattern method chaining
        let expected = AnyPattern::Radial(RadialPattern::center().with_transition_width(3.5));
        assert_result(
            "RadialPattern::center().with_transition_width(3.5)",
            expected,
            Arguments::pattern,
        );

        let expected = AnyPattern::Radial(RadialPattern::center().with_center((0.3, 0.7)));
        assert_result(
            "RadialPattern::center().with_center(0.3, 0.7)",
            expected,
            Arguments::pattern,
        );

        let expected = AnyPattern::Radial(
            RadialPattern::center()
                .with_center((0.2, 0.8))
                .with_transition_width(2.0),
        );
        assert_result(
            "RadialPattern::center().with_center(0.2, 0.8).with_transition_width(2.0)",
            expected,
            Arguments::pattern,
        );

        // CheckerboardPattern method chaining
        let expected = AnyPattern::Checkerboard(
            CheckerboardPattern::with_cell_size(2).with_transition_width(1.5),
        );
        assert_result(
            "CheckerboardPattern::with_cell_size(2).with_transition_width(1.5)",
            expected,
            Arguments::pattern,
        );

        // DiagonalPattern method chaining
        let expected = AnyPattern::Diagonal(
            DiagonalPattern::top_left_to_bottom_right().with_transition_width(4.0),
        );
        assert_result(
            "DiagonalPattern::top_left_to_bottom_right().with_transition_width(4.0)",
            expected,
            Arguments::pattern,
        );

        // CoalescePattern method chaining (clone only) - test that it compiles and returns a
        // Coalesce variant
        let result = {
            let dsl = Box::leak(Box::new(EffectDsl::new()));
            let env = Box::leak(Box::new(DslEnv::new()));
            let args = parse_expr("CoalescePattern::from(SimpleRng::default()).clone()");
            let mut args = Arguments::new([args].into(), dsl, env, ExprSpan::default());
            args.pattern()
                .expect("CoalescePattern with clone should work")
        };
        assert!(matches!(result, AnyPattern::Coalesce(_)));

        // DissolvePattern method chaining (clone only) - test that it compiles and returns a
        // Dissolve variant
        let result = {
            let dsl = Box::leak(Box::new(EffectDsl::new()));
            let env = Box::leak(Box::new(DslEnv::new()));
            let args = parse_expr("DissolvePattern::from(SimpleRng::default()).clone()");
            let mut args = Arguments::new([args].into(), dsl, env, ExprSpan::default());
            args.pattern()
                .expect("DissolvePattern with clone should work")
        };
        assert!(matches!(result, AnyPattern::Dissolve(_)));

        // SweepPattern method chaining (clone only)
        let expected = AnyPattern::Sweep(SweepPattern::left_to_right(5));
        assert_result(
            "SweepPattern::left_to_right(5).clone()",
            expected,
            Arguments::pattern,
        );
    }

    // ── Wave type DSL parsing tests ──────────────────────────────────

    #[test]
    fn test_modulator_parsing() {
        use crate::wave::Modulator;

        // Simple constructor
        assert_result(
            "Modulator::sin(1.0, 0.0, 0.5)",
            Modulator::sin(1.0, 0.0, 0.5),
            Arguments::modulator,
        );

        // Constructor with method chain
        assert_result(
            "Modulator::cos(0.5, 1.0, 0.0).phase(1.5).intensity(0.8).on_amplitude()",
            Modulator::cos(0.5, 1.0, 0.0)
                .phase(1.5)
                .intensity(0.8)
                .on_amplitude(),
            Arguments::modulator,
        );
    }

    #[test]
    fn test_oscillator_parsing() {
        use crate::wave::{Modulator, Oscillator};

        // Simple constructor
        assert_result(
            "Oscillator::sin(1.0, 0.0, 0.0)",
            Oscillator::sin(1.0, 0.0, 0.0),
            Arguments::oscillator,
        );

        // Constructor with method chain and nested modulator
        assert_result(
            "Oscillator::cos(0.5, 1.0, 0.0).phase(0.5).modulated_by(Modulator::sin(1.0, 0.0, 0.5))",
            Oscillator::cos(0.5, 1.0, 0.0)
                .phase(0.5)
                .modulated_by(Modulator::sin(1.0, 0.0, 0.5)),
            Arguments::oscillator,
        );
    }

    #[test]
    fn test_wave_layer_parsing() {
        use crate::wave::{Oscillator, WaveLayer};

        // Simple constructor
        assert_result(
            "WaveLayer::new(Oscillator::sin(1.0, 0.0, 0.0))",
            WaveLayer::new(Oscillator::sin(1.0, 0.0, 0.0)),
            Arguments::wave_layer,
        );

        // Constructor with combinator, amplitude, and abs
        assert_result(
            "WaveLayer::new(Oscillator::sin(1.0, 0.0, 0.0)).multiply(Oscillator::cos(0.5, 1.0, 0.0)).amplitude(0.5).abs()",
            WaveLayer::new(Oscillator::sin(1.0, 0.0, 0.0))
                .multiply(Oscillator::cos(0.5, 1.0, 0.0))
                .amplitude(0.5)
                .abs(),
            Arguments::wave_layer,
        );

        // Constructor with power post-transform
        assert_result(
            "WaveLayer::new(Oscillator::sin(1.0, 0.0, 0.0)).power(2)",
            WaveLayer::new(Oscillator::sin(1.0, 0.0, 0.0)).power(2),
            Arguments::wave_layer,
        );
    }

    #[test]
    fn test_modulator_dsl_roundtrip() {
        use crate::{dsl::DslFormat, wave::Modulator};

        let modulator = Modulator::sin(1.0, 0.0, 0.5)
            .phase(0.3)
            .intensity(0.7)
            .on_amplitude();
        let dsl_str = modulator.dsl_format();
        assert_result(&dsl_str, modulator, Arguments::modulator);
    }

    #[test]
    fn test_oscillator_dsl_roundtrip() {
        use crate::{
            dsl::DslFormat,
            wave::{Modulator, Oscillator},
        };

        let oscillator = Oscillator::triangle(2.0, 1.0, 0.5)
            .phase(0.25)
            .modulated_by(Modulator::cos(1.0, 0.0, 0.0).intensity(0.5));
        let dsl_str = oscillator.dsl_format();
        assert_result(&dsl_str, oscillator, Arguments::oscillator);
    }

    #[test]
    fn test_wave_layer_dsl_roundtrip() {
        use crate::{
            dsl::DslFormat,
            wave::{Oscillator, WaveLayer},
        };

        // Layer with combinator, amplitude, and post-transform
        let layer = WaveLayer::new(Oscillator::sin(1.0, 0.0, 0.0))
            .average(Oscillator::cos(0.5, 1.0, 0.0))
            .amplitude(0.75)
            .power(3);
        let dsl_str = layer.dsl_format();
        assert_result(&dsl_str, layer, Arguments::wave_layer);

        // Layer with abs
        let layer = WaveLayer::new(Oscillator::sawtooth(1.0, 0.0, 0.0)).abs();
        let dsl_str = layer.dsl_format();
        assert_result(&dsl_str, layer, Arguments::wave_layer);
    }

    #[test]
    fn test_wave_pattern_dsl_roundtrip() {
        use crate::{
            dsl::DslFormat,
            pattern::{AnyPattern, WavePattern},
            wave::{Oscillator, WaveLayer},
        };

        // Pattern with custom transition width
        let pattern = AnyPattern::Wave(
            WavePattern::new(WaveLayer::new(Oscillator::sin(1.0, 0.5, 0.0)))
                .with_contrast(2)
                .with_transition_width(0.3),
        );
        let dsl_str = pattern.dsl_format();
        assert_result(
            dsl_str
                .strip_prefix("AnyPattern::Wave(")
                .unwrap()
                .strip_suffix(')')
                .unwrap(),
            pattern,
            Arguments::pattern,
        );

        // Pattern with default transition width (should not emit .with_transition_width)
        let pattern_default = WavePattern::new(WaveLayer::new(Oscillator::cos(0.5, 1.0, 0.0)));
        let dsl_str = pattern_default.dsl_format();
        assert!(!dsl_str.contains("with_transition_width"));
    }
}
