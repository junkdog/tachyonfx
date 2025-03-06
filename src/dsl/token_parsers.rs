use anpa::{create_parser_trait, defer_parser, or, right, tuplify};
use anpa::combinators::{many_to_vec, middle, no_separator, right, separator};
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

    item_if(|_| true)
        .map_if(|t: &'a Token<'a>| Some(Expr::Literal(match t.kind {
            StringLiteral => Value::String(t.text.into()),
            FloatLiteral  => Value::F32(t.text.parse().unwrap()),
            IntLiteral    => Value::I32(t.text.parse().unwrap()),
            _             => None?,
        })))
}

fn variable<'a>() -> impl TokenParser<'a, Expr> {
    identifier()
        .map(|t| Expr::Var { name: t.into(), self_fns: vec![] })
}

fn argument<'a>() -> impl TokenParser<'a, Expr> {
    or!(
        literal(),
        defer_parser!(fn_call().map(|f| Expr::FnCall { call: f, self_fns: vec![] })),
        qualified_member(),
        variable(),
    )
}

fn arguments<'a>() -> impl TokenParser<'a, Vec<Expr>> {
    use TokenKind::*;

    many_to_vec(argument(), true, separator(token(Comma), true))
}

fn qualified_member<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    tuplify!(
        identifier(),
        token(DoubleColon),
        identifier()
    ).map(|(owner, _, member)| {
        Expr::QualifiedMember(format_compact!("{owner}::{member}"))
    })
}

fn fn_call<'a>() -> impl TokenParser<'a, FnCallInfo> {
    use TokenKind::*;

    tuplify!(
        identifier(),
        token(DoubleColon),
        identifier(),
        middle(token(LeftParen), arguments(), token(RightParen)),
        chained_fns() // todo: encapsulate in a separate parser
    ).map(|(owner, _, fun, args, self_fns)| {
        FnCallInfo::new(format_compact!("{owner}::{fun}"), args)
    })
}

fn chained_fns<'a>() -> impl TokenParser<'a, Vec<FnCallInfo>> {
    let chained_fn = right!(token(TokenKind::Dot), defer_parser!(fn_call()));
    many_to_vec(chained_fn, true, no_separator())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anpa::core::parse;
    use compact_str::ToCompactString;
    use crate::dsl::tokenizer::tokenize;
    use crate::dsl::expressions::{Expr, Value};

    // Helper function to run tests on tokenized input
    fn with_tokens(input: &str, f: impl FnOnce(&[Token])) {
        const DISCARD: &[TokenKind] = &[
            TokenKind::Whitespace,
            TokenKind::LineComment,
            TokenKind::BlockComment
        ];

        let tokens = tokenize(input)
            .map(|tokens| {
                tokens.into_iter()
                    .filter(|t| !DISCARD.contains(&t.kind))
                    .collect::<Vec<_>>()
            })
            .unwrap();

        f(&tokens);
    }

    #[test]
    fn test_token_parser() {
        with_tokens("test", |tokens| {
            // Test matching token kind
            assert_eq!(
                parse(token(TokenKind::Identifier), tokens).result.map(|t| t.text),
                Some("test")
            );

            // Test non-matching token kind
            assert_eq!(
                parse(token(TokenKind::StringLiteral), tokens).result,
                None
            );
        });
    }

    #[test]
    fn test_identifier_parser() {
        // Test with a simple identifier
        with_tokens("variable_name", |tokens| {
            assert_eq!(
                parse(identifier(), tokens).result,
                Some("variable_name")
            );
        });

        // Test with a token that's not an identifier
        with_tokens("123", |tokens| {
            assert_eq!(
                parse(identifier(), tokens).result,
                None
            );
        });
    }

    #[test]
    fn test_literal_parser() {
        // Test with string literal
        with_tokens("\"hello world\"", |tokens| {
            assert_eq!(
                parse(literal(), tokens).result,
                Some(Expr::Literal(Value::String("hello world".into())))
            );
        });

        // Test with integer literal
        with_tokens("42", |tokens| {
            assert_eq!(
                parse(literal(), tokens).result,
                Some(Expr::Literal(Value::I32(42)))
            );
        });

        // Test with float literal
        with_tokens("3.14", |tokens| {
            assert_eq!(
                parse(literal(), tokens).result,
                Some(Expr::Literal(Value::F32(3.14)))
            );
        });

        // Test with non-literal token
        with_tokens("variable", |tokens| {
            assert_eq!(
                parse(literal(), tokens).result,
                None
            );
        });
    }

    #[test]
    fn test_variable_parser() {
        // Test with valid identifier
        with_tokens("my_var", |tokens| {
            assert_eq!(
                parse(variable(), tokens).result,
                Some(Expr::Var {
                    name: "my_var".into(),
                    self_fns: vec![]
                })
            );
        });

        // Test with non-identifier token
        with_tokens("123", |tokens| {
            assert_eq!(
                parse(variable(), tokens).result,
                None
            );
        });
    }

    #[test]
    fn test_argument_parser() {
        // Test with literal
        with_tokens("42", |tokens| {
            assert_eq!(
                parse(argument(), tokens).result,
                Some(Expr::Literal(Value::I32(42)))
            );
        });

        // Test with variable
        with_tokens("my_var", |tokens| {
            assert_eq!(
                parse(argument(), tokens).result,
                Some(Expr::Var {
                    name: "my_var".into(),
                    self_fns: vec![]
                })
            );
        });

        // Test with function call
        with_tokens("fx::fade_to()", |tokens| {
            assert_eq!(
                parse(argument(), tokens).result,
                Some(Expr::FnCall {
                    call: FnCallInfo {
                        name: "fx::fade_to".into(),
                        args: vec![]
                    },
                    self_fns: vec![]
                })
            );
        });
    }

    #[test]
    fn test_arguments_parser() {
        // Test with empty arguments
        with_tokens("", |tokens| {
            assert_eq!(
                parse(arguments(), tokens).result,
                Some(vec![])
            );
        });

        // Test with single argument
        with_tokens("42", |tokens| {
            assert_eq!(
                parse(arguments(), tokens).result,
                Some(vec![Expr::Literal(Value::I32(42))])
            );
        });

        // Test with multiple arguments
        with_tokens("42, \"hello\", my_var", |tokens| {
            assert_eq!(
                parse(arguments(), tokens).result,
                Some(vec![
                    Expr::Literal(Value::I32(42)),
                    Expr::Literal(Value::String("hello".into())),
                    Expr::Var { name: "my_var".into(), self_fns: vec![] }
                ])
            );
        });
    }

    #[test]
    fn test_fn_call_parser() {
        // Test with no arguments
        with_tokens("fx::fade_to()", |tokens| {
            assert_eq!(
                parse(fn_call(), tokens).result,
                Some(FnCallInfo {
                    name: "fx::fade_to".into(),
                    args: vec![]
                })
            );
        });

        // Test with arguments
        with_tokens("fx::fade_to(42, \"hello\", my_var)", |tokens| {
            assert_eq!(
                parse(fn_call(), tokens).result,
                Some(FnCallInfo {
                    name: "fx::fade_to".into(),
                    args: vec![
                        Expr::Literal(Value::I32(42)),
                        Expr::Literal(Value::String("hello".into())),
                        Expr::Var { name: "my_var".into(), self_fns: vec![] }
                    ]
                })
            );
        });

        // Test with qualified member in arguments
        with_tokens("fx::fade_to(Color::Red, 500)", |tokens| {
            assert_eq!(
                parse(fn_call(), tokens).result,
                Some(FnCallInfo {
                    name: "fx::fade_to".into(),
                    args: vec![
                        Expr::QualifiedMember("Color::Red".into()),
                        Expr::Literal(Value::I32(500))
                    ]
                })
            );
        });

        // Test with invalid format
        with_tokens("fade_to()", |tokens| {
            assert_eq!(
                parse(fn_call(), tokens).result,
                None
            );
        });
    }
}