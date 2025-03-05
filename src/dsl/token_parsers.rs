use anpa::{create_parser_trait, defer_parser, or, right, tuplify};
use anpa::combinators::{many_to_vec, middle, separator};
use anpa::core::ParserExt;
use anpa::parsers::item_if;
use compact_str::format_compact;
use crate::dsl::expressions::{Expr, FnCallInfo, Value};
use crate::dsl::tokenizer::{Token, TokenKind};
use crate::dsl::tokenizer::TokenKind::{FloatLiteral, IntLiteral, StringLiteral};

create_parser_trait!(TokenParser, [Token<'a>], "effect dsl token parser");

fn token<'a>(kind: TokenKind) -> impl TokenParser<'a, &'a Token<'a>> {
    item_if(move |t: &'a Token<'a>| t.kind == kind)
}

fn identifier<'a>() -> impl TokenParser<'a, &'a str> {
    token(TokenKind::Identifier).map(|t| t.text)
}

fn literal<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    item_if(|t: &'a Token<'a>| true)
        .map_if(|t: &'a Token<'a>| Some(Expr::Literal(match t.kind {
            StringLiteral => Value::String(t.text.into()),
            FloatLiteral  => Value::F32(t.text.parse().unwrap()),
            IntLiteral    => Value::I32(t.text.parse().unwrap()),
            _ => None?,
        })))
}

fn variable<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    identifier()
        .map(|t| Expr::Var { name: t.into(), self_fns: vec![] })
}

fn argument<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    or!(
        literal(),
        defer_parser!(fn_call()),
        variable(),
    )
}

fn arguments<'a>() -> impl TokenParser<'a, Vec<Expr>> {
    use TokenKind::*;

    many_to_vec(argument(), true, separator(token(Comma), true))
}

fn fn_call<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    tuplify!(
        identifier(),
        token(DoubleColon),
        identifier(),
        middle(token(LeftParen), arguments(), token(RightParen))
    ).map(|(owner, _, fun, args)| {
        let call = FnCallInfo::new(format_compact!("{owner}::{fun}"), args);
        Expr::FnCall { call, self_fns: vec![] }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anpa::core::parse;
    use crate::dsl::tokenizer::tokenize;
    use crate::dsl::expressions::{Expr, Value};

    // Helper function to run tests on tokenized input
    fn with_tokens<F>(input: &str, f: F)
    where
        F: FnOnce(&[Token])
    {
        let tokens = tokenize(input).unwrap();
        f(&tokens);
    }

    #[test]
    fn test_token_parser() {
        with_tokens("test", |tokens| {
            // Test matching token kind
            let result = parse(token(TokenKind::Identifier), tokens);
            assert!(result.result.is_some());
            assert_eq!(result.result.unwrap().text, "test");

            // Test non-matching token kind
            let result = parse(token(TokenKind::StringLiteral), tokens);
            assert!(result.result.is_none());
        });
    }

    #[test]
    fn test_identifier_parser() {
        // Test with a simple identifier
        with_tokens("variable_name", |tokens| {
            let result = parse(identifier(), tokens);
            assert_eq!(result.result, Some("variable_name"));
        });

        // Test with a token that's not an identifier
        with_tokens("123", |tokens| {
            let result = parse(identifier(), tokens);
            assert_eq!(result.result, None);
        });
    }

    #[test]
    fn test_literal_parser() {
        // Test with string literal
        with_tokens("\"hello world\"", |tokens| {
            let result = parse(literal(), tokens).result;
            match result {
                Some(Expr::Literal(Value::String(s))) => assert_eq!(s, "hello world"),
                _ => panic!("Expected string literal, got {:?}", result),
            }
        });

        // Test with integer literal
        with_tokens("42", |tokens| {
            let result = parse(literal(), tokens).result;
            match result {
                Some(Expr::Literal(Value::I32(i))) => assert_eq!(i, 42),
                _ => panic!("Expected integer literal, got {:?}", result),
            }
        });

        // Test with float literal
        with_tokens("3.14", |tokens| {
            let result = parse(literal(), tokens).result;
            match result {
                Some(Expr::Literal(Value::F32(f))) => assert!((f - 3.14).abs() < f32::EPSILON),
                _ => panic!("Expected float literal, got {:?}", result),
            }
        });

        // Test with non-literal token
        with_tokens("variable", |tokens| {
            let result = parse(literal(), tokens).result;
            assert_eq!(result, None);
        });
    }

    #[test]
    fn test_variable_parser() {
        // Test with valid identifier
        with_tokens("my_var", |tokens| {
            let result = parse(variable(), tokens).result;
            match result {
                Some(Expr::Var { name, self_fns }) => {
                    assert_eq!(name, "my_var");
                    assert!(self_fns.is_empty());
                },
                _ => panic!("Expected variable, got {:?}", result),
            }
        });

        // Test with non-identifier token
        with_tokens("123", |tokens| {
            let result = parse(variable(), tokens).result;
            assert_eq!(result, None);
        });
    }

    #[test]
    fn test_argument_parser() {
        // Test with literal
        with_tokens("42", |tokens| {
            let result = parse(argument(), tokens).result;
            match result {
                Some(Expr::Literal(Value::I32(i))) => assert_eq!(i, 42),
                _ => panic!("Expected integer literal argument, got {:?}", result),
            }
        });

        // Test with variable
        with_tokens("my_var", |tokens| {
            let result = parse(argument(), tokens).result;
            match result {
                Some(Expr::Var { name, .. }) => assert_eq!(name, "my_var"),
                _ => panic!("Expected variable argument, got {:?}", result),
            }
        });

        // Test with function call
        with_tokens("fx::fade_to()", |tokens| {
            let result = parse(argument(), tokens).result;
            match result {
                Some(Expr::FnCall { call, .. }) => {
                    assert_eq!(call.name, "fx::fade_to");
                    assert!(call.args.is_empty());
                },
                _ => panic!("Expected function call argument, got {:?}", result),
            }
        });
    }

    #[test]
    fn test_arguments_parser() {
        // Test with empty arguments
        with_tokens("", |tokens| {
            let result = parse(arguments(), tokens).result;
            assert_eq!(result, Some(vec![]));
        });

        // Test with single argument
        with_tokens("42", |tokens| {
            let result = parse(arguments(), tokens).result;
            assert_eq!(result.unwrap().len(), 1);
        });

        // Test with multiple arguments
        with_tokens("42, \"hello\", my_var", |tokens| {
            let result = parse(arguments(), tokens).result;
            let args = result.unwrap();
            assert_eq!(args.len(), 3, "Expected 3 arguments, got {:?}", args);

            match &args[0] {
                Expr::Literal(Value::I32(i)) => assert_eq!(*i, 42),
                _ => panic!("Expected integer literal, got {:?}", &args[0]),
            }

            match &args[1] {
                Expr::Literal(Value::String(s)) => assert_eq!(s, "hello"),
                _ => panic!("Expected string literal, got {:?}", &args[1]),
            }

            match &args[2] {
                Expr::Var { name, .. } => assert_eq!(name, "my_var"),
                _ => panic!("Expected variable, got {:?}", &args[2]),
            }
        });
    }

    #[test]
    fn test_fn_call_parser() {
        // Test with no arguments
        with_tokens("fx::fade_to()", |tokens| {
            let result = parse(fn_call(), tokens).result;
            match result {
                Some(Expr::FnCall { call, self_fns }) => {
                    assert_eq!(call.name, "fx::fade_to");
                    assert!(call.args.is_empty());
                    assert!(self_fns.is_empty());
                },
                _ => panic!("Expected function call, got {:?}", result),
            }
        });

        // Test with arguments
        with_tokens("fx::fade_to(42, \"hello\", my_var)", |tokens| {
            let result = parse(fn_call(), tokens).result;
            match result {
                Some(Expr::FnCall { call, self_fns }) => {
                    assert_eq!(call.name, "fx::fade_to");
                    assert_eq!(call.args.len(), 3);
                    assert!(self_fns.is_empty());
                },
                _ => panic!("Expected function call with arguments, got {:?}", result),
            }
        });

        // Test with nested function call in arguments
        with_tokens("fx::fade_to(Color::Red(), 500)", |tokens| {
            let result = parse(fn_call(), tokens).result;
            match result {
                Some(Expr::FnCall { call, .. }) => {
                    assert_eq!(call.name, "fx::fade_to");
                    assert_eq!(call.args.len(), 2);

                    match &call.args[0] {
                        Expr::FnCall { call: inner_call, .. } => {
                            assert_eq!(inner_call.name, "Color::Red");
                            assert!(inner_call.args.is_empty());
                        },
                        _ => panic!("Expected nested function call, got {:?}", &call.args[0]),
                    }
                },
                _ => panic!("Expected function call with nested call, got {:?}", result),
            }
        });

        // Test with invalid format
        with_tokens("fade_to()", |tokens| {
            let result = parse(fn_call(), tokens).result;
            assert_eq!(result, None);
        });
    }

    #[test]
    fn test_complex_fn_call() {
        // Test a more complex function call with multiple nested arguments
        let input = "fx::sequence(fx::fade_to(Color::Red(), 500), fx::dissolve(1000))";
        with_tokens(input, |tokens| {
            let result = parse(fn_call(), tokens).result;

            match result {
                Some(Expr::FnCall { call, .. }) => {
                    assert_eq!(call.name, "fx::sequence");
                    assert_eq!(call.args.len(), 2);

                    // Check first argument (fade_to)
                    match &call.args[0] {
                        Expr::FnCall { call: inner_call, .. } => {
                            assert_eq!(inner_call.name, "fx::fade_to");
                            assert_eq!(inner_call.args.len(), 2);
                        },
                        _ => panic!("Expected nested function call for first arg, got {:?}", &call.args[0]),
                    }

                    // Check second argument (dissolve)
                    match &call.args[1] {
                        Expr::FnCall { call: inner_call, .. } => {
                            assert_eq!(inner_call.name, "fx::dissolve");
                            assert_eq!(inner_call.args.len(), 1);
                        },
                        _ => panic!("Expected nested function call for second arg, got {:?}", &call.args[1]),
                    }
                },
                _ => panic!("Expected complex function call, got {:?}", result),
            }
        });
    }
}