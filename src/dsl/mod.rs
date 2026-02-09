mod arguments;
mod completions;
#[allow(clippy::module_inception)]
mod dsl;
pub(crate) mod dsl_format;
mod dsl_writer;
mod environment;
mod expr_promotion;
mod expressions;
mod method_chains;
mod parse_error;
mod token_parsers;
mod token_verification;
mod tokenizer;

use alloc::fmt;

pub use arguments::Arguments;
use compact_str::CompactString;
pub use completions::{CompletionEngine, CompletionItem, CompletionKind};
pub use dsl::{DslCompiler, EffectDsl};
pub use dsl_format::DslFormat;
use dsl_writer::DslWriter;
pub use parse_error::DslParseError;

use crate::dsl::{
    expressions::{Expr, ExprSpan},
    token_parsers::parse_ast,
    tokenizer::{sanitize_tokens, tokenize},
};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum DslError {
    #[error("Failed to tokenize the input at position {location}. Check for invalid characters or syntax.")]
    TokenizationError { location: ExprSpan },

    #[error("Failed to parse expression at position {location}. This could be due to unexpected tokens or invalid syntax.")]
    TokenParseError { location: ExprSpan },

    #[error("An unexpected error occurred during parsing. Please report this as a bug with your code sample.")]
    OhNoError,

    #[error("Unknown effect '{name}'. Check the effect name or register the effect with EffectDsl::register.")]
    UnknownEffect { name: CompactString, location: ExprSpan },

    #[error(
        "Variable '{name}' not found. Make sure it's declared before use with 'let {name} = ...'."
    )]
    UnknownArgument { name: CompactString, location: ExprSpan },

    #[error("Cannot find variable '{name}' of type {expected}.")]
    NoSuchVariable {
        name: CompactString,
        expected: &'static str,
        location: ExprSpan,
    },

    #[error("Missing required argument '{name}' at position {position}. This function requires more arguments.")]
    MissingArgument {
        position: usize,
        name: &'static str,
        location: ExprSpan,
    },

    #[error("Unknown function '{name}'. Check the function name or import the required module.")]
    UnknownFunction { name: CompactString, location: ExprSpan },

    #[error("Unknown struct '{name}'. Check the struct name or import the required module.")]
    UnknownStruct { name: CompactString, location: ExprSpan },

    #[error("Unknown field '{field}' in struct '{struct_name}'. Valid fields are {valid_fields}.")]
    UnknownField {
        struct_name: CompactString,
        field: CompactString,
        location: ExprSpan,
        valid_fields: CompactString,
    },

    #[error("Missing required field '{field}' in struct '{struct_name}'.")]
    MissingField {
        struct_name: CompactString,
        field: &'static str,
        location: ExprSpan,
    },

    #[error("Invalid number of arguments. Expected {expected}, got {actual}.")]
    InvalidArgumentLength { expected: usize, actual: usize, location: ExprSpan },

    #[error("Invalid expression. Expected {expected}, but found {actual}.")]
    InvalidExpression {
        expected: &'static str,
        actual: &'static str,
        location: ExprSpan,
    },

    #[error("Unmatched {bracket_type} '{bracket}'.")]
    BracketMismatch {
        bracket: char,
        location: ExprSpan,
        bracket_type: &'static str, // "opening" or "closing"
    },

    #[error("Missing semicolon after let statement. Add a ';' to terminate the statement.")]
    MissingSemicolon { location: ExprSpan },

    #[error("Missing comma between elements. Add a ',' to separate items in the list.")]
    MissingComma { location: ExprSpan },

    #[error("{message}")]
    SyntaxError { message: CompactString, location: ExprSpan },

    #[error("Value cannot be converted from {from} to {to}. The number is out of range for the target type.")]
    CastOverflow {
        from: &'static str,
        to: &'static str,
        location: ExprSpan,
    },

    #[error("Type mismatch: expected '{expected}' but found '{actual}'.")]
    WrongArgumentType {
        expected: &'static str,
        actual: CompactString,
        location: ExprSpan,
    },

    #[error("Too many arguments for function '{name}'. Expected {count} arguments.")]
    TooManyArguments {
        name: CompactString,
        count: usize,
        location: ExprSpan,
    },

    #[error("The effect '{name}' cannot be converted to DSL format.")]
    EffectExpressionNotSupported { name: &'static str },

    #[error("The effect '{name}' can not be instantiated by the DSL.")]
    UnsupportedEffect {
        name: CompactString,
        // Consider adding: similar_effects: Vec<CompactString>,
    },

    #[error("Array has incorrect length. Expected {expected} elements, got {actual}.")]
    ArrayLengthMismatch { expected: usize, actual: usize, location: ExprSpan },

    #[error("Invalid cell filter: '{name}'. Valid cell filters include CellFilter::Text, CellFilter::All, etc.")]
    UnknownCellFilter { name: CompactString, location: ExprSpan },
}

/// A parsed representation of a tachyonfx effect expression.
///
/// `EffectExpression` provides a way to parse and represent effect descriptions in string
/// form. This allows effects to be defined using a domain-specific language (DSL) syntax
/// and later converted into actual effect instances.
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
/// - [`Shader::to_dsl`](crate::Shader::to_dsl) for converting a shader to a DSL
///   expression
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
        let dsl = self
            .expr
            .iter()
            .map(DslWriter::format)
            .collect::<Vec<_>>()
            .join("\n");

        write!(f, "{dsl}")
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use crate::{fx, fx::RepeatMode};

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
            fx::parallel(&[fx::dissolve(200), fx::dissolve(300), fx::sleep(400)]),
            fx::repeat(fx::dissolve(500), RepeatMode::Forever),
        ])
        .to_dsl()
        .expect("dsl expression from effect");

        assert_eq!(expr.to_string(), expected);
    }
}
