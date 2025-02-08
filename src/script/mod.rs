mod parser;
mod script;
mod args;
mod env;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ScriptError {
    #[error("Failed to parse script: {0}")]
    ParseError(String),

    #[error("Compiler not found for effect '{name}'")]
    UnknownEffect { name: String },

    #[error("Variable '{name}' not found")]
    UnknownArgument { name: String },

    #[error("Invalid argument type at position {position}. Expected {expected}")]
    NoSuchVariable {
        position: usize,
        name: String,
        expected: &'static str,
    },

    #[error("Missing required argument '{name}' at position {position}")]
    MissingArgument {
        position: usize,
        name: &'static str,
    },

    #[error("Too many arguments provided. Expected {expected}, got {actual}")]
    TooManyArguments {
        expected: usize,
        actual: usize,
    },

    #[error("Failed to compile effect: {0}")]
    CompilationError(String),

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

    #[error("Argument '{name}' at position {position} is not of expected type {expected}")]
    WrongArgumentType {
        position: usize,
        name: String,
        expected: &'static str,
    },
}