mod parsers;
mod dsl;
mod arguments;
mod environment;
mod expressions;
mod dsl_format;

use std::fmt;
use crate::dsl::expressions::Expr;
use crate::dsl::parsers::parse_expr;

pub use dsl_format::DslFormat;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum DslError {
    #[error("Failed to parse dsl: {0}")]
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

    #[error("Argument at position {position} is not of expected type {expected}")]
    WrongArgumentType {
        position: usize,
        expected: &'static str,
    },

    #[error("{name} does not provide a to_dsl() implementation")]
    EffectExpressionNotSupported {
        name: &'static str,
    },

    #[error("{name} is not supported by the dsl")]
    UnsupportedEffect {
        name: String,
    },
}

pub struct EffectExpression {
    expr: Expr,
}


impl EffectExpression {
    pub fn parse(input: &str) -> Result<Self, DslError> {
        let expr = parse_expr(input)?;

        Ok(Self { expr })
    }
}

impl fmt::Display for EffectExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.expr.format(0))
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use regex::Regex;
    use crate::dsl::dsl::EffectDsl;
    use crate::{fx, Effect};
    use crate::fx::RepeatMode;
    use crate::Shader;

    fn assert_effect_to_dsl_to_effect(
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

        // regex, replace SimpleRng { state: 3972560375 } with 'SimpleRng'
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
    fn to_dsl_format_complex_tree() {
        let expected = indoc! {
            "fx::sequence(&[
                fx::dissolve(100),
                fx::parallel(&[
                    fx::dissolve(200),
                    fx::dissolve(300),
                    fx::sleep(400)
                ]),
                fx::repeat(
                    fx::dissolve(500),
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