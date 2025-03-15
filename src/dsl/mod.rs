mod dsl;
mod arguments;
mod environment;
mod expressions;
mod dsl_format;
mod method_chains;
mod tokenizer;
mod token_parsers;
mod expr_promotion;
mod dsl_writer;

use crate::dsl::expressions::{Expr, ExprSpan};
use compact_str::CompactString;
use std::fmt;

use crate::dsl::token_parsers::parse_ast;
use crate::dsl::tokenizer::{sanitize_tokens, tokenize};
pub use arguments::Arguments;
pub use dsl::{DslCompiler, EffectDsl};
pub use dsl_format::DslFormat;
use dsl_writer::DslWriter;

/// Provides detailed information about errors that occurred while parsing or compiling
/// DSL expressions, including location information and context.
#[derive(Debug)]
pub struct EffectDslError {
    source: DslError,
    pub expression: String,
    pub error_on_line: u32,
    pub error_on_column: u32,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum DslError {
    #[error("Failed to parse dsl: {0}")]
    ParseError(CompactString),

    #[error("Unknown tokenizer error, please consider submitting a bug report")]
    BugInTokenizerError,

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
        location: ExprSpan,
    },

    #[error("Unknown function '{name}'")]
    UnknownFunction {
        name: CompactString,
        location: ExprSpan,
    },

    #[error("Unknown struct '{name}'")]
    UnknownStruct { name: CompactString, location: ExprSpan },

    #[error("Unknown field '{field}' in object '{struct_name}'")]
    UnknownField {
        struct_name: CompactString,
        field: CompactString,
        location: ExprSpan,
    },

    #[error("Missing field '{field}' in object '{struct_name}'")]
    MissingField {
        struct_name: CompactString,
        field: &'static str,
        location: ExprSpan,
    },

    #[error("Invalid argument length. Expected {expected}, got {actual}")]
    InvalidArgumentLength {
        expected: usize,
        actual: usize,
        location: ExprSpan,
    },

    #[error("Invalid expression. Expected {expected}, got {actual}")]
    InvalidExpression {
        expected: &'static str,
        actual: &'static str,
        location: ExprSpan,
    },

    #[error("Failed to cast {from} to expected type {to}")]
    CastOverflow {
        from: &'static str,
        to: &'static str,
        location: ExprSpan,
    },

    #[error("Argument is not of expected type '{expected}', actual type '{actual}'")]
    WrongArgumentType {
        expected: &'static str,
        actual: CompactString,
        location: ExprSpan,
    },

    #[error("Too many arguments for function '{name}'. Expected {count}")]
    TooManyArguments {
        name: CompactString,
        count: usize,
        location: ExprSpan
    },

    #[error("{name} does not provide a to_dsl() implementation")]
    EffectExpressionNotSupported { // fixme: consider moving elsewhere
        name: &'static str,
    },

    #[error("{name} is not supported by the dsl")]
    UnsupportedEffect { // fixme: consider moving elsewhere
        name: CompactString,
    },

    #[error("Array length mismatch. Expected {expected}, got {actual}")]
    ArrayLengthMismatch {
        expected: usize,
        actual: usize,
        location: ExprSpan,
    },

    #[error("Unknown cell filter '{name}'")]
    UnknownCellFilter {
        name: CompactString,
        location: ExprSpan,
    },
}


impl EffectDslError {
    pub(super) fn new(
        input: &str,
        cause: DslError,
    ) -> Self {
        let span = cause.span();

        if let Some(span) = span {
            Self {
                source: cause,
                expression: input[span.start as  _ .. span.end as usize].to_string(),
                error_on_line: input[0 ..span.end as usize].lines().count() as u32,
                error_on_column: span.start - input[0 .. span.end as usize]
                    .rfind("\n")
                    .map_or_else(|| 0, |pos| pos + 1) as u32,
            }
        } else {
            Self {
                source: cause,
                expression: input.to_string(),
                error_on_line: 0,
                error_on_column: 0,
            }
        }
    }
}

impl fmt::Display for EffectDslError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let line_str = format!("line {}", self.error_on_line);
        let col_str = format!("column {}", self.error_on_column);
        let location = format!("at {} {}", line_str, col_str);

        writeln!(f, "Error in DSL expression {}: {}", location, self.source)?;

        // Show the expression with the error marked
        writeln!(f, "Expression: {}", self.expression)
    }
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
        let expr = tokenize(input)
            .map(sanitize_tokens)
            .and_then(parse_ast)?;

        Ok(Self { expr })
    }
}

impl fmt::Display for EffectExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dsl = self.expr.iter()
            .map(DslWriter::format)
            .collect::<Vec<_>>()
            .join("\n");

        write!(f, "{}", dsl)
    }
}

#[cfg(test)]
mod tests {
    use crate::fx;
    use crate::fx::RepeatMode;
    use crate::Shader;
    use indoc::indoc;

    #[test]
    fn to_dsl_format_complex_tree() {
        let expected = indoc! {
            "fx::sequence(&[
                fx::dissolve(EffectTimer::from_ms(100, Interpolation::Linear)),
                fx::parallel(&[
                    fx::dissolve(EffectTimer::from_ms(200, Interpolation::Linear)),
                    fx::dissolve(EffectTimer::from_ms(300, Interpolation::Linear)),
                    fx::sleep(400)
                ]),
                fx::repeat(
                    fx::dissolve(EffectTimer::from_ms(500, Interpolation::Linear)),
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