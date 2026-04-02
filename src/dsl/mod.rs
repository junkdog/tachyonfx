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

/// Errors that can occur during DSL parsing, compilation, or expression conversion.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum DslError {
    /// Lexical analysis failed at the given position.
    #[error("Failed to tokenize the input at position {location}. Check for invalid characters or syntax.")]
    TokenizationError {
        /// Source location of the invalid token.
        location: ExprSpan,
    },

    /// Expression parsing failed due to unexpected tokens or invalid syntax.
    #[error("Failed to parse expression at position {location}. This could be due to unexpected tokens or invalid syntax.")]
    TokenParseError {
        /// Source location of the parse error.
        location: ExprSpan,
    },

    /// An internal parser error that should not normally occur.
    #[error("An unexpected error occurred during parsing. Please report this as a bug with your code sample.")]
    OhNoError,

    /// The referenced effect name is not registered.
    #[error("Unknown effect '{name}'. Check the effect name or register the effect with EffectDsl::register.")]
    UnknownEffect {
        /// The unrecognized effect name.
        name: CompactString,
        /// Source location of the reference.
        location: ExprSpan,
    },

    /// A referenced variable was not declared.
    #[error(
        "Variable '{name}' not found. Make sure it's declared before use with 'let {name} = ...'."
    )]
    UnknownArgument {
        /// The unresolved variable name.
        name: CompactString,
        /// Source location of the reference.
        location: ExprSpan,
    },

    /// A variable exists but does not match the expected type.
    #[error("Cannot find variable '{name}' of type {expected}.")]
    NoSuchVariable {
        /// The variable name.
        name: CompactString,
        /// The type that was expected.
        expected: &'static str,
        /// Source location of the reference.
        location: ExprSpan,
    },

    /// A required positional argument is missing.
    #[error("Missing required argument '{name}' at position {position}. This function requires more arguments.")]
    MissingArgument {
        /// 1-based position of the missing argument.
        position: usize,
        /// Name or type of the expected argument.
        name: &'static str,
        /// Source location of the call.
        location: ExprSpan,
    },

    /// The referenced function name is not recognized.
    #[error("Unknown function '{name}'. Check the function name or import the required module.")]
    UnknownFunction {
        /// The unrecognized function name.
        name: CompactString,
        /// Source location of the call.
        location: ExprSpan,
    },

    /// The referenced struct name is not recognized.
    #[error("Unknown struct '{name}'. Check the struct name or import the required module.")]
    UnknownStruct {
        /// The unrecognized struct name.
        name: CompactString,
        /// Source location of the reference.
        location: ExprSpan,
    },

    /// A struct field name is not valid for the given struct.
    #[error("Unknown field '{field}' in struct '{struct_name}'. Valid fields are {valid_fields}.")]
    UnknownField {
        /// The struct being initialized.
        struct_name: CompactString,
        /// The unrecognized field name.
        field: CompactString,
        /// Source location of the field.
        location: ExprSpan,
        /// Comma-separated list of valid field names.
        valid_fields: CompactString,
    },

    /// A required struct field was not provided.
    #[error("Missing required field '{field}' in struct '{struct_name}'.")]
    MissingField {
        /// The struct being initialized.
        struct_name: CompactString,
        /// The missing field name.
        field: &'static str,
        /// Source location of the struct init expression.
        location: ExprSpan,
    },

    /// The number of arguments does not match the expected count.
    #[error("Invalid number of arguments. Expected {expected}, got {actual}.")]
    InvalidArgumentLength {
        /// Expected argument count.
        expected: usize,
        /// Actual argument count.
        actual: usize,
        /// Source location of the call.
        location: ExprSpan,
    },

    /// An expression has the wrong type or form.
    #[error("Invalid expression. Expected {expected}, but found {actual}.")]
    InvalidExpression {
        /// The type or form that was expected.
        expected: &'static str,
        /// The type or form that was found.
        actual: &'static str,
        /// Source location of the expression.
        location: ExprSpan,
    },

    /// An opening or closing bracket has no matching counterpart.
    #[error("Unmatched {bracket_type} '{bracket}'.")]
    BracketMismatch {
        /// The unmatched bracket character.
        bracket: char,
        /// Source location of the bracket.
        location: ExprSpan,
        /// Whether this is an "opening" or "closing" bracket.
        bracket_type: &'static str,
    },

    /// A let statement is missing a terminating semicolon.
    #[error("Missing semicolon after let statement. Add a ';' to terminate the statement.")]
    MissingSemicolon {
        /// Source location where the semicolon was expected.
        location: ExprSpan,
    },

    /// A comma separator is missing between list elements.
    #[error("Missing comma between elements. Add a ',' to separate items in the list.")]
    MissingComma {
        /// Source location where the comma was expected.
        location: ExprSpan,
    },

    /// A general syntax error with a descriptive message.
    #[error("{message}")]
    SyntaxError {
        /// Description of the syntax error.
        message: CompactString,
        /// Source location of the error.
        location: ExprSpan,
    },

    /// A numeric value overflows when casting to a narrower type.
    #[error("Value cannot be converted from {from} to {to}. The number is out of range for the target type.")]
    CastOverflow {
        /// The source numeric type.
        from: &'static str,
        /// The target numeric type.
        to: &'static str,
        /// Source location of the value.
        location: ExprSpan,
    },

    /// An argument has the wrong type.
    #[error("Type mismatch: expected '{expected}' but found '{actual}'.")]
    WrongArgumentType {
        /// The type that was expected.
        expected: &'static str,
        /// The type that was provided.
        actual: CompactString,
        /// Source location of the argument.
        location: ExprSpan,
    },

    /// More arguments were provided than the function accepts.
    #[error("Too many arguments for function '{name}'. Expected {count} arguments.")]
    TooManyArguments {
        /// The function name.
        name: CompactString,
        /// The maximum number of arguments expected.
        count: usize,
        /// Source location of the excess arguments.
        location: ExprSpan,
    },

    /// The effect cannot be serialized to DSL format.
    #[error("The effect '{name}' cannot be converted to DSL format.")]
    EffectExpressionNotSupported {
        /// The effect name.
        name: &'static str,
    },

    /// The effect is recognized but cannot be compiled from the DSL.
    #[error("The effect '{name}' can not be instantiated by the DSL.")]
    UnsupportedEffect {
        /// The effect name.
        name: CompactString,
        // Consider adding: similar_effects: Vec<CompactString>,
    },

    /// An array literal has the wrong number of elements.
    #[error("Array has incorrect length. Expected {expected} elements, got {actual}.")]
    ArrayLengthMismatch {
        /// Expected element count.
        expected: usize,
        /// Actual element count.
        actual: usize,
        /// Source location of the array.
        location: ExprSpan,
    },

    /// The referenced cell filter variant is not recognized.
    #[error("Invalid cell filter: '{name}'. Valid cell filters include CellFilter::Text, CellFilter::All, etc.")]
    UnknownCellFilter {
        /// The unrecognized cell filter name.
        name: CompactString,
        /// Source location of the reference.
        location: ExprSpan,
    },
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
    ///
    /// # Errors
    ///
    /// Returns [`DslError`] if tokenization or AST parsing fails.
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
