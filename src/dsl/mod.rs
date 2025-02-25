mod parsers;
mod dsl;
mod arguments;
mod environment;
mod expressions;
mod dsl_format;
mod method_chains;

use crate::dsl::expressions::Expr;
use crate::dsl::parsers::parse_expr;
use std::fmt;
use compact_str::CompactString;
pub use arguments::Arguments;
pub use dsl::{DslCompiler, EffectDsl};
pub use dsl_format::DslFormat;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum DslError {
    #[error("Failed to parse dsl: {0}")]
    ParseError(CompactString),

    #[error("Compiler not found for effect '{name}'")]
    UnknownEffect { name: CompactString },

    #[error("Variable '{name}' not found")]
    UnknownArgument { name: CompactString },

    #[error("Invalid argument type '{name}'. Expected {expected}")]
    NoSuchVariable {
        name: CompactString,
        expected: &'static str,
    },

    #[error("Missing required argument '{name}' at position {position}")]
    MissingArgument {
        position: usize,
        name: &'static str,
    },

    #[error("Unknown function '{name}'")]
    UnknownFunction { name: CompactString },

    #[error("Invalid argument length. Expected {expected}, got {actual}")]
    InvalidArgumentLength {
        expected: usize,
        actual: usize,
    },

    #[error("Failed to compile effect: {0}")]
    CompilationError(CompactString),

    #[error("Invalid expression. Expected {expected}, got {actual}")]
    InvalidExpression {
        expected: &'static str,
        actual: &'static str,
    },

    #[error("Failed to cast {from} to expected type {to}")]
    CastOverflow {
        position: usize,
        from: &'static str,
        to: &'static str,
    },

    #[error("Argument at position {position} is not of expected type {expected}, actual {actual}")]
    WrongArgumentType {
        position: usize,
        expected: &'static str,
        actual: CompactString,
    },

    #[error("Too many arguments for function '{name}'. Expected {count}")]
    TooManyArguments {
        name: CompactString,
        count: usize,
    },

    #[error("{name} does not provide a to_dsl() implementation")]
    EffectExpressionNotSupported {
        name: &'static str,
    },

    #[error("{name} is not supported by the dsl")]
    UnsupportedEffect {
        name: CompactString,
    },

    #[error("Array length mismatch. Expected {expected}, got {actual}")]
    ArrayLengthMismatch {
        expected: usize,
        actual: usize,
    },

    #[error("Unknown cell filter '{name}'")]
    UnknownCellFilter { name: CompactString },
}

/// A parsed representation of a tachyonfx effect expression.
///
/// `EffectExpression` provides a way to parse and represent effect descriptions in string form.
/// This allows effects to be defined using a domain-specific language (DSL) syntax and later
/// converted into actual effect instances.
///
/// # Examples
///
/// ```
/// use tachyonfx::dsl::EffectExpression;
///
/// // Parse a simple fade effect
/// let expr = EffectExpression::parse("fx::fade_to(Color::from_u32(0), (500, Linear))").unwrap();
///
/// // Parse a more complex effect chain
/// let expr = EffectExpression::parse(r#"
///     fx::sequence(&[
///         fx::fade_from(Color::Black, Color::from_u32(0), (1000, QuadOut)),
///         fx::dissolve((500, BounceOut))
///     ])
/// "#);
/// ```
///
/// # See Also
///
/// - [`Shader::to_dsl`](crate::Shader::to_dsl) for converting a shader to a DSL expression
/// - [`DslError`] for possible error types
pub struct EffectExpression {
    expr: Vec<Expr>,
}


impl EffectExpression {
    /// Parses a string into an `EffectExpression`.
    ///
    /// This method takes a string containing a tachyonfx effect description and attempts
    /// to parse it into a structured `EffectExpression`. The input string should follow
    /// the tachyonfx DSL syntax.
    ///
    /// # Arguments
    ///
    /// * `input` - A string slice containing the effect expression to parse
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing either:
    /// - `Ok(EffectExpression)` if parsing was successful
    /// - `Err(DslError)` if the input could not be parsed
    pub fn parse(input: &str) -> Result<Self, DslError> {
        let expr = parse_expr(input)?;
        Ok(Self { expr })
    }
}

impl fmt::Display for EffectExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dsl = self.expr.iter()
            .map(|e| e.format(0, false))
            .collect::<Vec<_>>()
            .join(",\n");

        write!(f, "{}", dsl)
    }
}

#[cfg(test)]
mod tests {
    use crate::fx::RepeatMode;
    use crate::Shader;
    use crate::fx;
    use indoc::indoc;

    #[test]
    fn to_dsl_format_complex_tree() {
        let expected = indoc! {
            "fx::sequence(&[
                fx::dissolve(EffectTimer::from_ms(
                    100,
                    Interpolation::Linear
                )),
                fx::parallel(&[
                    fx::dissolve(EffectTimer::from_ms(
                        200,
                        Interpolation::Linear
                    )),
                    fx::dissolve(EffectTimer::from_ms(
                        300,
                        Interpolation::Linear
                    )),
                    fx::sleep(400)
                ]),
                fx::repeat(
                    fx::dissolve(EffectTimer::from_ms(
                        500,
                        Interpolation::Linear
                    )),
                    RepeatMode::Forever
                )
            ])"
        };

        let expr = fx::sequence(&[
            fx::dissolve(100),
            fx::parallel(&[
                fx::dissolve(200),
                fx::dissolve(300),
                fx::sleep(400),
            ]),
            fx::repeat(fx::dissolve(500), RepeatMode::Forever),
        ]).to_dsl().expect("dsl expression from effect");

        assert_eq!(expr.to_string(), expected);
    }


}