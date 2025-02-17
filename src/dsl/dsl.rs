use crate::dsl::arguments::Arguments;
use crate::dsl::environment::DslEnv;
use crate::dsl::expressions::{Expr, FnCallInfo};
use crate::dsl::parsers::parse_expr;
use crate::dsl::DslError;
use crate::fx::{consume_tick, dissolve, never_complete, ping_pong, repeating};
use crate::{fx, Effect, Shader};
use std::fmt;
use std::fmt::Formatter;

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
///
/// let dsl = EffectDsl::new()
///     .register("custom_effect", |args| {
///         // Implement custom effect compilation
///         # todo!()
///     });
/// ```
#[derive(Debug, Default)]
pub struct EffectDsl {
    compilers: Vec<EffectCompiler>,
}

struct EffectCompiler {
    effect_name: &'static str,
    compile: Box<dyn Fn(&mut Arguments) -> Result<Effect, DslError>>,
}

impl EffectDsl {
    /// Creates a new `EffectDsl` instance with all standard effect compilers registered.
    pub fn new() -> Self {
        register_default_compilers(Self {
            compilers: Vec::new(),
        })
    }

    /// Registers a new effect compiler with the DSL.
    ///
    /// This method allows extending the DSL with custom effects. The compiler function
    /// receives parsed arguments and should return a concrete `Effect` instance or `DslError`
    /// if compilation fails.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the effect as it will appear in DSL expressions (e.g., "my_effect" for `fx::my_effect(...)`)
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
        compiler: impl Fn(&mut Arguments) -> Result<Effect, DslError> + 'static
    ) -> Self {
        let mut this = self;
        this.compilers.push(EffectCompiler::new(name, compiler));
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
    /// use ratatui::style::Color;
    ///
    /// let dsl = EffectDsl::new();
    /// let compiler = dsl.compiler()
    ///     .bind("bg_color", Color::Blue);
    ///
    /// // Use bound variables in expressions
    /// let effect = compiler.compile(r#"
    ///     fx::fade_to(bg_color, (1000, Linear))
    /// "#);
    /// ```
    pub fn compiler(&self) -> DslCompiler {
        DslCompiler {
            dsl: self,
            environment: DslEnv::new(),
        }
    }

    pub(super) fn compile(
        &self,
        env: &DslEnv,
        input: Expr
    ) -> Result<Effect, DslError> {
        match input {
            Expr::Fx { name, arguments, self_fns } => self.compilers
                .iter()
                .find(|d| d.effect_name == name.as_str())
                .ok_or(DslError::UnknownEffect { name })
                .and_then(|d| {
                    let mut args = Arguments::new(arguments.into(), self, env);
                    let effect = self.apply_effect_fns((d.compile)(&mut args)?, self_fns, env);

                    match () {
                        _ if effect.is_err() => effect,
                        _ if !args.remaining_args().is_empty() => Err(DslError::InvalidArgumentLength {
                            expected: args.original_arg_count() - args.remaining_args().len(),
                            actual: args.original_arg_count(),
                        }),
                        _ => effect,
                    }
                }),
            Expr::Sequence { effects, self_fns } => {
                let mut args = Arguments::new(effects.into(), self, env);
                let effects = (0..args.remaining_arg_count())
                    .map(|_| args.effect())
                    .collect::<Result<Vec<Effect>, DslError>>()?;

                Ok(self.apply_effect_fns(fx::sequence(&effects), self_fns, env)?)
            },
            Expr::Parallel { effects, self_fns } => {
                let mut args = Arguments::new(effects.into(), self, env);
                let effects = (0..args.remaining_arg_count())
                    .map(|_| args.effect())
                    .collect::<Result<Vec<Effect>, DslError>>()?;

                Ok(self.apply_effect_fns(fx::parallel(&effects), self_fns, env)?)
            },
            _ => Err(DslError::InvalidExpression {
                expected: "effect",
                actual: input.type_name(),
            }),
        }
    }

    fn apply_effect_fns(
        &self,
        effect: Effect,
        fns: Vec<FnCallInfo>,
        env: &DslEnv
    ) -> Result<Effect, DslError> {
        let mut effect = effect;
        for self_fn in fns {
            match self_fn.name.as_str() {
                "with_area" => {
                    let area = Arguments::extract_with(
                        self_fn.args, self, env, Arguments::rect
                    )?;
                    effect = effect.with_area(area);
                },
                "with_filter" => {
                    let cell_filter = Arguments::extract_with(
                        self_fn.args, self, env, Arguments::cell_filter
                    )?;

                    effect = effect.with_filter(cell_filter);
                },
                "filter" => {
                    let cell_filter = Arguments::extract_with(
                        self_fn.args, self, env, Arguments::cell_filter
                    )?;
                    effect.filter(cell_filter);
                },
                _ => return Err(DslError::UnknownFunction { name: self_fn.name }),
            }
        }

        Ok(effect)
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
        K: Into<String>,
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
    pub fn compile(self, input: &str) -> Result<Effect, DslError> {
        parse_expr(input)
            .and_then(|expr| self.dsl.compile(&self.environment, expr))
    }
}

fn register_default_compilers(effect_dsl: EffectDsl) -> EffectDsl {
    effect_dsl
        .register("term256_colors", |_args| fx::term256_colors().into())
        .register("coalesce",       compilers::coalesce)
        .register("coalesce_from",  compilers::coalesce_from)
        .register("consume_tick",   |_args| consume_tick().into())
        .register("delay",          compilers::delay)
        .register("dissolve",       |args| dissolve(args.effect_timer()?).into())
        .register("dissolve_to",    compilers::dissolve_to)
        .register("fade_from",      compilers::fade_from)
        .register("fade_from_fg",   compilers::fade_from_fg)
        .register("fade_to",        compilers::fade_to)
        .register("fade_to_fg",     compilers::fade_to_fg)
        .register("hsl_shift",      compilers::hsl_shift)
        .register("hsl_shift_fg",   compilers::hsl_shift_fg)
        .register("never_complete", |args| never_complete(args.effect()?).into())
        .register("ping_pong",      |args| ping_pong(args.effect()?).into())
        .register("prolong_end",    compilers::prolong_end)
        .register("prolong_start",  compilers::prolong_start)
        .register("repeat",         compilers::repeat)
        .register("sleep",          compilers::sleep)
        .register("repeating",      |args| repeating(args.effect()?).into())
        .register("slide_in",       compilers::slide_in)
        .register("slide_out",      compilers::slide_out)
        .register("sweep_in",       compilers::sweep_in)
        .register("sweep_out",      compilers::sweep_out)
        .register("with_duration",  compilers::with_duration)
        .register(
            "timed_never_complete",
            compilers::timed_never_complete
        )
}

impl EffectCompiler {
    fn new(
        name: &'static str,
        compile: impl Fn(&mut Arguments) -> Result<Effect, DslError> + 'static
    ) -> Self {
        Self {
            effect_name: name,
            compile: Box::new(compile),
        }
    }
}


impl From<Effect> for Result<Effect, DslError> {
    fn from(effect: Effect) -> Self {
        Ok(effect)
    }
}

mod compilers {
    use crate::dsl::dsl::Arguments;
    use crate::dsl::DslError;
    use crate::{fx, Effect};

    pub(super) fn coalesce(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::coalesce(args.effect_timer()?).into()
    }

    pub(super) fn coalesce_from(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::coalesce_from(
            args.style()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn fade_to_fg(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::fade_to_fg(
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn fade_from_fg(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::fade_from_fg(
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn fade_to(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::fade_to(
            args.color()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn dissolve_to(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::dissolve_to(
            args.style()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn fade_from(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::fade_from(
            args.color()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn hsl_shift(args: &mut Arguments) -> Result<Effect, DslError> {
        let into_array = |data: Vec<f32>| -> Result<[f32; 3], DslError> {
            match data.len() {
                3 => Ok([data[0], data[1], data[2]]),
                l => Err(DslError::ArrayLengthMismatch {
                    expected: 3,
                    actual: l
                }),
            }
        };

        let fg: Option<[f32; 3]> = args
            .option(|args| into_array(args.array(Arguments::read_into_f32)?))?;
        let bg: Option<[f32; 3]> = args
            .option(|args| into_array(args.array(Arguments::read_into_f32)?))?;

        fx::hsl_shift(fg, bg, args.effect_timer()?).into()
    }

    pub(super) fn hsl_shift_fg(args: &mut Arguments) -> Result<Effect, DslError> {
        let into_array = |data: Vec<f32>| -> Result<[f32; 3], DslError> {
            match data.len() {
                3 => Ok([data[0], data[1], data[2]]),
                l => Err(DslError::ArrayLengthMismatch {
                    expected: 3,
                    actual: l
                }),
            }
        };

        fx::hsl_shift_fg(
            into_array(args.array(Arguments::read_into_f32)?)?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn sweep_out(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::sweep_out(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn sleep(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::sleep(args.effect_timer()?).into()
    }

    pub(super) fn delay(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::delay(args.effect_timer()?, args.effect()?).into()
    }

    pub(super) fn prolong_start(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::prolong_start(args.effect_timer()?, args.effect()?).into()
    }

    pub(super) fn prolong_end(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::prolong_end(args.effect_timer()?, args.effect()?).into()
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
            args.effect_timer()?
        ).into()
    }

    pub(super) fn slide_in(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::slide_in(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn slide_out(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::slide_out(
            args.motion()?,
            args.read_u16()?,
            args.read_u16()?,
            args.color()?,
            args.effect_timer()?
        ).into()
    }

    pub(super) fn with_duration(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::with_duration(
            args.duration()?,
            args.effect()?
        ).into()
    }

    pub(super) fn timed_never_complete(args: &mut Arguments) -> Result<Effect, DslError> {
        fx::timed_never_complete(args.duration()?, args.effect()?).into()
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
mod tests {
    use crate::dsl::arguments::Arguments;
    use crate::dsl::dsl::{compilers, EffectDsl};
    use crate::dsl::environment::DslEnv;
    use crate::dsl::expressions::{Expr, Value};
    use crate::dsl::DslError;
    use crate::fx::RepeatMode;
    use crate::Interpolation::QuadOut;
    use crate::{fx, CellFilter, Duration, Effect, EffectTimer, Interpolation, Motion, Shader};
    use ratatui::style::{Color, Style};
    use regex::Regex;
    use std::collections::VecDeque;
    use ratatui::layout::{Layout, Margin, Rect};
    use ratatui::layout::Constraint::Percentage;
    use Interpolation::Linear;

    fn assert_effect_roundtrip_eq(
        effect: Effect,
    ) {
        let expr = effect
            .to_dsl()
            .expect("dsl expression from effect")
            .to_string();

        let dsl = EffectDsl::new();
        let actual = dsl.compiler()
            .compile(&expr)
            .expect("effect from compiled dsl expression");

        let regex = Regex::new("SimpleRng \\{ state: \\d+ }").unwrap();
        let sanitized = |t| {
            let debugged = format!("{:?}", t);
            regex.replace_all(&debugged, "SimpleRng").to_string()
        };

        assert_eq!(
            format!("{:?}", sanitized(actual)),
            format!("{:?}", sanitized(effect)),
        );
    }

    #[test]
    fn test_compiler_dsl_roundtrips() {
        let color = Color::from_u32(0);

        [
            fx::coalesce((1000, Linear)),
            fx::coalesce_from(Style::default(), (1000, Linear)),
            fx::consume_tick(),
            fx::delay((1000, Linear), fx::dissolve((1000, Linear))),
            fx::dissolve((1000, Linear)),
            fx::dissolve_to(Style::default(), (1000, Linear)),
            fx::fade_from(color, color, (1000, Linear)),
            fx::fade_from_fg(color, (1000, Linear)),
            fx::fade_to(color, color, (1000, Linear)),
            fx::fade_to_fg(color, (1000, Linear)),
            fx::hsl_shift(Some([1.0, 2.0, 3.0]), Some([1.0, 2.0, 3.0]), (1000, Linear)),
            fx::hsl_shift_fg([1.0, 2.0, 3.0], (1000, Linear)),
            fx::never_complete(fx::dissolve((1000, Linear))),
            fx::ping_pong(fx::dissolve((1000, Linear))),
            fx::prolong_end((1000, Linear), fx::dissolve((1000, Linear))),
            fx::prolong_start((1000, Linear), fx::dissolve((1000, Linear))),
            fx::repeat(fx::dissolve((1000, Linear)), RepeatMode::Forever),
            fx::repeating(fx::dissolve((1000, Linear))),
            fx::sleep((1000, Linear)),
            fx::slide_in(Motion::LeftToRight, 10, 5, color, (1000, Linear)),
            fx::slide_out(Motion::UpToDown, 10, 5, color, (1000, Linear)),
            fx::sweep_in(Motion::LeftToRight, 10, 5, color, (1000, Linear)),
            fx::sweep_out(Motion::UpToDown, 10, 5, color, (1000, Linear)),
            fx::term256_colors(),
            fx::timed_never_complete(Duration::from_millis(1000), fx::dissolve((1000, Linear))),
            fx::with_duration(Duration::from_millis(1000), fx::dissolve((1000, Linear))),
        ].into_iter()
            .for_each(assert_effect_roundtrip_eq);
    }

    #[test]
    fn happy_path_no_bound_vars() {
        let input = r#"fx::sweep_in(
            Motion::LeftToRight,
            10,
            0,
            Color::from_u32(0x1d2021),
            (Duration::from_millis(1000), QuadOut)
        )"#;

        let expected = fx::sweep_in(
            Motion::LeftToRight,
            10,
            0,
            Color::from_u32(0x1d2021),
            (Duration::from_millis(1000), QuadOut)
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
            EffectTimer::from_ms(1000, QuadOut)
        );

        let input = r#"fx::sweep_in(motion, 10, 0, c, (1000, QuadOut))"#;

        let dsl = EffectDsl::new();
        let effect = dsl.compiler()
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
            EffectTimer::from_ms(1000, QuadOut)
        ).with_filter(
            CellFilter::Not(Box::new(CellFilter::Layout(
                Layout::horizontal([Percentage(50), Percentage(50)])
                    .spacing(1)
                    .vertical_margin(1)
                    .horizontal_margin(2),
                1)
            ))
        ).with_area(Rect::new(0, 0, 10, 10));

        let input = r#"fx::sweep_in(
            Motion::LeftToRight,
            10,
            0,
            Color::from_u32(0x1d2021),
            EffectTimer::from_ms(1000, QuadOut)
        ).with_filter(
            CellFilter::Not(Box::new(CellFilter::Layout(
                Layout::horizontal([Percentage(50), Percentage(50)])
                    .spacing(1)
                    .vertical_margin(1)
                    .horizontal_margin(2),
                1)
            ))
        ).with_area(Rect::new(0, 0, 10, 10));"#;


        let effect = EffectDsl::new()
            .compiler()
            .compile(input)
            .expect("effect to be compiled");


        assert_eq!(effect.name(), "sweep_in");
        assert_eq!(format!("{effect:#?}"), format!("{expected:#?}"));
    }

    #[test]
    fn error_unknown_effect() {
        let input = r#"fx::nonexistent()"#;
        let ctx = EffectDsl::new();
        let err = ctx.compiler().compile(input).unwrap_err();
        assert!(matches!(err, DslError::UnknownEffect { .. }));
    }

    #[test]
    fn error_invalid_argument() {
        let input = r#"fx::sweep_in("wrong", 10, 0, Color::from_u32(0x1d2021), 1000)"#;
        let ctx = EffectDsl::new();
        let err = ctx.compiler().compile(input).unwrap_err();
        let actual = "string".to_string();
        assert!(matches!(err, DslError::WrongArgumentType {
            position: 0,
            expected: "motion",
            ref actual
        }), "{:?}", err);
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
        assert!(matches!(err, DslError::InvalidArgumentLength { .. }), "{:?}", err);
    }

    // Error cases
    #[test]
    fn test_compiler_missing_arguments() {
        let dsl = EffectDsl::new();
        let exprs = vec![];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        assert!(compilers::fade_to_fg(&mut args).is_err());
    }

    #[test]
    fn test_compiler_wrong_argument_type() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::String("wrong".to_string())),
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        assert!(compilers::fade_to_fg(&mut args).is_err());
    }
}