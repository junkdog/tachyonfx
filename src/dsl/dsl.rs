use core::{fmt, fmt::Formatter};

use compact_str::CompactString;

use crate::{
    dsl::{
        arguments::Arguments,
        environment::DslEnv,
        expressions::{Expr, ExprSpan, FnCallInfo},
        method_chains::ChainableMethods,
        token_parsers::parse_ast,
        token_verification::verify_tokens,
        tokenizer::{sanitize_tokens, tokenize},
        DslError, DslParseError,
    },
    fx,
    fx::{consume_tick, dissolve, never_complete, ping_pong, repeating, run_once},
    Effect,
};

/// A compiler and registry for tachyonfx effect DSL expressions.
///
/// `EffectDsl` manages a collection of compilers that can compile DSL expressions into
/// concrete effect instances. It comes pre-registered with compilers for all standard
/// tachyonfx effects.
///
/// # Examples
///
/// ```
/// use tachyonfx::dsl::EffectDsl;
///
/// // Create a new DSL compiler with all standard effects registered
/// let dsl = EffectDsl::new();
///
/// // Use the DSL to interpret effect expressions
/// let effect = dsl.compiler().compile("fx::dissolve(500)").unwrap();
/// ```
///
/// The DSL supports binding variable to effects:
///
/// ```
/// use ratatui::prelude::Color;
/// use tachyonfx::dsl::EffectDsl;
/// use tachyonfx::Motion;
///
/// let input = r#"fx::sweep_in(motion, 10, 0, c, (1000, QuadOut))"#;
///
/// let dsl = EffectDsl::new();
/// let effect = dsl.compiler()
///     .bind("motion", Motion::LeftToRight)
///     .bind("c", Color::from_u32(0x1d2021))
///     .compile(input)
///     .unwrap();
/// ```
///
/// # Extending
///
/// While `EffectDsl` comes with all standard effects pre-registered, you can register
/// additional custom effect compilers if needed:
///
/// ```
/// use tachyonfx::dsl::EffectDsl;
/// use tachyonfx::fx;
///
/// let dsl = EffectDsl::new()
///     .register("custom_effect", |args| {
///         fx::sleep(args.duration()?).into()
///     });
/// ```
#[derive(Debug, Default)]
pub struct EffectDsl {
    compilers: Vec<EffectCompiler>,
}

struct EffectCompiler {
    effect_name: &'static str,
    #[allow(clippy::type_complexity)]
    compile: Box<dyn Fn(&mut Arguments) -> Result<Effect, DslError>>,
}

impl EffectDsl {
    /// Creates a new `EffectDsl` instance with all standard effect compilers registered.
    pub fn new() -> Self {
        register_default_compilers(Self { compilers: Vec::new() })
    }

    /// Registers a new effect compiler with the DSL.
    ///
    /// This method allows extending the DSL with custom effects. The compiler function
    /// receives parsed arguments and should return a concrete `Effect` instance or
    /// `DslError` if compilation fails.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the effect as it will appear in DSL expressions (e.g.,
    ///   "my_effect" for `fx::my_effect(...)`)
    /// * `compiler` - A function that compiles DSL arguments into an `Effect`
    ///
    /// # Returns
    ///
    /// Returns self for method chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use tachyonfx::dsl::EffectDsl;
    ///
    /// let dsl = EffectDsl::new()
    ///     .register("custom_effect", |args| {
    ///         // Parse arguments and create an effect
    ///         let duration = args.duration()?;
    ///         let color = args.color()?;
    ///
    ///         // Return your custom effect
    ///         todo!("e.g. Ok(custom_effect(duration, color))")
    ///     });
    /// ```
    pub fn register(
        self,
        name: &'static str,
        compiler: impl Fn(&mut Arguments) -> Result<Effect, DslError> + 'static,
    ) -> Self {
        let mut this = self;
        this.compilers
            .push(EffectCompiler::new(name, compiler));
        this
    }

    /// Creates a new DSL compiler for executing effect expressions.
    ///
    /// The compiler maintains its own environment of bound variables and can
    /// execute DSL expressions into concrete `Effect` instances.
    ///
    /// # Returns
    ///
    /// A new `DslCompiler` instance configured with this DSL's compilers.
    ///
    /// # Examples
    ///
    /// ```
    /// use tachyonfx::dsl::EffectDsl;
    /// use ratatui_core::style::Color;
    ///
    /// let dsl = EffectDsl::new();
    /// let compiler = dsl.compiler()
    ///     .bind("fg_color", Color::Blue);
    ///
    /// // Use bound variables in expressions
    /// let effect = compiler.compile(r#"
    ///     fx::fade_to_fg(fg_color, (1000, Linear))
    /// "#);
    /// ```
    pub fn compiler(&self) -> DslCompiler<'_> {
        DslCompiler { dsl: self, environment: DslEnv::new() }
    }

    pub(super) fn compile(&self, env: &DslEnv, input: Vec<Expr>) -> Result<Effect, DslError> {
        // compile expressions leading up to last
        let remaining_expr = compile_let_bindings(input, env)?;

        match remaining_expr {
            Expr::FnCall {
                call: FnCallInfo { name, args, span }, self_fns, ..
            } => {
                let effect_name = name.strip_prefix("fx::").unwrap_or(&name);
                self.compilers
                    .iter()
                    .find(|d| d.effect_name == effect_name)
                    .ok_or(DslError::UnknownEffect {
                        name: effect_name.into(),
                        location: ExprSpan::default(),
                    })
                    .and_then(|d| {
                        let mut args = Arguments::new(args.into(), self, env, span);
                        let effect = (d.compile)(&mut args)?.fold_fns(self_fns, self, env);

                        match () {
                            _ if effect.is_err() => effect,
                            _ if !args.remaining_args().is_empty() => {
                                Err(DslError::InvalidArgumentLength {
                                    expected: args.original_arg_count()
                                        - args.remaining_args().len(),
                                    actual: args.original_arg_count(),
                                    location: args
                                        .remaining_args()
                                        .iter()
                                        .next()
                                        .unwrap()
                                        .span(),
                                })
                            },
                            _ => effect,
                        }
                    })
            },
            Expr::Sequence { effects, self_fns, span } => {
                let mut args = Arguments::new(effects.into(), self, env, span);
                let effects = (0..args.remaining_arg_count())
                    .map(|_| args.effect())
                    .collect::<Result<Vec<Effect>, DslError>>()?;

                fx::sequence(&effects).fold_fns(self_fns, self, env)
            },
            Expr::Parallel { effects, self_fns, span } => {
                let mut args = Arguments::new(effects.into(), self, env, span);
                let effects = (0..args.remaining_arg_count())
                    .map(|_| args.effect())
                    .collect::<Result<Vec<Effect>, DslError>>()?;

                fx::parallel(&effects).fold_fns(self_fns, self, env)
            },
            Expr::Var { name, self_fns, span } => env
                .bound_var::<Effect>(self, name, span)
                .and_then(|effect| effect.fold_fns(self_fns, self, env)),
            ref e => Err(DslError::InvalidExpression {
                expected: "effect",
                actual: remaining_expr.type_name(),
                location: e.span(),
            }),
        }
    }
}

fn compile_let_bindings(expr: Vec<Expr>, env: &DslEnv) -> Result<Expr, DslError> {
    let mut expr = expr;
    let final_effect_expr = expr.remove(expr.len() - 1);

    let err = expr
        .into_iter()
        .map(|e| match e {
            Expr::LetBinding { name, let_expr, .. } => {
                env.bind_local(name, *let_expr);
                None
            },
            e => Some(DslError::InvalidExpression {
                expected: "let binding",
                actual: e.type_name(),
                location: e.span(),
            }),
        })
        .find(Option::is_some);

    if let Some(Some(err)) = err {
        Err(err)
    } else {
        Ok(final_effect_expr) // effect expr
    }
}

/// A compiler that can execute tachyonfx DSL expressions into concrete effects.
///
/// The compiler maintains its own environment of bound variables that can be referenced
/// in effect expressions. It uses its parent `EffectDsl` to compile the expressions.
///
/// ### See also:
/// - [`EffectDsl::compiler`](EffectDsl::compiler) for creating a new compiler
pub struct DslCompiler<'ctx> {
    dsl: &'ctx EffectDsl,
    environment: DslEnv,
}

impl DslCompiler<'_> {
    /// Binds a value to a name in the compiler's environment.
    ///
    /// The bound value can then be referenced by name in DSL expressions.
    ///
    /// # Arguments
    ///
    /// * `name` - The name to bind the value to
    /// * `value` - The value to bind
    ///
    /// # Returns
    ///
    /// Returns self for method chaining.
    pub fn bind<K, T>(mut self, name: K, value: T) -> Self
    where
        K: Into<CompactString>,
        T: 'static,
    {
        self.environment = self.environment.bind(name, value);
        self
    }

    /// Compiles a DSL expression string into a concrete effect.
    ///
    /// # Arguments
    ///
    /// * `input` - The DSL expression to compile
    ///
    /// # Returns
    ///
    /// Returns either:
    /// - `Ok(Effect)` if compilation succeeds
    /// - `Err(DslError)` if parsing or compilation fails
    ///
    /// # Examples
    ///
    /// ```
    /// use tachyonfx::dsl::EffectDsl;
    ///
    /// let effect = EffectDsl::new().compiler()
    ///     .compile("fx::dissolve(500)")
    ///     .unwrap();
    /// ```
    #[allow(clippy::result_large_err)]
    pub fn compile(self, input: &str) -> Result<Effect, DslParseError> {
        tokenize(input)
            .map(sanitize_tokens)
            .and_then(verify_tokens)
            .and_then(parse_ast)
            .and_then(|ast| self.dsl.compile(&self.environment, ast))
            .map_err(|e| DslParseError::new(input, e))
    }
}

fn register_default_compilers(effect_dsl: EffectDsl) -> EffectDsl {
    effect_dsl
        .register("term256_colors", |_args| {
            #[allow(deprecated)]
            fx::term256_colors().into()
        })
        .register("coalesce", compilers::coalesce)
        .register("coalesce_from", compilers::coalesce_from)
        .register("consume_tick", |_args| consume_tick().into())
        .register("delay", compilers::delay)
        .register("dissolve", |args| dissolve(args.effect_timer()?).into())
        .register("dissolve_to", compilers::dissolve_to)
        .register("evolve", compilers::evolve)
        .register("evolve_into", compilers::evolve_into)
        .register("evolve_from", compilers::evolve_from)
        .register("expand", compilers::expand)
        .register("explode", compilers::explode)
        .register("fade_from", compilers::fade_from)
        .register("fade_from_fg", compilers::fade_from_fg)
        .register("fade_to", compilers::fade_to)
        .register("fade_to_fg", compilers::fade_to_fg)
        .register("darken", compilers::darken)
        .register("darken_fg", compilers::darken_fg)
        .register("freeze_at", compilers::freeze_at)
        .register("hsl_shift", compilers::hsl_shift)
        .register("hsl_shift_fg", compilers::hsl_shift_fg)
        .register("lighten", compilers::lighten)
        .register("lighten_fg", compilers::lighten_fg)
        .register("never_complete", |args| {
            never_complete(args.effect()?).into()
        })
        .register("paint", compilers::paint)
        .register("paint_bg", compilers::paint_bg)
        .register("paint_fg", compilers::paint_fg)
        .register("saturate", compilers::saturate)
        .register("saturate_fg", compilers::saturate_fg)
        .register("ping_pong", |args| ping_pong(args.effect()?).into())
        .register("prolong_end", compilers::prolong_end)
        .register("prolong_start", compilers::prolong_start)
        .register("remap_alpha", compilers::remap_alpha)
        .register("repeat", compilers::repeat)
        .register("run_once", |args| run_once(args.effect()?).into())
        .register("sleep", compilers::sleep)
        .register("repeating", |args| repeating(args.effect()?).into())
        .register("slide_in", compilers::slide_in)
        .register("slide_out", compilers::slide_out)
        .register("stretch", compilers::stretch)
        .register("sweep_in", compilers::sweep_in)
        .register("sweep_out", compilers::sweep_out)
        .register("with_duration", compilers::with_duration)
        .register("timed_never_complete", compilers::timed_never_complete)
        .register("translate", compilers::translate)
}

impl EffectCompiler {
    fn new(
        name: &'static str,
        compile: impl Fn(&mut Arguments) -> Result<Effect, DslError> + 'static,
    ) -> Self {
        Self { effect_name: name, compile: Box::new(compile) }
    }
}

impl From<Effect> for Result<Effect, DslError> {
    fn from(effect: Effect) -> Self {
        Ok(effect)
    }
}

mod compilers {
    use crate::{
        dsl::{dsl::Arguments, expressions::Expr, DslError},
        fx, Effect,
    };

    pub(super) fn coalesce(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::coalesce(args.effect_timer()?).into()
    }

    pub(super) fn coalesce_from(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::coalesce_from(args.style()?, args.effect_timer()?).into()
    }

    pub(super) fn evolve(args: &mut Arguments) -> Result<Effect, DslError> {
        if let Some(Expr::Tuple(_, _)) = args.peek() {
            let symbols = args.tuple_2(Arguments::evolve_symbol_set, Arguments::style)?;
            let timer = args.effect_timer()?;
            fx::evolve(symbols, timer).into()
        } else {
            let symbols = args.evolve_symbol_set()?;
            let timer = args.effect_timer()?;
            fx::evolve(symbols, timer).into()
        }
    }

    pub(super) fn evolve_into(args: &mut Arguments) -> Result<Effect, DslError> {
        if let Some(Expr::Tuple(_, _)) = args.peek() {
            let symbols = args.tuple_2(Arguments::evolve_symbol_set, Arguments::style)?;
            let timer = args.effect_timer()?;
            fx::evolve_into(symbols, timer).into()
        } else {
            let symbols = args.evolve_symbol_set()?;
            let timer = args.effect_timer()?;
            fx::evolve_into(symbols, timer).into()
        }
    }

    pub(super) fn evolve_from(args: &mut Arguments) -> Result<Effect, DslError> {
        if let Some(Expr::Tuple(_, _)) = args.peek() {
            let symbols = args.tuple_2(Arguments::evolve_symbol_set, Arguments::style)?;
            let timer = args.effect_timer()?;
            fx::evolve_from(symbols, timer).into()
        } else {
            let symbols = args.evolve_symbol_set()?;
            let timer = args.effect_timer()?;
            fx::evolve_from(symbols, timer).into()
        }
    }

    pub(super) fn expand(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::expand(
            args.expand_direction()?,
            args.style()?,
            args.effect_timer()?,
        )
        .into()
    }

    pub(super) fn explode(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::explode(
            args.read_into_f32()?,
            args.read_into_f32()?,
            args.effect_timer()?,
        )
        .into()
    }

    pub(super) fn fade_to_fg(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::fade_to_fg(args.color()?, args.effect_timer()?).into()
    }

    pub(super) fn fade_from_fg(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::fade_from_fg(args.color()?, args.effect_timer()?).into()
    }

    pub(super) fn fade_to(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::fade_to(args.color()?, args.color()?, args.effect_timer()?).into()
    }

    pub(super) fn freeze_at(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::freeze_at(args.read_into_f32()?, args.read_bool()?, args.effect()?).into()
    }

    pub(super) fn dissolve_to(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::dissolve_to(args.style()?, args.effect_timer()?).into()
    }

    pub(super) fn fade_from(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::fade_from(args.color()?, args.color()?, args.effect_timer()?).into()
    }

    pub(super) fn hsl_shift(args: &mut Arguments) -> Result<Effect, DslError> {
        let span = args.span(); // fixme: improve array() to include span
        let into_array = |data: Vec<f32>| -> Result<[f32; 3], DslError> {
            match data.len() {
                3 => Ok([data[0], data[1], data[2]]),
                l => Err(DslError::ArrayLengthMismatch { expected: 3, actual: l, location: span }),
            }
        };

        let fg: Option<[f32; 3]> =
            args.option(|args| into_array(args.array(Arguments::read_into_f32)?))?;
        let bg: Option<[f32; 3]> =
            args.option(|args| into_array(args.array(Arguments::read_into_f32)?))?;

        fx::hsl_shift(fg, bg, args.effect_timer()?).into()
    }

    pub(super) fn hsl_shift_fg(args: &mut Arguments) -> Result<Effect, DslError> {
        let span = args.span(); // fixme: improve array() to include span
        let into_array = |data: Vec<f32>| -> Result<[f32; 3], DslError> {
            match data.len() {
                3 => Ok([data[0], data[1], data[2]]),
                l => Err(DslError::ArrayLengthMismatch { expected: 3, actual: l, location: span }),
            }
        };

        fx::hsl_shift_fg(
            into_array(args.array(Arguments::read_into_f32)?)?,
            args.effect_timer()?,
        )
        .into()
    }

    pub(super) fn sweep_out(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::sweep_out(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?,
        )
        .into()
    }

    pub(super) fn sleep(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::sleep(args.effect_timer()?).into()
    }

    pub(super) fn delay(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::delay(args.effect_timer()?, args.effect()?).into()
    }

    pub(super) fn paint(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::paint(args.color()?, args.color()?, args.effect_timer()?).into()
    }

    pub(super) fn paint_fg(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::paint_fg(args.color()?, args.effect_timer()?).into()
    }

    pub(super) fn paint_bg(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::paint_bg(args.color()?, args.effect_timer()?).into()
    }

    pub(super) fn lighten(args: &mut Arguments) -> Result<Effect, DslError> {
        let fg = args.option(Arguments::read_into_f32)?;
        let bg = args.option(Arguments::read_into_f32)?;
        fx::lighten(fg, bg, args.effect_timer()?).into()
    }

    pub(super) fn lighten_fg(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::lighten_fg(args.read_into_f32()?, args.effect_timer()?).into()
    }

    pub(super) fn darken(args: &mut Arguments) -> Result<Effect, DslError> {
        let fg = args.option(Arguments::read_into_f32)?;
        let bg = args.option(Arguments::read_into_f32)?;
        fx::darken(fg, bg, args.effect_timer()?).into()
    }

    pub(super) fn darken_fg(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::darken_fg(args.read_into_f32()?, args.effect_timer()?).into()
    }

    pub(super) fn saturate(args: &mut Arguments) -> Result<Effect, DslError> {
        let fg = args.option(Arguments::read_into_f32)?;
        let bg = args.option(Arguments::read_into_f32)?;
        fx::saturate(fg, bg, args.effect_timer()?).into()
    }

    pub(super) fn saturate_fg(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::saturate_fg(args.read_into_f32()?, args.effect_timer()?).into()
    }

    pub(super) fn prolong_start(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::prolong_start(args.effect_timer()?, args.effect()?).into()
    }

    pub(super) fn prolong_end(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::prolong_end(args.effect_timer()?, args.effect()?).into()
    }

    pub(super) fn remap_alpha(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::remap_alpha(args.read_into_f32()?, args.read_into_f32()?, args.effect()?).into()
    }

    pub(super) fn repeat(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::repeat(args.effect()?, args.repeat_mode()?).into()
    }

    pub(super) fn sweep_in(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::sweep_in(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?,
        )
        .into()
    }

    pub(super) fn slide_in(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::slide_in(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?,
        )
        .into()
    }

    pub(super) fn slide_out(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::slide_out(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?,
        )
        .into()
    }

    pub(super) fn with_duration(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::with_duration(args.duration()?, args.effect()?).into()
    }

    pub(super) fn stretch(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::stretch(args.motion()?, args.style()?, args.effect_timer()?).into()
    }

    pub(super) fn timed_never_complete(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::timed_never_complete(args.duration()?, args.effect()?).into()
    }

    pub(super) fn translate(args: &mut Arguments) -> Result<Effect, DslError> {
        let fx = args.effect()?;
        let offset = args.offset()?;

        fx::translate(fx, offset, args.effect_timer()?).into()
    }
}

impl fmt::Debug for EffectCompiler {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("compiler")
            .field("name", &self.effect_name)
            .finish()
    }
}

#[cfg(test)]
#[allow(clippy::std_instead_of_alloc)]
mod tests {
    use std::collections::VecDeque;

    use compact_str::ToCompactString;
    use ratatui_core::{
        layout::{Constraint::Percentage, Flex, Layout, Margin, Rect},
        style::{Color, Modifier, Style},
    };
    use regex::Regex;
    use Interpolation::Linear;

    use crate::{
        dsl::{
            arguments::Arguments,
            dsl::{compilers, EffectDsl},
            environment::DslEnv,
            expressions::{Expr, ExprSpan, Value},
            DslError,
        },
        fx,
        fx::RepeatMode,
        CellFilter, Duration, Effect, EffectTimer, Interpolation,
        Interpolation::{CircOut, QuadOut},
        Motion,
    };

    fn assert_effect_roundtrip_eq(effect: Effect) {
        let expr = effect
            .to_dsl()
            .expect("dsl expression from effect")
            .to_string();

        let dsl = EffectDsl::new();
        let actual = dsl
            .compiler()
            .compile(&expr)
            .expect("effect from compiled dsl expression");

        let regex = Regex::new("SimpleRng \\{ state: \\d+ }").unwrap();
        let sanitized = |t| {
            let debugged = format!("{t:?}");
            regex
                .replace_all(&debugged, "SimpleRng")
                .to_string()
        };

        assert_eq!(
            format!("{:?}", sanitized(actual)),
            format!("{:?}", sanitized(effect)),
        );
    }

    #[test]
    #[allow(deprecated)]
    fn test_compiler_dsl_roundtrips() {
        use ratatui_core::layout::Offset;
        let color = Color::from_u32(0);

        [
            fx::coalesce((1000, Linear)),
            fx::coalesce_from(Style::default(), (1000, Linear)),
            fx::consume_tick(),
            fx::delay((1000, Linear), fx::dissolve((1000, Linear))),
            fx::dissolve((1000, Linear)),
            fx::dissolve_to(Style::default(), (1000, Linear)),
            fx::expand(
                fx::ExpandDirection::Horizontal,
                Style::new().bg(Color::Cyan),
                (1000, Linear),
            ),
            fx::darken(Some(0.5), Some(0.3), (1000, Linear)),
            fx::darken(Some(0.5), None, (1000, Linear)),
            fx::darken(None, Some(0.3), (1000, Linear)),
            fx::darken_fg(0.5, (1000, Linear)),
            fx::fade_from(color, color, (1000, Linear)),
            fx::fade_from_fg(color, (1000, Linear)),
            fx::fade_to(color, color, (1000, Linear)),
            fx::fade_to_fg(color, (1000, Linear)),
            fx::freeze_at(0.8, true, fx::dissolve((1000, Linear))),
            fx::freeze_at(0.8, false, fx::dissolve((1000, Linear))),
            fx::hsl_shift(Some([1.0, 2.0, 3.0]), Some([1.0, 2.0, 3.0]), (1000, Linear)),
            fx::hsl_shift_fg([1.0, 2.0, 3.0], (1000, Linear)),
            fx::lighten(Some(0.5), Some(0.3), (1000, Linear)),
            fx::lighten(Some(0.5), None, (1000, Linear)),
            fx::lighten(None, Some(0.3), (1000, Linear)),
            fx::lighten_fg(0.5, (1000, Linear)),
            fx::never_complete(fx::dissolve((1000, Linear))),
            fx::paint(color, color, (1000, Linear)),
            fx::paint_fg(color, (1000, Linear)),
            fx::paint_bg(color, (1000, Linear)),
            fx::saturate(Some(0.5), Some(0.3), (1000, Linear)),
            fx::saturate(Some(0.5), None, (1000, Linear)),
            fx::saturate(None, Some(0.3), (1000, Linear)),
            fx::saturate_fg(0.5, (1000, Linear)),
            fx::ping_pong(fx::dissolve((1000, Linear))),
            fx::prolong_end((1000, Linear), fx::dissolve((1000, Linear))),
            fx::prolong_start((1000, Linear), fx::dissolve((1000, Linear))),
            fx::repeat(fx::dissolve((1000, Linear)), RepeatMode::Times(3)),
            fx::remap_alpha(0.3, 0.6, fx::dissolve((1000, Linear))),
            fx::repeating(fx::dissolve((1000, Linear))),
            fx::run_once(fx::dissolve((1000, Linear))),
            fx::sleep((1000, Linear)),
            fx::slide_in(Motion::LeftToRight, 10, 5, color, (1000, Linear)),
            fx::slide_out(Motion::UpToDown, 10, 5, color, (1000, Linear)),
            fx::stretch(
                Motion::LeftToRight,
                Style::new().bg(Color::Cyan).bg(Color::Black),
                (1000, Linear),
            ),
            fx::sweep_in(Motion::LeftToRight, 10, 5, color, (1000, Linear)),
            fx::sweep_out(Motion::UpToDown, 10, 5, color, (1000, Linear)),
            fx::term256_colors(),
            fx::timed_never_complete(Duration::from_millis(1000), fx::dissolve((1000, Linear))),
            fx::translate(
                fx::fade_to_fg(color, (500, Linear)),
                Offset { x: 15, y: -10 },
                (1500, Linear),
            ),
            fx::with_duration(Duration::from_millis(1000), fx::dissolve((1000, Linear))),
        ]
        .into_iter()
        .for_each(assert_effect_roundtrip_eq);
    }

    #[test]
    fn happy_path_no_bound_vars() {
        let input = r#"fx::sweep_in(
            Motion::LeftToRight,
            10,
            0,
            Color::from_u32(0x1d2021),
            (1000, QuadOut)
        )"#;

        let expected = fx::sweep_in(
            Motion::LeftToRight,
            10,
            0,
            Color::from_u32(0x1d2021),
            (Duration::from_millis(1000), QuadOut),
        );

        let effect = EffectDsl::new()
            .compiler()
            .compile(input)
            .expect("effect to be compiled");

        assert_eq!(format!("{effect:?}"), format!("{expected:?}"));
    }

    #[test]
    fn happy_path_with_bound_vars() {
        let expected = fx::sweep_in(
            Motion::LeftToRight,
            10,
            0,
            Color::from_u32(0x1d2021),
            EffectTimer::from_ms(1000, QuadOut),
        );

        let input = r#"fx::sweep_in(motion, 10, 0, c, (1000, QuadOut))"#;

        let dsl = EffectDsl::new();
        let effect = dsl
            .compiler()
            .bind("motion", Motion::LeftToRight)
            .bind("c", Color::from_u32(0x1d2021))
            .compile(input)
            .expect("effect to be compiled");

        assert_eq!(effect.name(), "sweep_in");
        assert_eq!(format!("{effect:?}"), format!("{expected:?}"));
    }

    #[test]
    fn happy_path_method_chaining() {
        let expected = fx::sweep_in(
            Motion::LeftToRight,
            10,
            0,
            Color::from_u32(0x1d2021),
            EffectTimer::from_ms(1000, QuadOut),
        )
        .with_filter(CellFilter::Not(Box::new(CellFilter::Layout(
            Layout::horizontal([Percentage(40), Percentage(40)])
                .spacing(1)
                .vertical_margin(1)
                .flex(Flex::SpaceBetween)
                .horizontal_margin(2),
            1,
        ))))
        .with_area(Rect::new(0, 0, 10, 10));

        let input = r#"fx::sweep_in(
            Motion::LeftToRight,
            10,
            0,
            Color::from_u32(0x1d2021),
            EffectTimer::from_ms(1000, QuadOut)
        ).with_filter(
            CellFilter::Not(Box::new(CellFilter::Layout(
                Layout::horizontal([Percentage(40), Percentage(40)])
                    .spacing(1)
                    .vertical_margin(1)
                    .flex(Flex::SpaceBetween)
                    .horizontal_margin(2),
                1)
            ))
        ).with_area(Rect::new(0, 0, 10, 10))"#;

        let effect = EffectDsl::new()
            .compiler()
            .compile(input)
            .expect("effect to be compiled");

        assert_eq!(effect.name(), "sweep_in");
        assert_eq!(format!("{effect:#?}"), format!("{expected:#?}"));
    }

    #[test]
    fn test_pattern_method_chaining_with_dissolve() {
        use crate::pattern::RadialPattern;

        let expected = fx::dissolve(EffectTimer::from_ms(1000, Linear))
            .with_pattern(RadialPattern::center().with_transition_width(3.5));

        let input = r#"fx::dissolve(1000)
            .with_pattern(RadialPattern::center().with_transition_width(3.5))"#;

        let effect = EffectDsl::new()
            .compiler()
            .compile(input)
            .expect("effect to be compiled");

        // Handle random number generator state differences like other tests
        let regex = Regex::new("SimpleRng \\{\\s*state: \\d+,?\\s*\\}").unwrap();
        let sanitized = |t| {
            let debugged = format!("{t:#?}");
            regex
                .replace_all(&debugged, "SimpleRng")
                .to_string()
        };

        assert_eq!(effect.name(), "dissolve");
        assert_eq!(
            format!("{:?}", sanitized(expected)),
            format!("{:?}", sanitized(effect)),
        );
    }

    #[test]
    fn happy_path_with_let_binding() {
        let motion = Motion::LeftToRight;
        let c = Color::from_u32(0x1d2021);
        let expected = fx::sweep_in(motion, 10, 0, c, EffectTimer::from_ms(1000, QuadOut));

        let input = r#"
            let motion = Motion::LeftToRight;
            let c = Color::from_u32(0x1d2021);

            fx::sweep_in(motion, 10, 0, c, (1000, QuadOut))
        "#;

        let dsl = EffectDsl::new();
        let effect = dsl
            .compiler()
            .compile(input)
            .expect("effect to be compiled");

        assert_eq!(effect.name(), "sweep_in");
        assert_eq!(format!("{effect:?}"), format!("{expected:?}"));
    }

    #[test]
    fn test_let_bindings_with_style_chaining() {
        let expected = fx::dissolve_to(
            Style::default()
                .fg(Color::Red)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            EffectTimer::from_ms(500, CircOut),
        );

        let input = r#"
            let style = Style::new()
                .fg(Color::Red)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD);
            let timer = (500, CircOut);

            fx::dissolve_to(style, timer)
        "#;

        let effect = EffectDsl::new()
            .compiler()
            .compile(input)
            .expect("effect to be compiled");

        let without_rng_state = |e: Effect| -> String {
            let regex = Regex::new("SimpleRng \\{ state: \\d+ }").unwrap();
            let s = format!("{e:?}");
            regex.replace_all(&s, "SimpleRng").to_string()
        };

        assert_eq!("dissolve_to", effect.name());
        assert_eq!(without_rng_state(expected), without_rng_state(effect));
    }

    #[test]
    fn test_let_bindings_with_effect_chaining() {
        let filter =
            CellFilter::AllOf(vec![CellFilter::Text, CellFilter::Outer(Margin::new(1, 1))]);
        let color = Color::from_u32(0xffaabb);
        let expected =
            fx::fade_to_fg(color, EffectTimer::from_ms(1000, Linear)).with_filter(filter);

        let input = r#"
            let color = Color::from_u32(0xffaabb);
            let filter = AllOf(vec![Text, Outer(Margin::new(1, 1))]);

            fx::fade_to_fg(color, 1000)
                .with_filter(filter)
        "#;

        let effect = EffectDsl::new()
            .compiler()
            .compile(input)
            .map_err(|e| {
                println!("{e}");
                e
            })
            .expect("effect to be compiled");

        assert_eq!("fade_to", effect.name());
        assert_eq!(format!("{expected:?}"), format!("{effect:?}"));
    }

    #[test]
    fn test_effect_with_pattern_chaining() {
        use crate::pattern::CheckerboardPattern;

        let pattern = CheckerboardPattern::with_cell_size(2).with_transition_width(1.5);
        let expected = fx::fade_to_fg(Color::Green, EffectTimer::from_ms(800, QuadOut))
            .with_pattern(pattern)
            .with_filter(CellFilter::Text);

        let input = r#"
            fx::fade_to_fg(Color::Green, (800, QuadOut))
                .with_pattern(CheckerboardPattern::with_cell_size(2).with_transition_width(1.5))
                .with_filter(CellFilter::Text)
        "#;

        let effect = EffectDsl::new()
            .compiler()
            .compile(input)
            .expect("effect to be compiled");

        assert_eq!("fade_to", effect.name());
        assert_eq!(format!("{expected:#?}"), format!("{effect:#?}"));
    }

    #[test]
    fn test_let_bindings_with_layout_chaining() {
        let expected = {
            let layout = Layout::horizontal([Percentage(50), Percentage(50)])
                .spacing(1)
                .horizontal_margin(2);

            fx::fade_to_fg(Color::Red, EffectTimer::from_ms(500, QuadOut))
                .with_filter(CellFilter::Layout(layout, 1))
        };

        let input = r#"
            let layout = Layout::horizontal([Percentage(50), Percentage(50)])
                .spacing(1)
                .horizontal_margin(2);

            let filter = CellFilter::Layout(layout, 1);
            let color = Color::Red;

            fx::fade_to_fg(color.clone(), (500, QuadOut))
                .with_filter(filter)
        "#;

        let effect = EffectDsl::new()
            .compiler()
            .compile(input)
            .expect("effect to be compiled");

        assert_eq!(effect.name(), "fade_to");
        assert_eq!(format!("{effect:?}"), format!("{expected:?}"));
    }

    #[test]
    fn test_let_bindings_with_compound_effects() {
        let expected = {
            let base_effect = fx::fade_to_fg(Color::Red, 500);
            fx::sequence(&[
                base_effect.clone(),
                base_effect.clone().reversed(),
                base_effect
                    .reversed()
                    .with_filter(CellFilter::Not(Box::new(CellFilter::Text))),
            ])
        };

        let effect = EffectDsl::new()
            .compiler()
            .bind("base", fx::fade_to_fg(Color::Red, 500))
            .compile(
                r#"
                let reversed = base.reversed();
                let filtered = reversed
                    .with_filter(Not(Box::new(Text)));

                let effect = fx::sequence(&[base.clone(), reversed, filtered]);
                effect
            "#,
            )
            .expect("effect to be compiled");

        assert_eq!(effect.name(), "sequence");
        assert_eq!(format!("{effect:?}"), format!("{expected:?}"));
    }

    #[test]
    fn test_let_bindings_with_nested_effects() {
        let margin = Margin::new(1, 1);
        let expected = fx::parallel(&[
            fx::fade_from_fg(Color::Blue, (500, CircOut)).with_filter(CellFilter::Inner(margin)),
            fx::fade_to_fg(Color::Red, (500, CircOut)).with_filter(CellFilter::Outer(margin)),
        ]);

        let effect = EffectDsl::new()
            .compiler()
            .compile(
                r#"
                let margin = Margin::new(1, 1);
                let inner_effect = fx::fade_from_fg(Color::Blue, (500, CircOut))
                    .with_filter(CellFilter::Inner(margin));
                let outer_effect = fx::fade_to_fg(Color::Red, (500, CircOut))
                    .with_filter(CellFilter::Outer(margin));

                fx::parallel(&[inner_effect, outer_effect])
            "#,
            )
            .expect("effect to be compiled");

        assert_eq!(effect.name(), "parallel");
        assert_eq!(format!("{effect:?}"), format!("{expected:?}"));
    }

    #[test]
    fn error_unknown_effect() {
        let input = r#"fx::nonexistent()"#;
        let ctx = EffectDsl::new();
        let err = ctx.compiler().compile(input).unwrap_err();
        println!("{err}");
        assert!(matches!(err.source, DslError::UnknownEffect { .. }));
    }

    #[test]
    fn error_invalid_argument() {
        let input = r#"fx::sweep_in("wrong", 10, 0, Color::from_u32(0x1d2021), 1000)"#;
        let ctx = EffectDsl::new();
        let err = ctx.compiler().compile(input).unwrap_err();
        println!("{err}");
        assert!(
            matches!(err.source, DslError::WrongArgumentType {
                location: _,
                expected: "motion",
                actual: _
            }),
            "{err:?}",
        );
    }

    #[test]
    fn too_many_arguments() {
        let input = r#"fx::sweep_in(
                Motion::LeftToRight,
                10,
                0,
                Color::from_u32(0x1d2021),
                (1000, QuadOut),
                "extra"
            )"#;

        let ctx = EffectDsl::new();
        let err = ctx.compiler().compile(input).unwrap_err();
        assert!(
            matches!(err.source, DslError::InvalidArgumentLength { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn test_compiler_missing_arguments() {
        let dsl = EffectDsl::new();
        let exprs = vec![];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env, ExprSpan::default());
        assert!(compilers::fade_to_fg(&mut args).is_err());
    }

    #[test]
    fn test_compiler_wrong_argument_type() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(
                Value::String("wrong".to_compact_string()),
                ExprSpan::new(0, 0),
            ),
            Expr::Literal(Value::OptionNone, ExprSpan::new(0, 0)),
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env, ExprSpan::default());
        assert!(compilers::fade_to_fg(&mut args).is_err());
    }

    #[test]
    fn test_missing_brackets() {
        let dsl = EffectDsl::new();

        for expr in ["(x", "x)", "[x", "x]", "{x", "x}", "{[x}]", "[(x])", "{(x})"] {
            let err = dsl
                .compiler()
                .compile(expr)
                .expect_err("should fail")
                .source;

            assert!(
                matches!(err, DslError::BracketMismatch { .. }),
                "expr: {expr} - {err:?}"
            );
        }
    }

    #[test]
    fn test_evolve_effects() {
        use crate::{
            fx, fx::EvolveSymbolSet, pattern::RadialPattern, EffectTimer, Interpolation::*,
        };

        // Test fx::evolve with EvolveSymbolSet, style tuple, and pattern chaining
        let pattern = RadialPattern::center().with_transition_width(2.5);
        let expected = fx::evolve(
            (
                EvolveSymbolSet::Circles,
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
            EffectTimer::from_ms(800, QuadOut),
        )
        .with_pattern(pattern);

        let input = r#"
            let symbols = EvolveSymbolSet::Circles;
            let style = Style::new()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD);

            fx::evolve((symbols, style), (800, QuadOut))
                .with_pattern(RadialPattern::center().with_transition_width(2.5))
        "#;

        let effect = EffectDsl::new()
            .compiler()
            .compile(input)
            .expect("effect to be compiled");

        let without_rng_state = |e: Effect| -> String {
            let regex = Regex::new("SimpleRng \\{ state: \\d+ }").unwrap();
            let s = format!("{e:?}");
            regex.replace_all(&s, "SimpleRng").to_string()
        };

        assert_eq!("evolve", effect.name());
        assert_eq!(without_rng_state(expected), without_rng_state(effect));
    }

    #[test]
    fn test_evolve_from_and_into_effects() {
        use crate::{
            fx, fx::EvolveSymbolSet, pattern::DiagonalPattern, EffectTimer, Interpolation::*,
        };

        // Test fx::evolve_from and fx::evolve_into with patterns
        let diagonal_pattern =
            DiagonalPattern::top_left_to_bottom_right().with_transition_width(1.8);
        let _expected_from = fx::evolve_from(
            EvolveSymbolSet::BlocksHorizontal,
            EffectTimer::from_ms(1000, BounceIn),
        )
        .with_pattern(diagonal_pattern);
        let _expected_into = fx::evolve_into(
            (EvolveSymbolSet::Quadrants, Style::default().bg(Color::Blue)),
            EffectTimer::from_ms(600, CubicOut),
        );

        let input = r#"
            let blocks_symbols = EvolveSymbolSet::BlocksHorizontal;
            let quadrants_symbols = EvolveSymbolSet::Quadrants;
            let style = Style::new().bg(Color::Blue);

            let from_effect = fx::evolve_from(blocks_symbols, (1000, BounceIn))
                .with_pattern(DiagonalPattern::top_left_to_bottom_right().with_transition_width(1.8));
            let into_effect = fx::evolve_into((quadrants_symbols, style), (600, CubicOut));

            fx::sequence(&[from_effect, into_effect])
        "#;

        let effect = EffectDsl::new()
            .compiler()
            .compile(input)
            .expect("effect to be compiled");

        let without_rng_state = |e: Effect| -> String {
            let regex = Regex::new("SimpleRng \\{ state: \\d+ }").unwrap();
            let s = format!("{e:?}");
            regex.replace_all(&s, "SimpleRng").to_string()
        };

        assert_eq!("sequence", effect.name());
        // Verify the sequence contains our evolve effects with patterns
        let effect_debug = without_rng_state(effect);
        assert!(effect_debug.contains("mode: From"));
        assert!(effect_debug.contains("mode: Into"));
        assert!(effect_debug.contains("BlocksHorizontal"));
        assert!(effect_debug.contains("Quadrants"));
        assert!(effect_debug.contains("DiagonalPattern"));
    }

    #[test]
    fn test_missing_semicolon() {
        let dsl = EffectDsl::new();

        let expr = "let fx::dissolve(500) fx::dissolve(500)";
        let err = dsl
            .compiler()
            .compile(expr)
            .expect_err("should fail")
            .source;

        assert!(
            matches!(err, DslError::MissingSemicolon { .. }),
            "expr: {expr} - {err:?}"
        );
    }

    #[test]
    fn test_missing_commma() {
        let dsl = EffectDsl::new();

        let expr = "(1000 QuadOut)";
        let err = dsl
            .compiler()
            .compile(expr)
            .expect_err("should fail")
            .source;

        assert!(
            matches!(err, DslError::MissingComma { .. }),
            "expr: {expr} - {err:?}"
        );
    }

    #[test]
    fn test_pattern_variable_binding() {
        use crate::{pattern::RadialPattern, ColorSpace};

        let expected = fx::fade_to_fg(
            Color::from_u32(0x32302F),
            EffectTimer::from_ms(1500, QuadOut),
        )
        .with_color_space(ColorSpace::Rgb)
        .with_pattern(RadialPattern::center().with_transition_width(15.0));

        let input = r#"
            let p = RadialPattern::center()
                .with_transition_width(15.0);

            fx::fade_to_fg(Color::from_u32(0x32302F), (1500, QuadOut))
                .with_color_space(ColorSpace::Rgb)
                .with_pattern(p)
        "#;

        let effect = EffectDsl::new()
            .compiler()
            .compile(input)
            .expect("effect to be compiled");

        // Handle random number generator state differences like other tests
        let regex = Regex::new("SimpleRng \\{\\s*state: \\d+,?\\s*\\}").unwrap();
        let sanitized = |t| {
            let debugged = format!("{t:#?}");
            regex
                .replace_all(&debugged, "SimpleRng")
                .to_string()
        };

        assert_eq!(effect.name(), "fade_to");
        assert_eq!(
            format!("{:?}", sanitized(expected)),
            format!("{:?}", sanitized(effect)),
        );
    }

    #[test]
    fn test_translate_effect() {
        use ratatui_core::layout::Offset;
        let expected = fx::translate(
            fx::dissolve((500, Linear)),
            Offset { x: 10, y: -5 },
            EffectTimer::from_ms(1000, Linear),
        );

        let input = r#"fx::translate(fx::dissolve((500, Linear)), Offset { x: 10, y: -5 }, (1000, Linear))"#;
        let effect = EffectDsl::new()
            .compiler()
            .compile(input)
            .expect("effect to be compiled");

        // Handle random number generator state differences like other tests
        let regex = Regex::new("SimpleRng \\{\\s*state: \\d+,?\\s*\\}").unwrap();
        let sanitized = |t| {
            let debugged = format!("{t:#?}");
            regex
                .replace_all(&debugged, "SimpleRng")
                .to_string()
        };
        assert_eq!(effect.name(), "translate_by");
        assert_eq!(
            format!("{:?}", sanitized(expected)),
            format!("{:?}", sanitized(effect)),
        );
    }
}
