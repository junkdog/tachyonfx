use crate::dsl::arguments::Arguments;
use crate::dsl::environment::DslEnv;
use crate::dsl::expressions::Expr;
use crate::dsl::parsers::parse_expr;
use crate::dsl::DslError;
use crate::fx::{consume_tick, dissolve, never_complete, ping_pong, repeating};
use crate::{fx, Effect};
use std::fmt;
use std::fmt::Formatter;


struct Interpreter {
    name: &'static str,
    eval: Box<dyn Fn(&mut Arguments) -> Result<Effect, DslError>>,
}

impl Interpreter {
    fn new(
        name: &'static str,
        eval: impl Fn(&mut Arguments) -> Result<Effect, DslError> + 'static
    ) -> Self {
        Self {
            name,
            eval: Box::new(eval),
        }
    }
}

/// A compiler and registry for tachyonfx effect DSL expressions.
///
/// `EffectDsl` manages a collection of interpreters that can compile DSL expressions into
/// concrete effect instances. It comes pre-registered with interpreters for all standard
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
/// let effect = dsl.interpreter().eval("fx::dissolve(500)").unwrap();
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
/// let effect = dsl.interpreter()
///     .bind("motion", Motion::LeftToRight)
///     .bind("c", Color::from_u32(0x1d2021))
///     .eval(input)
///     .unwrap();
/// ```
///
/// # Extending
///
/// While `EffectDsl` comes with all standard effects pre-registered, you can register
/// additional custom effect interpreters if needed:
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
#[derive(Debug)]
pub struct EffectDsl {
    compilers: Vec<Interpreter>,
}


impl EffectDsl {
    /// Creates a new `EffectDsl` instance with all standard effect interpreters registered.
    pub fn new() -> Self {
        register_default_interpreters(Self {
            compilers: Vec::new(),
        })
    }

    /// Registers a new effect compiler with the DSL.
    ///
    /// This method allows extending the DSL with custom effects. The compiler function
    /// receives parsed arguments and should return a concrete `Effect` instance.
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
    ///         # todo!()
    ///     });
    /// ```
    pub fn register(
        self,
        name: &'static str,
        compiler: impl Fn(&mut Arguments) -> Result<Effect, DslError> + 'static
    ) -> Self {
        let mut this = self;
        this.compilers.push(Interpreter::new(name, compiler));
        this
    }

    /// Creates a new DSL interpreter for evaluating effect expressions.
    ///
    /// The interpreter maintains its own environment of bound variables and can
    /// evaluate DSL expressions into concrete `Effect` instances.
    ///
    /// # Returns
    ///
    /// A new `DslInterpreter` instance configured with this DSL's compilers.
    ///
    /// # Examples
    ///
    /// ```
    /// use tachyonfx::dsl::EffectDsl;
    /// use ratatui::style::Color;
    ///
    /// let dsl = EffectDsl::new();
    /// let interpreter = dsl.interpreter()
    ///     .bind("bg_color", Color::Blue);
    ///
    /// // Use bound variables in expressions
    /// let effect = interpreter.eval(r#"
    ///     fx::fade_to(bg_color, (1000, Linear))
    /// "#);
    /// ```
    pub fn interpreter(&self) -> DslInterpreter {
        DslInterpreter {
            dsl: self,
            environment: DslEnv::new(),
        }
    }

    pub(super) fn eval(
        &self,
        env: &DslEnv,
        input: Expr
    ) -> Result<Effect, DslError> {
        match input {
            Expr::Fx { name, arguments } => self.compilers
                .iter()
                .find(|d| d.name == name)
                .ok_or(DslError::UnknownEffect { name })
                .and_then(|d| {
                    let mut args = Arguments::new(arguments.into(), self, env);
                    let effect = (d.eval)(&mut args);

                    match () {
                        _ if effect.is_err() => effect,
                        // todo: check on each interpreter if there are any remaining arguments
                        _ if !args.args().is_empty() => Err(DslError::TooManyArguments {
                            expected: args.original_arg_count() - args.args().len(),
                            actual: args.original_arg_count(),
                        }),
                        _ => effect,
                    }
                }),
            Expr::Sequence(exprs) => {
                let mut args = Arguments::new(exprs.into(), self, env);
                let effects = (0..args.args_count())
                    .map(|_| args.effect())
                    .collect::<Result<Vec<Effect>, DslError>>()?;

                Ok(fx::sequence(&effects))
            },
            Expr::Parallel(exprs) => {
                let mut args = Arguments::new(exprs.into(), self, env);
                let effects = (0..args.args_count())
                    .map(|_| args.effect())
                    .collect::<Result<Vec<Effect>, DslError>>()?;

                Ok(fx::parallel(&effects))
            },
            _ => Err(DslError::InvalidExpression {
                expected: "effect",
                actual: input.type_name(),
            }),
        }
    }
}

/// An interpreter that can evaluate tachyonfx DSL expressions into concrete effects.
///
/// The interpreter maintains its own environment of bound variables that can be referenced
/// in effect expressions. It uses its parent `EffectDsl` to compile the expressions.
///
/// ### See also:
/// - [`EffectDsl::interpreter`](EffectDsl::interpreter) for creating a new interpreter
pub struct DslInterpreter<'ctx> {
    dsl: &'ctx EffectDsl,
    environment: DslEnv,
}

impl DslInterpreter<'_> {

    /// Binds a value to a name in the interpreter's environment.
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

    /// Evaluates a DSL expression string into a concrete effect.
    ///
    /// # Arguments
    ///
    /// * `input` - The DSL expression to evaluate
    ///
    /// # Returns
    ///
    /// Returns either:
    /// - `Ok(Effect)` if evaluation succeeds
    /// - `Err(DslError)` if parsing or compilation fails
    ///
    /// # Examples
    ///
    /// ```
    /// use tachyonfx::dsl::EffectDsl;
    ///
    /// let effect = EffectDsl::new()
    ///     .interpreter()
    ///     .eval("fx::dissolve(500)")
    ///     .unwrap();
    /// ```
    pub fn eval(self, input: &str) -> Result<Effect, DslError> {
        parse_expr(input)
            .and_then(|expr| self.dsl.eval(&self.environment, expr))
    }
}

fn register_default_interpreters(effect_dsl: EffectDsl) -> EffectDsl {
    effect_dsl
        .register("term256_colors", |_args| fx::term256_colors().into())
        .register("coalesce",       interpreters::coalesce)
        .register("coalesce_from",  interpreters::coalesce_from)
        .register("consume_tick",   |_args| consume_tick().into())
        .register("delay",          interpreters::delay)
        .register("dissolve",       |args| dissolve(args.effect_timer()?).into())
        .register("dissolve_to",    interpreters::dissolve_to)
        .register("fade_from",      interpreters::fade_from)
        .register("fade_from_fg",   interpreters::fade_from_fg)
        .register("fade_to",        interpreters::fade_to)
        .register("fade_to_fg",     interpreters::fade_to_fg)
        .register("never_complete", |args| never_complete(args.effect()?).into())
        .register("ping_pong",      |args| ping_pong(args.effect()?).into())
        .register("prolong_end",    interpreters::prolong_end)
        .register("prolong_start",  interpreters::prolong_start)
        .register("repeat",         interpreters::repeat)
        .register("sleep",          interpreters::sleep)
        .register("repeating",      |args| repeating(args.effect()?).into())
        .register("slide_in",       interpreters::slide_in)
        .register("slide_out",      interpreters::slide_out)
        .register("sweep_in",       interpreters::sweep_in)
        .register("sweep_out",      interpreters::sweep_out)
        .register("with_duration",  interpreters::with_duration)
        .register(
            "timed_never_complete",
            interpreters::timed_never_complete
        )
}

impl From<Effect> for Result<Effect, DslError> {
    fn from(effect: Effect) -> Self {
        Ok(effect)
    }
}

mod interpreters {
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

impl fmt::Debug for Interpreter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Interpreter")
            .field("name", &self.name)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::dsl::expressions::{Expr, Value};
    use crate::fx::RepeatMode;
    use crate::Interpolation::QuadOut;
    use crate::{fx, Duration, Effect, EffectTimer, Interpolation, Motion, Shader};
    use ratatui::style::{Color, Style};
    use regex::Regex;
    use std::collections::VecDeque;
    use ratatui::prelude::Modifier;
    use Interpolation::Linear;
    use crate::dsl::arguments::Arguments;
    use crate::dsl::dsl::{interpreters, EffectDsl};
    use crate::dsl::DslError;
    use crate::dsl::environment::DslEnv;

    fn assert_effect_roundtrip_eq(
        effect: Effect,
    ) {
        let expr = effect
            .to_dsl()
            .expect("dsl expression from effect")
            .to_string();

        let dsl = EffectDsl::new();
        let actual = dsl.interpreter()
            .eval(&expr)
            .expect("effect from evaluating dsl expression");

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
    fn test_interpreter_dsl_roundtrips() {
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

        let dsl = EffectDsl::new();
        let effect = dsl.interpreter().eval(input)
            .expect("effect to be compiled");

        assert_eq!(effect.name(), "sweep_in");
        assert_eq!(effect.timer(), Some(EffectTimer::from_ms(1000, QuadOut)));
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
        let effect = dsl.interpreter()
            .bind("motion", Motion::LeftToRight)
            .bind("c", Color::from_u32(0x1d2021))
            .eval(input)
            .expect("effect to be compiled");


        assert_eq!(effect.name(), "sweep_in");
        assert_eq!(format!("{effect:?}"), format!("{expected:?}"));
    }

    #[test]
    fn error_unknown_effect() {
        let input = r#"fx::nonexistent()"#;
        let ctx = EffectDsl::new();
        let err = ctx.interpreter().eval(input).unwrap_err();
        assert!(matches!(err, DslError::UnknownEffect { .. }));
    }

    #[test]
    fn error_invalid_argument() {
        let input = r#"fx::sweep_in("wrong", 10, 0, Color::from_u32(0x1d2021), 1000)"#;
        let ctx = EffectDsl::new();
        let err = ctx.interpreter().eval(input).unwrap_err();
        assert!(matches!(err, DslError::WrongArgumentType {
            position: 0,
            expected: "motion"
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
        let err = ctx.interpreter().eval(input).unwrap_err();
        assert!(matches!(err, DslError::TooManyArguments { .. }), "{:?}", err);
    }

    // Error cases
    #[test]
    fn test_interpreter_missing_arguments() {
        let dsl = EffectDsl::new();
        let exprs = vec![];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        assert!(interpreters::fade_to_fg(&mut args).is_err());
    }

    #[test]
    fn test_interpreter_wrong_argument_type() {
        let dsl = EffectDsl::new();
        let exprs = vec![
            Expr::Literal(Value::String("wrong".to_string())),
            Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Linear)))
        ];
        let env = DslEnv::new();
        let mut args = Arguments::new(VecDeque::from(exprs), &dsl, &env);
        assert!(interpreters::fade_to_fg(&mut args).is_err());
    }
}