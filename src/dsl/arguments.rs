use crate::dsl::environment::DslEnv;
use crate::dsl::dsl::EffectDsl;
use crate::dsl::DslError;
use crate::{CellFilter, Duration, Effect, EffectTimer, Interpolation, Motion};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::prelude::{Color, Style};
use std::collections::VecDeque;
use std::fmt;
use std::fmt::Formatter;
use compact_str::{CompactString, ToCompactString};
use ratatui::style::Modifier;
use crate::dsl::expressions::{Expr, FnCallInfo, Value};
use crate::fx::RepeatMode;

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
pub struct Arguments<'a> {
    args: VecDeque<Expr>,
    vars: &'a DslEnv,
    context: &'a EffectDsl,
    initial_arg_count: usize,
}

impl<'a> Arguments<'a> {
    pub(super) fn new(
        args: VecDeque<Expr>,
        context: &'a EffectDsl,
        vars: &'a DslEnv
    ) -> Self {
        let initial_arg_count = args.len();
        Self { args, vars, context, initial_arg_count }
    }

    pub(super) fn extract_with<T>(
        args: Vec<Expr>,
        context: &'a EffectDsl,
        vars: &'a DslEnv,
        get: impl Fn(&mut Self) -> Result<T, DslError>
    ) -> Result<T, DslError> {
        let mut args = Self::new(args.into(), context, vars);
        get(&mut args)
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
            Expr::FnCall(FnCallInfo { name, args }) => Ok(match name.as_str() {
                "Duration::from_millis" => {
                    let ms = self.extract_nested(args, Arguments::read_u32)?;
                    Duration::from_millis(ms as _)
                },
                "Duration::from_secs_f32" => {
                    let seconds = self.extract_nested(args, Arguments::read_f32)?;
                    Duration::from_secs_f32(seconds)
                },
                _ => self.expected_type("duration", name)?,
            }),
            Expr::Literal(v)  => match v {
                Value::Duration(d)       => Ok(d),
                Value::U32(ms)           => Ok(Duration::from_millis(ms as _)),
                e                        => self.expected_type("duration", e.format()),
            },

            Expr::Var { name, self_fns } => self.bound_var(name),
            e                            => self.expected_type_expr("duration", e),
        }
    }

    /// Consumes the next argument and returns an [`EffectTimer`].
    pub fn effect_timer(&mut self) -> Result<EffectTimer, DslError> {
        match self.next("timer")? {
            Expr::FnCall(FnCallInfo { name, args }) => Ok(match name.as_str() {
                "EffectTimer::from_ms" => {
                    let mut inner_args = self.nested_args(args, 2)?;
                    let ms = inner_args.read_u32()?;
                    let interpolation = inner_args.interpolation()?;
                    EffectTimer::from_ms(ms, interpolation)
                },
                "EffectTimer::new" => {
                    let mut inner_args = self.nested_args(args, 2)?;
                    let duration = inner_args.duration()?;
                    let interpolation = inner_args.interpolation()?;
                    EffectTimer::new(duration, interpolation)
                },
                _ => self.expected_type("timer", name)?,
            }),
            Expr::Literal(Value::Timer(t))     => Ok(t),
            Expr::Literal(Value::U32(ms))      => Ok(ms.into()),
            Expr::Var { name: name, self_fns } => self.bound_var(name),
            e                                  => self.expected_type_expr("timer", e),
        }
    }

    /// Consumes the next argument and returns a [`Color`].
    pub fn cell_filter(&mut self) -> Result<CellFilter, DslError> {
        match self.next("cell_filter")? {
            Expr::CellFilter { filter_type, arguments } => {
                let mut args = Arguments::new(arguments.into(), self.context, self.vars);
                match filter_type {
                    "All"        => Ok(CellFilter::All),
                    "FgColor"    => Ok(CellFilter::FgColor(args.color()?)),
                    "BgColor"    => Ok(CellFilter::BgColor(args.color()?)),
                    "Inner"      => Ok(CellFilter::Inner(args.margin()?)),
                    "Outer"      => Ok(CellFilter::Outer(args.margin()?)),
                    "Text"       => Ok(CellFilter::Text),
                    "AllOf"      => Ok(CellFilter::AllOf(args.array(Arguments::cell_filter)?)),
                    "AnyOf"      => Ok(CellFilter::AnyOf(args.array(Arguments::cell_filter)?)),
                    "NoneOf"     => Ok(CellFilter::NoneOf(args.array(Arguments::cell_filter)?)),
                    "Not"        => Ok(CellFilter::Not(Box::new(args.cell_filter()?))),
                    "Layout"     => Ok(CellFilter::Layout(args.layout()?, args.read_u16()?)),
                    "PositionFn" => Ok(CellFilter::PositionFn(todo!("fn"))),
                    "EvalCell"   => Ok(CellFilter::EvalCell(todo!("fn"))),
                    _            => Err(DslError::UnknownCellFilter {
                        name: filter_type.to_compact_string(),
                    })?,
                }
            }
            Expr::Literal(Value::CellFilter(f)) => Ok(f),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e                                   => self.expected_type_expr("cell_filter", e),
        }
    }

    /// Consumes the next argument and returns a [`Constraint`].
    pub fn constraint(&mut self) -> Result<Constraint, DslError> {
        use Constraint::*;

        match self.next("constraint")? {
            Expr::FnCall(FnCallInfo { name, args }) => Ok(match name.as_str() {
                "Constraint::Min"        => Min(self.extract_nested(args, Arguments::read_u16)?),
                "Constraint::Max"        => Max(self.extract_nested(args, Arguments::read_u16)?),
                "Constraint::Length"     => Length(self.extract_nested(args, Arguments::read_u16)?),
                "Constraint::Percentage" => Percentage(self.extract_nested(args, Arguments::read_u16)?),
                "Constraint::Fill"       => Fill(self.extract_nested(args, Arguments::read_u16)?),
                "Constraint::Ratio"      => {
                    let mut inner_args = self.nested_args(args, 2)?;
                    let a = inner_args.read_u32()?;
                    let b = inner_args.read_u32()?;
                    Ratio(a, b)
                },
                _ => self.expected_type("constraint", name)?,
            }),
            Expr::Literal(Value::Constraint(c)) => Ok(c),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e               => self.expected_type("constraint", e.type_name().into()),
        }
    }

    /// Consumes the next argument and returns a [`Direction`].
    pub fn direction(&mut self) -> Result<Direction, DslError> {
        match self.next("direction")? {
            Expr::Literal(Value::Direction(d)) => Ok(d),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e               => self.expected_type("direction", e.type_name().into()),
        }
    }

    /// Consumes the next argument and returns a [`Layout`].
    pub fn layout(&mut self) -> Result<Layout, DslError> {
        // First get the layout expression
        let layout_expr = self.next("layout")?;

        // Helper closure that doesn't capture self
        let apply_single_fn = |layout: Layout, f: FnCallInfo, context: &EffectDsl, vars: &DslEnv| -> Result<Layout, DslError> {
            let mut args = Arguments::new(f.args.into(), context, vars);
            match f.name.as_str() {
                "constraints"       => Ok(layout.constraints(args.array(Arguments::constraint)?)),
                "margin"            => Ok(layout.margin(args.read_u16()?)),
                "horizontal_margin" => Ok(layout.horizontal_margin(args.read_u16()?)),
                "vertical_margin"   => Ok(layout.vertical_margin(args.read_u16()?)),
                "spacing"           => Ok(layout.spacing(args.read_u16()?)),
                _                   => Err(DslError::WrongArgumentType {
                    actual: f.name,
                    expected: "layout method",
                    position: 0,
                }),
            }
        };

        // Process the layout expression
        match layout_expr {
            Expr::Layout { expr, self_fns } => {
                let base_layout = match *expr {
                    Expr::FnCall(FnCallInfo { name, args }) => {
                        match name.as_str() {
                            "Layout::horizontal" => {
                                let constraints = self.extract_nested(
                                    args, |a| a.array(Arguments::constraint))?;

                                Ok(Layout::horizontal(constraints))
                            },
                            "Layout::vertical" => {
                                let constraints = self.extract_nested(
                                    args, |a| a.array(Arguments::constraint))?;
                                Ok(Layout::vertical(constraints))
                            },
                            "Layout::new" => {
                                let mut inner_args = self.nested_args(args, 2)?;
                                let direction = inner_args.direction()?;
                                let constraints = inner_args.array(Arguments::constraint)?;
                                Ok(Layout::new(direction, constraints))
                            },
                            _ => self.expected_type("layout", name),
                        }
                    },
                    e => self.expected_type_expr("layout", e),
                }?;

                // Apply method chains
                self_fns.into_iter().try_fold(base_layout, |layout, f| {
                    apply_single_fn(layout, f, self.context, self.vars)
                })
            },
            Expr::Var { name, self_fns } => self.bound_var(name),
            e               => self.expected_type_expr("layout", e),
        }
    }

    /// Consumes the next argument and returns a [`Interpolation`].
    pub fn interpolation(&mut self) -> Result<Interpolation, DslError> {
        match self.next("interpolation")? {
            Expr::Literal(Value::Interpolation(i)) => Ok(i),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e                          => self.expected_type_expr("interpolation", e),
        }
    }

    /// Consumes the next argument and returns a `u8`.
    pub fn read_u8(&mut self) -> Result<u8, DslError> {
        u8::try_from(self.read_u32()?)
            .map_err(|_| DslError::CastOverflow {
                position: self.initial_arg_count - self.args.len() - 1, // -1 for the current argument
                from: "u32",
                to: "u8",
            })
    }

    /// Consumes the next argument and returns a `u16`.
    pub fn read_u16(&mut self) -> Result<u16, DslError> {
        u16::try_from(self.read_u32()?)
            .map_err(|_| DslError::CastOverflow {
                position: self.initial_arg_count - self.args.len() - 1, // -1 for the current argument
                from: "u32",
                to: "u16",
            })
    }

    /// Consumes the next argument and returns a `u32`.
    pub fn read_u32(&mut self) -> Result<u32, DslError> {
        match self.next("u32")? {
            Expr::Literal(Value::U32(u)) => Ok(u),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e                            => self.expected_type_expr("u32", e),
        }
    }

    /// Consumes the next argument and returns a `f32`.
    pub fn read_into_f32(&mut self) -> Result<f32, DslError> {
        match self.next("f32")? {
            Expr::Literal(Value::F32(f)) => Ok(f),
            Expr::Literal(Value::U32(v)) => Ok(v as f32),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e                            => self.expected_type_expr("f32", e),
        }
    }

    /// Consumes the next argument and returns a `f32`.
    pub fn read_f32(&mut self) -> Result<f32, DslError> {
        match self.next("f32")? {
            Expr::Literal(Value::F32(f)) => Ok(f),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e                            => self.expected_type_expr("f32", e),
        }
    }

    /// Consumes the next argument and returns a [`CompactString`].
    pub fn string(&mut self) -> Result<CompactString, DslError> {
        match self.next("string")? {
            Expr::Literal(Value::String(s)) => Ok(s),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e                               => self.expected_type_expr("string", e),
        }
    }

    /// Consumes the next argument and returns an `Option<T>`.
    pub fn option<T: Clone + 'static>(
        &mut self,
        inner: impl Fn(&mut Self) -> Result<T, DslError>
    ) -> Result<Option<T>, DslError> {
        match self.next("option")? {
            Expr::Literal(Value::OptionNone) => Ok(None),
            Expr::OptionSome(expr)     => {
                let mut args = self.nested_args(vec![*expr], 1)?;
                inner(&mut args).map(Some)
            },
            Expr::Var { name, self_fns } => self.bound_var(name),
            e                          => self.expected_type_expr("option", e),
        }
    }

    /// Consumes the next argument and returns an [`Effect`].
    pub fn effect(&mut self) -> Result<Effect, DslError> {
        match self.next("effect")? {
            Expr::Fx { name, arguments, self_fns } =>
                self.compile_effect(Expr::Fx { name, arguments, self_fns }),
            Expr::Sequence { effects, self_fns } =>
                self.compile_effect(Expr::Sequence { effects, self_fns }),
            Expr::Parallel { effects, self_fns } =>
                self.compile_effect(Expr::Parallel { effects, self_fns }),

            Expr::Var { name, self_fns } => self.bound_var(name),
            e               => self.expected_type_expr("effect", e),
        }
    }

    /// Consumes the next argument and returns a [`Color`].
    pub fn color(&mut self) -> Result<Color, DslError> {
        match self.next("color")? {
            Expr::FnCall(FnCallInfo { name, args }) => Ok(match name.as_str() {
                "Color::Rgb" => {
                    let mut inner_args = self.nested_args(args, 3)?;
                    let r = inner_args.read_u8()?;
                    let g = inner_args.read_u8()?;
                    let b = inner_args.read_u8()?;
                    Color::Rgb(r, g, b)
                },
                "Color::from_u32" => {
                    Color::from_u32(self.extract_nested(args, Arguments::read_u32)?)
                }
                "Color::Indexed" => {
                    Color::Indexed(self.extract_nested(args, Arguments::read_u8)?)
                }
                _ => self.expected_type("color", name)?,
            }),
            Expr::Literal(Value::Color(c)) => Ok(c),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e                              => self.expected_type_expr("color", e),
        }
    }

    /// Consumes the next argument and returns a [`Modifier`].
    pub fn modifier(&mut self) -> Result<Modifier, DslError> {
        match self.next("modifier")? {
            Expr::Literal(Value::Modifier(m)) => Ok(m),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e                                 => self.expected_type_expr("modifier", e),
        }
    }

    /// Consumes the next argument and returns a [`Style`].
    pub fn style(&mut self) -> Result<Style, DslError> {
        match self.next("style")? {
            Expr::Literal(Value::Style(s))  => Ok(s),
            Expr::Style(methods)            => self.compile_style(methods),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e                               => self.expected_type("style", e.type_name().into()),
        }
    }

    /// Consumes the next argument and returns a [`Motion`].
    pub fn motion(&mut self) -> Result<Motion, DslError> {
        match self.next("motion")? {
            Expr::Literal(Value::Motion(m))  => Ok(m),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e                                => self.expected_type("motion", e.type_name().into()),
        }
    }

    /// Consumes the next argument and returns a [`RepeatMode`].
    pub fn repeat_mode(&mut self) -> Result<RepeatMode, DslError> {
        match self.next("repeat_mode")? {
            Expr::FnCall(FnCallInfo { name, args }) => Ok(match name.as_str() {
                "RepeatMode::Forever" => RepeatMode::Forever,
                "RepeatMode::Times"   => RepeatMode::Times(self.extract_nested(args, Arguments::read_u32)?),
                _                     => self.expected_type("repeat_mode", name)?,
            }),
            Expr::Literal(Value::RepeatMode(m))  => Ok(m),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e                                    => self.expected_type("repeat_mode", e.type_name().into()),
        }
    }

    /// Consumes the next argument and returns a [`Margin`].
    pub fn margin(&mut self) -> Result<Margin, DslError> {
        match self.next("margin")? {
            Expr::Literal(Value::Margin(m)) => Ok(m),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e                               => self.expected_type_expr("margin", e),
        }
    }

    /// Consumes the next argument and returns a [`Rect`].
    pub fn rect(&mut self) -> Result<Rect, DslError> {
        match self.next("rect")? {
            Expr::FnCall(FnCallInfo{ name, args }) => match name.as_str() {
                "Rect::new" => {
                    let mut inner_args = self.nested_args(args, 4)?;
                    let x = inner_args.read_u16()?;
                    let y = inner_args.read_u16()?;
                    let width = inner_args.read_u16()?;
                    let height = inner_args.read_u16()?;
                    Ok(Rect::new(x, y, width, height))
                },
                e => Err(DslError::UnknownFunction {
                    name: e.to_compact_string(),
                }),
            },
            Expr::Literal(Value::Rect(r)) => Ok(r),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e                             => self.expected_type_expr("rect", e),
        }
    }


    /// Consumes the next argument and returns a `Vec<T>`.
    pub fn array<T: Clone + 'static>(
        &mut self,
        inner: impl Fn(&mut Self) -> Result<T, DslError>
    ) -> Result<Vec<T>, DslError> {
        match self.next("array")? {
            Expr::Array(exprs)    => self.map_exprs(exprs, inner),
            Expr::ArrayRef(exprs) => self.map_exprs(exprs, inner),
            Expr::Var { name, self_fns } => self.bound_var(name),
            e                     => self.expected_type_expr("array", e),
        }
    }

    pub(super) fn original_arg_count(&self) -> usize {
        self.initial_arg_count
    }

    fn map_exprs<T: Clone>(
        &mut self,
        exprs: Vec<Expr>,
        inner: impl Fn(&mut Self) -> Result<T, DslError>
    ) -> Result<Vec<T>, DslError> {
        let mut args = self.all_inner_args(exprs);
        (0..args.initial_arg_count)
            .map(|_| inner(&mut args)).collect()
    }

    fn compile_effect(&self, expr: Expr) -> Result<Effect, DslError> {
        self.context.compile(self.vars, expr)
    }

    fn compile_style(&mut self, methods: Vec<FnCallInfo>) -> Result<Style, DslError> {
        methods.into_iter().fold(Ok(Style::new()), |style, method| {
            match method.name.as_str() {
                "fg" => self.extract_nested(method.args, Arguments::color)
                    .map(|color| style.map(|s| s.fg(color)))?,

                "bg" => self.extract_nested(method.args, Arguments::color)
                    .map(|color| style.map(|s| s.bg(color)))?,

                "add_modifier" => self.extract_nested(method.args, Arguments::modifier)
                    .map(|modifier| style.map(|s| s.add_modifier(modifier)))?,

                _ => style
            }
        })
    }

    fn bound_var<T: Clone + 'static>(&self, name: impl Into<CompactString>) -> Result<T, DslError> {
        self.vars.get(name.into()).cloned()
    }

    fn next(&mut self, type_name: &'static str) -> Result<Expr, DslError> {
        self.args.pop_front()
            .ok_or(DslError::MissingArgument {
                position: self.initial_arg_count - self.args.len(),
                name: type_name,
            })
    }

    fn expected_type<T>(
        &self,
        expected: &'static str,
        actual: CompactString,
    ) -> Result<T, DslError>  {
        Err(DslError::WrongArgumentType {
            position: self.initial_arg_count - self.args.len() - 1,
            expected,
            actual
        })
    }

    fn expected_type_expr<T>(
        &self,
        expected: &'static str,
        actual: Expr,
    ) -> Result<T, DslError>  {
        self.expected_type(expected, actual.type_name().to_compact_string())
    }

    fn nested_args(&mut self, exprs: Vec<Expr>, required_arg_count: usize) -> Result<Self, DslError> {
        if exprs.len() != required_arg_count {
            return Err(DslError::InvalidArgumentLength {
                expected: required_arg_count,
                actual: exprs.len(),
            });
        }

        Ok(self.all_inner_args(exprs))
    }

    fn extract_nested<T>(
        &mut self,
        exprs: Vec<Expr>,
        inner: impl Fn(&mut Self) -> Result<T, DslError>
    ) -> Result<T, DslError> {
        let mut args = self.nested_args(exprs, 1)?;
        inner(&mut args)
    }

    fn all_inner_args(&mut self, exprs: Vec<Expr>) -> Self {
        Self::new(exprs.into(), self.context, self.vars)
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

#[cfg(test)]
mod tests {
    use crate::dsl::arguments::Arguments;
    use crate::dsl::environment::DslEnv;
    use crate::dsl::dsl::EffectDsl;
    use crate::dsl::{parsers, DslError};
    use crate::{Duration, EffectTimer, Interpolation, Motion};
    use ratatui::layout::{Margin, Rect};
    use ratatui::prelude::{Color, Style};
    use std::collections::VecDeque;
    use anpa::core::parse;
    use compact_str::ToCompactString;
    use crate::dsl::expressions::{Expr, Value};

    fn empty_env() -> DslEnv {
        DslEnv::new()
    }

    #[test]
    fn test_duration_parsing() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::Duration(Duration::from_millis(500))),
                Expr::Literal(Value::U32(1000)),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.duration(), Ok(Duration::from_millis(500)));
        assert_eq!(args.duration(), Ok(Duration::from_millis(1000)));
        assert_eq!(args.duration(), Err(DslError::MissingArgument {
            position: 2,
            name: "duration",
        }));
    }

    #[test]
    fn test_effect_timer_parsing() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Interpolation::Linear))),
                Expr::Literal(Value::U32(1000)),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.effect_timer(), Ok(EffectTimer::from_ms(500, Interpolation::Linear)));
        assert_eq!(args.effect_timer(), Ok(EffectTimer::from_ms(1000, Interpolation::Linear)));
        assert_eq!(args.effect_timer(), Err(DslError::MissingArgument {
            position: 2,
            name: "timer",
        }));
    }

    #[test]
    fn test_numeric_parsing() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::U32(42)),
                Expr::Literal(Value::F32(3.14)),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.read_u32(), Ok(42));
        assert_eq!(args.read_f32(), Ok(3.14));
        assert_eq!(args.read_u32(), Err(DslError::MissingArgument {
            position: 2,
            name: "u32",
        }));
    }

    #[test]
    fn test_array_parsing() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::ArrayRef(vec![
                    Expr::Literal(Value::F32(10.0)),
                    Expr::Literal(Value::F32(3.14)),
                ]),
            ].into(),
            &context,
            &binding
        );

        let floats = args.array(Arguments::read_f32).unwrap();
        assert_eq!(floats, vec![10.0, 3.14]);


        let mut args = Arguments::new(
            vec![
                Expr::ArrayRef(vec![
                    Expr::Literal(Value::String("a".into())),
                    Expr::Literal(Value::String("b".into())),
                    Expr::Literal(Value::String("c".into())),
                ]),
            ].into(),
            &context,
            &binding
        );

        let strings = args.array(Arguments::string).unwrap();
        assert_eq!(strings, vec!["a", "b", "c"]);

        // let mut inner_args = args.array_ref().unwrap();
        // assert_eq!(inner_args.read_u32(), Ok(42));
        // assert_eq!(inner_args.read_f32(), Ok(3.14));
        // assert_eq!(inner_args.read_u32(), Err(DslError::MissingArgument {
        //     position: 2,
        //     name: "u32",
        // }));
    }

    #[test]
    fn test_option_parsing() {
        let binding = empty_env();
        let context = EffectDsl::new();

        let mut args = Arguments::new(
            vec![
                Expr::OptionSome(Box::new(
                    Expr::Array(vec![
                        Expr::Literal(Value::U32(1)),
                        Expr::Literal(Value::U32(2)),
                        Expr::Literal(Value::U32(3)),
                    ])
                )),
            ].into(),
            &context,
            &binding
        );

        let inner_arg = args.option(|args| args.array(Arguments::read_u32)).unwrap();
        assert_eq!(inner_arg, Some(vec![1, 2, 3]));

        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::OptionNone),
            ].into(),
            &context,
            &binding
        );

        let inner_arg = args.option(Arguments::read_u32).unwrap();
        assert_eq!(inner_arg, None);
    }

    fn parse_expr(input: &str) -> Expr {
        let binding = empty_env();
        let parsing_result = parse(parsers::argument(), input);
        assert_eq!(parsing_result.state, "");
        parsing_result.result.unwrap()
    }

    #[test]
    fn test_string_parsing() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::String("hello".to_compact_string())),
                Expr::Literal(Value::U32(42)), // Wrong type
                Expr::Literal(Value::String("world".to_compact_string())),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.string(), Ok("hello".to_compact_string()));
        assert_eq!(args.string(), Err(DslError::WrongArgumentType {
            position: 1,
            expected: "string",
            actual: "literal".to_compact_string(), // fixme: should be u32?
        }));
        assert_eq!(args.string(), Ok("world".to_compact_string()));
    }

    #[test]
    fn test_color_parsing() {
        let dsl = EffectDsl::new();
        let env = empty_env();

        let expr = parse_expr("Color::Rgb(1, 2, 3)");
        let color = Arguments::extract_with(vec![expr], &EffectDsl::new(), &env, Arguments::color)
            .expect("expected color");
        assert_eq!(color, Color::Rgb(1, 2, 3));

        let expr = parse_expr("Color::from_u32(0xffaabb)");
        let color = Arguments::extract_with(vec![expr], &EffectDsl::new(), &env, Arguments::color)
            .expect("expected color");
        assert_eq!(color, Color::from_u32(0xffaabb));



        //
        //
        // let binding = empty_env();
        // let context = EffectDsl::new();
        // let mut args = Arguments::new(
        //     vec![
        //         Expr::Literal(Value::Color(Color::Red)),
        //         Expr::Literal(Value::Color(Color::Blue)),
        //     ].into(),
        //     &context,
        //     &binding
        // );
        //
        // assert_eq!(args.color(), Ok(Color::Red));
        // assert_eq!(args.color(), Ok(Color::Blue));
        // assert_eq!(args.color(), Err(DslError::MissingArgument {
        //     position: 2,
        //     name: "color",
        // }));
    }

    #[test]
    fn test_style_parsing() {
        let context = EffectDsl::new();
        let style = Style::default().fg(Color::Red);
        let binding = empty_env();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::Style(style)),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.style(), Ok(style));
        assert_eq!(args.style(), Err(DslError::MissingArgument {
            position: 1,
            name: "style",
        }));
    }

    #[test]
    fn test_motion_parsing() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::Motion(Motion::LeftToRight)),
                Expr::Literal(Value::Motion(Motion::UpToDown)),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.motion(), Ok(Motion::LeftToRight));
        assert_eq!(args.motion(), Ok(Motion::UpToDown));
        assert_eq!(args.motion(), Err(DslError::MissingArgument {
            position: 2,
            name: "motion",
        }));
    }

    #[test]
    fn test_margin_parsing() {
        let margin = Margin::new(10, 20);
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::Margin(margin)),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.margin(), Ok(margin));
        assert_eq!(args.margin(), Err(DslError::MissingArgument {
            position: 1,
            name: "margin",
        }));
    }

    #[test]
    fn test_rect_parsing() {
        let rect = Rect::new(0, 0, 100, 100);
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::Rect(rect)),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.rect(), Ok(rect));
        assert_eq!(args.rect(), Err(DslError::MissingArgument {
            position: 1,
            name: "rect",
        }));
    }

    #[test]
    fn test_effect_parsing() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Fx {
                    name: "test".to_compact_string(),
                    arguments: vec![Expr::Literal(Value::U32(500))],
                    self_fns: vec![],
                },
            ].into(),
            &context,
            &binding
        );

        let result = args.effect();
        assert!(result.is_err());
        assert_eq!(
            result.expect_err("expected error"),
            DslError::UnknownEffect {
                name: "test".to_compact_string(),
            }
        );
    }

    #[test]
    fn test_mixed_arguments() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::U32(500)),
                Expr::Literal(Value::Motion(Motion::LeftToRight)),
                Expr::Literal(Value::Color(Color::Blue)),
                Expr::Literal(Value::Timer(EffectTimer::from_ms(1000, Interpolation::Linear))),
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.read_u32(), Ok(500));
        assert_eq!(args.motion(), Ok(Motion::LeftToRight));
        assert_eq!(args.color(), Ok(Color::Blue));
        assert_eq!(args.effect_timer(), Ok(EffectTimer::from_ms(1000, Interpolation::Linear)));
        assert_eq!(args.read_u32(), Err(DslError::MissingArgument {
            position: 4,
            name: "u32",
        }));
    }

    #[test]
    fn test_u16_conversion() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(
            vec![
                Expr::Literal(Value::U32(65535)), // Max u16
                Expr::Literal(Value::U32(65536)), // Too large for u16
            ].into(),
            &context,
            &binding
        );

        assert_eq!(args.read_u16(), Ok(65535));
        assert_eq!(args.read_u16(), Err(DslError::CastOverflow {
            position: 1,
            from: "u32",
            to: "u16",
        })); // Truncated
    }

    #[test]
    fn test_empty_args() {
        let binding = empty_env();
        let context = EffectDsl::new();
        let mut args = Arguments::new(VecDeque::new(), &context, &binding);

        let missing = |idx, name| Err(DslError::MissingArgument {
            position: idx,
            name,
        });

        assert_eq!(args.duration(), missing(0, "duration"));
    }
}
