use anpa::{
    combinators::{and_parsed, attempt, many_to_vec, middle, no_separator, separator, succeed},
    core::{parse, ParserExt},
    create_parser_trait, or,
    parsers::item_if,
    right, tuplify,
};
use compact_str::{format_compact, ToCompactString};

use crate::dsl::{
    expr_promotion::maybe_promote,
    expressions::{Expr, ExprSpan, FnCallInfo, Value},
    tokenizer::{Token, TokenKind},
    DslError,
};

create_parser_trait!(TokenParser, [Token<'a>], "effect dsl token parser");

// region main parsers with pub(super) visibility

#[allow(clippy::needless_pass_by_value)] // used with .and_then(parse_ast)
pub(super) fn parse_ast(input: Vec<Token>) -> Result<Vec<Expr>, DslError> {
    let statements = many_to_vec(
        or!(
            let_binding(),
            sequence(),
            parallel(),
            function_expression(),
            struct_instantiation(),
            variable().map(maybe_promote)
        ),
        true,
        separator(token(TokenKind::Semicolon), false),
    );

    let ast = parse(statements, &input);
    if !ast.state.is_empty() {
        return Err(DslError::TokenParseError {
            location: ast
                .state
                .first()
                .map_or(ExprSpan::default(), |t| ExprSpan::new(t.span.0, t.span.1)),
        });
    };

    match ast.result {
        Some(exprs) => Ok(exprs),
        None => Err(DslError::OhNoError),
    }
}

pub(super) fn expression<'a>() -> impl TokenParser<'a, Expr> {
    or!(
        boolean(),
        literal(),
        let_binding(),
        sequence(),
        parallel(),
        some(),
        function_expression(),
        macro_expr(),
        array(),
        struct_instantiation(),
        qualified_name().map(maybe_promote),
        variable().map(maybe_promote),
        tuple(),
    )
}

// endregion

// region supporting parsers

fn yield_consumed<'a, T>(parser: impl TokenParser<'a, T>) -> impl TokenParser<'a, (ExprSpan, T)> {
    and_parsed(parser).map(|(consumed_tokens, value)| {
        // Calculate the span from the first and last token
        let start = consumed_tokens.first().map_or(0, |t| t.span.0);
        let end = consumed_tokens.last().map_or(start, |t| t.span.1);

        (ExprSpan { start, end }, value)
    })
}

fn maybe_qualified<'a>(owner: &str) -> impl TokenParser<'a, ()> + use<'a, '_> {
    use TokenKind::*;

    succeed(attempt(right!(id(owner), token(DoubleColon)))).map(|_| ())
}

fn identifier<'a>() -> impl TokenParser<'a, &'a str> {
    token(TokenKind::Identifier).map(|t| t.text)
}

fn function_call<'a>() -> impl TokenParser<'a, FnCallInfo> {
    use TokenKind::*;

    let qualified = yield_consumed(tuplify!(
        identifier(),
        token(DoubleColon),
        identifier(),
        within(LeftParen, arguments(), RightParen),
    ))
    .map(|(span, (owner, _, fun, args))| {
        FnCallInfo::new(format_compact!("{owner}::{fun}"), args, span)
    });

    let unqualified = yield_consumed(tuplify!(
        identifier(),
        within(LeftParen, arguments(), RightParen),
    ))
    .map(|(span, (fun, args))| FnCallInfo::new(fun, args, span));

    or!(qualified, unqualified)
}

fn method_chain<'a>() -> impl TokenParser<'a, Vec<FnCallInfo>> {
    use TokenKind::*;

    let chained_fn = yield_consumed(tuplify!(
        token(Dot),
        identifier(),
        within(LeftParen, arguments(), RightParen),
    ))
    .map(|(span, (_, fun, args))| FnCallInfo::new(fun, args, span));

    many_to_vec(chained_fn, true, no_separator())
}

// endregion

// region token parsers

fn id<'a>(identifier: &str) -> impl TokenParser<'a, &'a Token<'a>> + use<'a, '_> {
    let p = token(TokenKind::Identifier).filter(move |t| t.text == identifier);
    attempt(p)
}

/// Returns a parser that matches a token with the specified kind.
fn token<'a>(kind: TokenKind) -> impl TokenParser<'a, &'a Token<'a>> {
    item_if(move |t: &'a Token<'a>| t.kind == kind)
}

fn keyword<'a>(id: &str) -> impl TokenParser<'a, &'a Token<'a>> + use<'a, '_> {
    let p = token(TokenKind::Keyword).filter(move |t| t.text == id);

    attempt(p)
}

// endregion

// region Expr parsers

fn some<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    yield_consumed(tuplify!(
        id("Some"),
        within(LeftParen, expression(), RightParen),
    ))
    .map(|(span, (_, expr))| Expr::OptionSome(Box::new(expr), span))
}

fn delimiter<'a>(kind: TokenKind) -> impl TokenParser<'a, Expr> {
    token(kind)
        .filter(move |t| t.kind == kind)
        .map(|t| Expr::Delimiter {
            symbol: t.text.chars().next().unwrap(),
            span: ExprSpan::new(t.span.0, t.span.1),
        })
}

fn arguments<'a>() -> impl TokenParser<'a, Vec<Expr>> {
    use TokenKind::*;

    let args_with_comma = or!(expression(), delimiter(Comma));
    many_to_vec(args_with_comma, true, no_separator()).map(sanitize_syntax)
}

fn tuple<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    #[allow(clippy::needless_question_mark)]
    yield_consumed(tuplify!(within(LeftParen, arguments(), RightParen),))
        .map(|(span, args)| Expr::Tuple(args, span))
}

fn sequence<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    let arr_effects = array().map_if(|a| match a {
        Expr::Array(args, _) => Some(args),
        Expr::ArrayRef(args, _) => Some(args),
        _ => Some(vec![]),
    });

    let effects = or!(arr_effects, succeed(item_if(|_| false)).map(|_| vec![]));

    yield_consumed(tuplify!(
        maybe_qualified("fx"),
        id("sequence"),
        within(LeftParen, effects, RightParen),
        method_chain(),
    ))
    .map(|(span, (_, _, args, self_fns))| Expr::Sequence { effects: args, self_fns, span })
}

fn parallel<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    let arr_effects = array().map_if(|a| match a {
        Expr::Array(args, _) => Some(args),
        Expr::ArrayRef(args, _) => Some(args),
        _ => Some(vec![]),
    });

    let effects = or!(arr_effects, succeed(item_if(|_| false)).map(|_| vec![]));

    yield_consumed(tuplify!(
        maybe_qualified("fx"),
        id("parallel"),
        within(LeftParen, effects, RightParen),
        method_chain(),
    ))
    .map(|(span, (_, _, args, self_fns))| Expr::Parallel { effects: args, self_fns, span })
}

fn boolean<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    yield_consumed(token(Keyword)).map_if(|(span, t): (_, &'a Token<'a>)| {
        Some(Expr::Literal(
            match t.text {
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                _ => None?,
            },
            span,
        ))
    })
}

fn literal<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    yield_consumed(item_if(|_| true)).map_if(|(span, t): (_, &'a Token<'a>)| {
        Some(Expr::Literal(
            match t.kind {
                FloatLiteral => Value::F32(t.text.parse().unwrap()),
                HexLiteral => Value::U32(u32::from_str_radix(&t.text[2..], 16).unwrap()),
                IntLiteral if t.text.starts_with("-") => Value::I32(t.text.parse().unwrap()),
                IntLiteral => Value::U32(t.text.parse().unwrap()),
                StringLiteral => Value::String(t.text.into()),
                _ => None?,
            },
            span,
        ))
    })
}

fn let_binding<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    yield_consumed(tuplify!(
        keyword("let"),
        identifier(),
        token(Equals),
        expression(),
    ))
    .map(|(span, (_, name, _, expr))| Expr::LetBinding {
        name: name.into(),
        let_expr: Box::new(expr),
        span,
    })
}

fn variable<'a>() -> impl TokenParser<'a, Expr> {
    yield_consumed(tuplify!(identifier(), method_chain())).map(|(span, (t, self_fns))| Expr::Var {
        name: t.into(),
        self_fns,
        span,
    })
}

fn qualified_name<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    yield_consumed(tuplify!(
        identifier(),
        token(DoubleColon),
        identifier(),
        method_chain()
    ))
    .map(
        |(span, (owner, _, member, self_fns))| Expr::QualifiedMember {
            name: format_compact!("{owner}::{member}"),
            self_fns,
            span,
        },
    )
}

fn function_expression<'a>() -> impl TokenParser<'a, Expr> {
    (tuplify!(function_call(), method_chain()))
        .map(|(call, self_fns)| Expr::FnCall { call, self_fns })
}

fn array<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    let nop = succeed(item_if(|_| false));

    yield_consumed(tuplify!(
        or!(token(Ampersand).map(|_| true), nop.map(|_| false)),
        within(LeftBracket, arguments(), RightBracket)
    ))
    .map(
        |(span, (is_ref, args))| {
            if is_ref {
                Expr::ArrayRef(args, span)
            } else {
                Expr::Array(args, span)
            }
        },
    )
}

fn struct_instantiation<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    // todo: support shorthand syntax for fields
    let field = (tuplify!(identifier(), token(Colon), expression(),))
        .map(|(name, _, value)| (name.to_compact_string(), value));

    let fields = many_to_vec(field, true, separator(token(Comma), true));

    yield_consumed(tuplify!(
        identifier(),
        within(LeftBrace, fields, RightBrace),
    ))
    .map(|(span, (name, fields))| Expr::StructInit { name: name.into(), fields, span })
}

fn macro_expr<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    yield_consumed(tuplify!(
        identifier(),
        token(Bang),
        within(LeftBracket, arguments(), RightBracket)
    ))
    .map(|(span, (name, _, args))| Expr::Macro { name: name.into(), args, span })
}

fn within<'a, T>(
    start: TokenKind,
    inner_parser: impl TokenParser<'a, T>,
    end: TokenKind,
) -> impl TokenParser<'a, T> {
    #[allow(clippy::needless_question_mark)]
    yield_consumed(tuplify!(middle(token(start), inner_parser, token(end)),))
        .map(move |(_, args)| args)
}

// endregion

fn sanitize_syntax(args: Vec<Expr>) -> Vec<Expr> {
    let (args, delimeters): (Vec<_>, Vec<_>) = args
        .into_iter()
        .enumerate()
        .partition(|(i, _)| i & 1 == 0);

    // ensure that args are delimited correctly
    let not_delimeted = delimeters
        .iter()
        .find(|(_, e)| !matches!(e, Expr::Delimiter { symbol: ',', .. }));

    // ensure that delimeters are in the correct place
    let wrong_place = args
        .iter()
        .find(|(_, e)| matches!(e, Expr::Delimiter { symbol: ',', .. }));

    // if there are no errors, return the arguments
    if not_delimeted.is_none() && wrong_place.is_none() {
        args.into_iter().map(|(_, e)| e).collect()
    } else {
        let unexpected_comma =
            |span| Expr::SyntaxError { message: "Unexpected comma delimiter".into(), span };

        let expected_comma =
            |span| Expr::SyntaxError { message: "Expected comma delimiter".into(), span };

        match (not_delimeted, wrong_place) {
            (Some((i, a)), Some((j, b))) => {
                if i < j {
                    vec![expected_comma(a.span())]
                } else {
                    vec![unexpected_comma(b.span())]
                }
            },
            (Some((_, e)), _) => vec![expected_comma(e.span())],
            (_, Some((_, e))) => vec![unexpected_comma(e.span())],
            _ => vec![], // unreachable
        }
    }
}

#[cfg(test)]
mod tests {
    use anpa::core::parse;
    use compact_str::ToCompactString;
    use ratatui_core::style::Color;

    use super::*;
    use crate::{
        dsl::tokenizer::{sanitize_tokens, tokenize},
        CellFilter,
    };

    // Helper function to create a Expr::FnCall expression
    fn expr_fn_call(name: &str, args: Vec<Expr>) -> Expr {
        Expr::FnCall {
            call: FnCallInfo::new(name, args, ExprSpan { start: 0, end: 0 }),
            self_fns: vec![],
        }
    }

    impl FnCallInfo {
        fn with_span(mut self, span: ExprSpan) -> Self {
            self.span = span;
            self
        }
    }

    impl Expr {
        /// Add chained methods to the expression
        fn with_self_fns(self, self_fns: Vec<FnCallInfo>) -> Self {
            match self {
                Expr::FnCall { call, .. } => Expr::FnCall { call, self_fns },
                _ => panic!("Expected FnCall expression"),
            }
        }

        fn with_span(self, start: u32, end: u32) -> Self {
            use Expr::*;

            let span = ExprSpan::new(start, end);
            match self {
                Literal(value, _) => Literal(value, span),
                Var { name, self_fns, .. } => Var { name, self_fns, span },
                LetBinding { name, let_expr, .. } => LetBinding { name, let_expr, span },
                ArrayRef(args, _) => ArrayRef(args, span),
                Array(args, _) => Array(args, span),
                FnCall { call, self_fns, .. } => FnCall { call: call.with_span(span), self_fns },
                QualifiedMember { name, self_fns, .. } => QualifiedMember { name, self_fns, span },
                OptionSome(expr, _) => OptionSome(expr, span),
                Sequence { effects, self_fns, .. } => Sequence { effects, self_fns, span },
                Parallel { effects, self_fns, .. } => Parallel { effects, self_fns, span },
                StructInit { name, fields, .. } => StructInit { name, fields, span },
                Tuple(exprs, _) => Tuple(exprs, span),
                Macro { name, args, .. } => Macro { name, args, span },
                Delimiter { symbol, .. } => Delimiter { symbol, span },
                SyntaxError { message, .. } => SyntaxError { message, span },
            }
        }
    }

    /// Helper function to create a FnCallInfo struct
    fn fn_info(name: &str, args: Vec<Expr>) -> FnCallInfo {
        FnCallInfo { name: name.into(), args, span: ExprSpan::new(0, 0) }
    }

    /// Helper function to run tests on tokenized input
    fn with_tokens(input: &str, f: impl FnOnce(&[Token])) {
        let tokens = tokenize(input).map(sanitize_tokens).unwrap();

        // println!("{:#?}", tokens);

        f(&tokens);
    }

    #[test]
    fn test_token_parser() {
        with_tokens("test", |tokens| {
            // Test matching token kind
            assert_eq!(
                parse(token(TokenKind::Identifier), tokens)
                    .result
                    .map(|t| t.text),
                Some("test")
            );

            // Test non-matching token kind
            assert_eq!(parse(token(TokenKind::StringLiteral), tokens).result, None);
        });
    }

    #[test]
    fn test_keyword_parser() {
        // Test with matching keyword
        with_tokens("let", |tokens| {
            assert!(parse(keyword("let"), tokens).result.is_some());
        });

        // Test with non-matching keyword
        with_tokens("const", |tokens| {
            assert_eq!(parse(keyword("let"), tokens).result, None);
        });

        // Test with non-keyword token
        with_tokens("variable", |tokens| {
            assert_eq!(parse(keyword("let"), tokens).result, None);
        });
    }

    #[test]
    fn test_identifier_parser() {
        // Test with a simple identifier
        with_tokens("variable_name", |tokens| {
            assert_eq!(parse(identifier(), tokens).result, Some("variable_name"));
        });

        // Test with a token that's not an identifier
        with_tokens("123", |tokens| {
            assert_eq!(parse(identifier(), tokens).result, None);
        });
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_literal_parser() {
        // Test with string literal
        with_tokens("\"hello world\"", |tokens| {
            assert_eq!(
                parse(literal(), tokens).result,
                Some(Expr::Literal(
                    Value::String("hello world".into()),
                    ExprSpan::new(0, 13)
                ))
            );
        });

        // Test with integer literal
        with_tokens("-42", |tokens| {
            assert_eq!(
                parse(literal(), tokens).result,
                Some(Expr::Literal(Value::I32(-42), ExprSpan::new(0, 3)))
            );
        });

        // Test with integer literal
        with_tokens("0x20", |tokens| {
            assert_eq!(
                parse(literal(), tokens).result,
                Some(Expr::Literal(Value::U32(32), ExprSpan::new(0, 4)))
            );
        });

        // Test with float literal
        with_tokens("3.14", |tokens| {
            assert_eq!(
                parse(literal(), tokens).result,
                Some(Expr::Literal(Value::F32(3.14), ExprSpan::new(0, 4)))
            );
        });

        // Test with non-literal token
        with_tokens("variable", |tokens| {
            assert_eq!(parse(literal(), tokens).result, None);
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
                    self_fns: vec![],
                    span: ExprSpan::new(0, 6)
                })
            );
        });

        // Test with non-identifier token
        with_tokens("123", |tokens| {
            assert_eq!(parse(variable(), tokens).result, None);
        });
    }

    #[test]
    fn test_qualified_member_parser() {
        // Test with valid qualified member
        with_tokens("Color::Red", |tokens| {
            assert_eq!(
                parse(qualified_name(), tokens).result,
                Some(Expr::QualifiedMember {
                    name: "Color::Red".into(),
                    self_fns: vec![],
                    span: ExprSpan::new(0, 10)
                })
            );
        });

        // Test with unqualified member
        with_tokens("Red", |tokens| {
            assert_eq!(parse(qualified_name(), tokens).result, None);
        });

        // Test with invalid format
        with_tokens("Color.Red", |tokens| {
            assert_eq!(parse(qualified_name(), tokens).result, None);
        });
    }

    #[test]
    fn test_argument_parser() {
        // Test with literal
        with_tokens("42", |tokens| {
            assert_eq!(
                parse(expression(), tokens).result,
                Some(Expr::Literal(Value::U32(42), ExprSpan::new(0, 2)))
            );
        });

        // Test with variable
        with_tokens("my_var", |tokens| {
            assert_eq!(
                parse(expression(), tokens).result,
                Some(Expr::Var {
                    name: "my_var".into(),
                    self_fns: vec![],
                    span: ExprSpan::new(0, 6)
                })
            );
        });

        // Test with qualified member
        with_tokens("Color::Red", |tokens| {
            assert_eq!(
                parse(expression(), tokens).result,
                Some(Expr::Literal(
                    Value::Color(Color::Red),
                    ExprSpan::new(0, 10)
                ))
            );
        });

        // Test with function call
        with_tokens("fx::fade_to()", |tokens| {
            assert_eq!(
                parse(expression(), tokens).result,
                Some(expr_fn_call("fx::fade_to", vec![]).with_span(0, 13))
            );
        });
    }

    #[test]
    fn test_arguments_parser() {
        // Test with empty arguments
        with_tokens("", |tokens| {
            assert_eq!(parse(arguments(), tokens).result, Some(vec![]));
        });

        // Test with single argument
        with_tokens("42", |tokens| {
            assert_eq!(
                parse(arguments(), tokens).result,
                Some(vec![Expr::Literal(Value::U32(42), ExprSpan::new(0, 2))])
            );
        });

        // Test with multiple arguments
        with_tokens("42, \"hello\", my_var", |tokens| {
            assert_eq!(
                parse(arguments(), tokens).result,
                Some(vec![
                    Expr::Literal(Value::U32(42), ExprSpan::new(0, 2)),
                    Expr::Literal(Value::String("hello".into()), ExprSpan::new(4, 11)),
                    Expr::Var {
                        name: "my_var".into(),
                        self_fns: vec![],
                        span: ExprSpan::new(13, 19)
                    }
                ])
            );
        });

        // Test with complex arguments including qualified members
        with_tokens("Color::Red, 500, fx::dissolve(200)", |tokens| {
            assert_eq!(
                parse(arguments(), tokens).result,
                Some(vec![
                    Expr::Literal(Value::Color(Color::Red), ExprSpan::new(0, 10)),
                    Expr::Literal(Value::U32(500), ExprSpan::new(12, 15)),
                    expr_fn_call("fx::dissolve", vec![Expr::Literal(
                        Value::U32(200),
                        ExprSpan::new(30, 33)
                    )])
                    .with_span(17, 34)
                ])
            );
        });
    }

    #[test]
    fn test_let_expr_parser() {
        // Test basic let binding
        with_tokens("let x = 42;", |tokens| {
            assert_eq!(
                parse(let_binding(), tokens).result,
                Some(Expr::LetBinding {
                    name: "x".into(),
                    let_expr: Box::new(Expr::Literal(Value::U32(42), ExprSpan::new(8, 10))),
                    span: ExprSpan::new(0, 10)
                })
            );
        });

        // Test let binding with complex expression
        with_tokens("let color = Color::Red;", |tokens| {
            assert_eq!(
                parse(let_binding(), tokens).result,
                Some(Expr::LetBinding {
                    name: "color".into(),
                    let_expr: Box::new(Expr::Literal(
                        Value::Color(Color::Red),
                        ExprSpan::new(12, 22)
                    )),
                    span: ExprSpan::new(0, 22)
                })
            );
        });

        // Test let binding with function call
        with_tokens("let effect = fx::fade_to(Color::Red, 500);", |tokens| {
            assert_eq!(
                parse(let_binding(), tokens).result,
                Some(Expr::LetBinding {
                    name: "effect".into(),
                    let_expr: Box::new(
                        expr_fn_call("fx::fade_to", vec![
                            Expr::Literal(Value::Color(Color::Red), ExprSpan::new(25, 35)),
                            Expr::Literal(Value::U32(500), ExprSpan::new(37, 40))
                        ])
                        .with_span(13, 41)
                    ),
                    span: ExprSpan::new(0, 41)
                })
            );
        });
    }

    #[test]
    fn test_fn_call_parser() {
        // Test with no arguments
        with_tokens("fx::fade_to()", |tokens| {
            assert_eq!(
                parse(function_call(), tokens).result,
                Some(FnCallInfo {
                    name: "fx::fade_to".into(),
                    args: vec![],
                    span: ExprSpan::new(0, 13)
                })
            );
        });

        // Test with arguments
        with_tokens("fx::fade_to(42, \"hello\", my_var)", |tokens| {
            assert_eq!(
                parse(function_call(), tokens).result,
                Some(
                    fn_info("fx::fade_to", vec![
                        Expr::Literal(Value::U32(42), ExprSpan::new(12, 14)),
                        Expr::Literal(Value::String("hello".into()), ExprSpan::new(16, 23)),
                        Expr::Var {
                            name: "my_var".into(),
                            self_fns: vec![],
                            span: ExprSpan::new(25, 31)
                        }
                    ])
                    .with_span(ExprSpan::new(0, 32))
                )
            );
        });

        // Test with qualified member in arguments
        with_tokens("fx::fade_to(Color::Red, 500)", |tokens| {
            assert_eq!(
                parse(function_call(), tokens).result,
                Some(
                    fn_info("fx::fade_to", vec![
                        Expr::Literal(Value::Color(Color::Red), ExprSpan::new(12, 22)),
                        Expr::Literal(Value::U32(500), ExprSpan::new(24, 27))
                    ])
                    .with_span(ExprSpan::new(0, 28))
                )
            );
        });

        // Test with nested function calls
        with_tokens(
            "fx::sequence(fx::dissolve(200), fx::fade_to(Color::Red, 300))",
            |tokens| {
                assert_eq!(
                    parse(function_call(), tokens).result,
                    Some(
                        fn_info("fx::sequence", vec![
                            expr_fn_call("fx::dissolve", vec![Expr::Literal(
                                Value::U32(200),
                                ExprSpan::new(26, 29)
                            )])
                            .with_span(13, 30),
                            expr_fn_call("fx::fade_to", vec![
                                Expr::Literal(Value::Color(Color::Red), ExprSpan::new(44, 54)),
                                Expr::Literal(Value::U32(300), ExprSpan::new(56, 59))
                            ])
                            .with_span(32, 60)
                        ])
                        .with_span(ExprSpan::new(0, 61))
                    )
                );
            },
        );

        with_tokens("fade_to()", |tokens| {
            assert_eq!(
                parse(function_call(), tokens).result,
                Some(fn_info("fade_to", vec![]).with_span(ExprSpan::new(0, 9)))
            );
        });
    }

    #[test]
    fn test_chained_fns_parser() {
        // Test with no chained methods
        with_tokens("", |tokens| {
            assert_eq!(parse(method_chain(), tokens).result, Some(vec![]));
        });

        // Test with single chained method
        with_tokens(".filter(CellFilter::Text)", |tokens| {
            assert_eq!(
                parse(method_chain(), tokens).result,
                Some(vec![FnCallInfo {
                    name: "filter".into(),
                    args: vec![Expr::Literal(
                        Value::CellFilter(CellFilter::Text),
                        ExprSpan::new(8, 24)
                    )],
                    span: ExprSpan::new(0, 25)
                }])
            );
        });

        // Test with multiple chained methods
        with_tokens(
            ".filter(CellFilter::Text).with_area(Rect::new(0, 0, 10, 10))",
            |tokens| {
                assert_eq!(
                    parse(method_chain(), tokens).result,
                    Some(vec![
                        fn_info("filter", vec![Expr::Literal(
                            Value::CellFilter(CellFilter::Text),
                            ExprSpan::new(8, 24)
                        )])
                        .with_span(ExprSpan::new(0, 25)),
                        fn_info("with_area", vec![expr_fn_call("Rect::new", vec![
                            Expr::Literal(Value::U32(0), ExprSpan::new(46, 47)),
                            Expr::Literal(Value::U32(0), ExprSpan::new(49, 50)),
                            Expr::Literal(Value::U32(10), ExprSpan::new(52, 54)),
                            Expr::Literal(Value::U32(10), ExprSpan::new(56, 58)),
                        ])
                        .with_span(36, 59)])
                        .with_span(ExprSpan::new(25, 60))
                    ])
                );
            },
        );
    }

    #[test]
    fn test_fn_call_expr_parser() {
        // Test function call with method chaining
        with_tokens(
            "fx::fade_to(Color::Red, 500).filter(CellFilter::Text)",
            |tokens| {
                assert_eq!(
                    parse(function_expression(), tokens).result,
                    Some(
                        expr_fn_call("fx::fade_to", vec![
                            Expr::Literal(Value::Color(Color::Red), ExprSpan::new(12, 22)),
                            Expr::Literal(Value::U32(500), ExprSpan::new(24, 27))
                        ])
                        .with_span(0, 53)
                        .with_self_fns(vec![fn_info("filter", vec![Expr::Literal(
                            Value::CellFilter(CellFilter::Text),
                            ExprSpan::new(36, 52)
                        )])
                        .with_span(ExprSpan::new(28, 53))])
                        .with_span(0, 28)
                    )
                );
            },
        );

        // Test function call with multiple method chains
        with_tokens(
            "fx::dissolve(200).filter(CellFilter::Text).with_area(Rect::new(0, 0, 10, 10))",
            |tokens| {
                assert_eq!(
                    parse(function_expression(), tokens).result,
                    Some(
                        expr_fn_call("fx::dissolve", vec![Expr::Literal(
                            Value::U32(200),
                            ExprSpan::new(13, 16)
                        )])
                        .with_self_fns(vec![
                            fn_info("filter", vec![Expr::Literal(
                                Value::CellFilter(CellFilter::Text),
                                ExprSpan::new(25, 41)
                            )])
                            .with_span(ExprSpan::new(17, 42)),
                            fn_info("with_area", vec![expr_fn_call("Rect::new", vec![
                                Expr::Literal(Value::U32(0), ExprSpan::new(63, 64)),
                                Expr::Literal(Value::U32(0), ExprSpan::new(66, 67)),
                                Expr::Literal(Value::U32(10), ExprSpan::new(69, 71)),
                                Expr::Literal(Value::U32(10), ExprSpan::new(73, 75))
                            ])
                            .with_span(53, 76)])
                            .with_span(ExprSpan::new(42, 77))
                        ])
                        .with_span(0, 17)
                    )
                );
            },
        );

        // Test nested function calls with method chaining
        with_tokens("fx::sequence(fx::dissolve(200).reversed(), fx::fade_to(Color::Red, 300)).filter(CellFilter::Text)", |tokens| {
            assert_eq!(
                parse(function_expression(), tokens).result,
                Some(
                    expr_fn_call("fx::sequence", vec![
                        expr_fn_call("fx::dissolve", vec![
                            Expr::Literal(Value::U32(200), ExprSpan::new(26, 29))
                        ]).with_self_fns(vec![fn_info("reversed", vec![]).with_span(ExprSpan::new(30, 41))]).with_span(13, 30),
                        expr_fn_call("fx::fade_to", vec![
                            Expr::Literal(Value::Color(Color::Red), ExprSpan::new(55, 65)),
                            Expr::Literal(Value::U32(300), ExprSpan::new(67, 70))
                        ]).with_span(43, 71)
                    ]).with_span(0, 72).with_self_fns(vec![
                        fn_info("filter", vec![
                            Expr::Literal(Value::CellFilter(CellFilter::Text), ExprSpan::new(80, 96))
                        ]).with_span(ExprSpan::new(72, 97))
                    ])
                )
            );
        });
    }

    #[test]
    fn test_array_parser() {
        // Test empty array
        with_tokens("[]", |tokens| {
            assert_eq!(
                parse(array(), tokens).result,
                Some(Expr::Array(vec![], ExprSpan::new(0, 2)))
            );
        });

        // Test array with single element
        with_tokens("[42]", |tokens| {
            assert_eq!(
                parse(array(), tokens).result,
                Some(Expr::Array(
                    vec![Expr::Literal(Value::U32(42), ExprSpan::new(1, 3))],
                    ExprSpan::new(0, 4)
                ))
            );
        });

        // Test array with multiple elements
        with_tokens("[42, \"hello\", Color::Red]", |tokens| {
            assert_eq!(
                parse(array(), tokens).result,
                Some(Expr::Array(
                    vec![
                        Expr::Literal(Value::U32(42), ExprSpan::new(1, 3)),
                        Expr::Literal(Value::String("hello".into()), ExprSpan::new(5, 12)),
                        Expr::Literal(Value::Color(Color::Red), ExprSpan::new(14, 24))
                    ],
                    ExprSpan::new(0, 25)
                ))
            );
        });

        // Test array with function calls
        with_tokens(
            "[fx::dissolve(200), fx::fade_to(Color::Red, 300)]",
            |tokens| {
                assert_eq!(
                    parse(array(), tokens).result,
                    Some(Expr::Array(
                        vec![
                            expr_fn_call("fx::dissolve", vec![Expr::Literal(
                                Value::U32(200),
                                ExprSpan::new(14, 17)
                            )])
                            .with_span(1, 18),
                            expr_fn_call("fx::fade_to", vec![
                                Expr::Literal(Value::Color(Color::Red), ExprSpan::new(32, 42)),
                                Expr::Literal(Value::U32(300), ExprSpan::new(44, 47))
                            ])
                            .with_span(20, 48)
                        ],
                        ExprSpan::new(0, 49)
                    ))
                );
            },
        );
    }

    #[test]
    fn test_array_ref_parser() {
        // Test empty array reference
        with_tokens("&[]", |tokens| {
            assert_eq!(
                parse(array(), tokens).result,
                Some(Expr::ArrayRef(vec![], ExprSpan::new(0, 3)))
            );
        });

        // Test array reference with single element
        with_tokens("&[42]", |tokens| {
            assert_eq!(
                parse(array(), tokens).result,
                Some(Expr::ArrayRef(
                    vec![Expr::Literal(Value::U32(42), ExprSpan::new(2, 4))],
                    ExprSpan::new(0, 5)
                ))
            );
        });

        // Test array reference with multiple elements
        with_tokens("&[42, \"hello\", Color::Red]", |tokens| {
            assert_eq!(
                parse(array(), tokens).result,
                Some(Expr::ArrayRef(
                    vec![
                        Expr::Literal(Value::U32(42), ExprSpan::new(2, 4)),
                        Expr::Literal(Value::String("hello".into()), ExprSpan::new(6, 13)),
                        Expr::Literal(Value::Color(Color::Red), ExprSpan::new(15, 25))
                    ],
                    ExprSpan::new(0, 26)
                ))
            );
        });

        // Test array reference with function calls
        with_tokens(
            "&[fx::dissolve(200), fx::fade_to(Color::Red, 300)]",
            |tokens| {
                assert_eq!(
                    parse(array(), tokens).result,
                    Some(Expr::ArrayRef(
                        vec![
                            expr_fn_call("fx::dissolve", vec![Expr::Literal(
                                Value::U32(200),
                                ExprSpan::new(15, 18)
                            )])
                            .with_span(2, 19),
                            expr_fn_call("fx::fade_to", vec![
                                Expr::Literal(Value::Color(Color::Red), ExprSpan::new(33, 43)),
                                Expr::Literal(Value::U32(300), ExprSpan::new(45, 48))
                            ])
                            .with_span(21, 49)
                        ],
                        ExprSpan::new(0, 50)
                    ))
                );
            },
        );
    }

    #[test]
    fn test_complex_method_chaining() {
        // Test complex method chaining with nested function calls
        with_tokens(
            "fx::sequence(&[fx::dissolve(200), fx::fade_to(Color::Red, 300)])
                    .filter(CellFilter::Text)
                    .with_area(Rect::new(0, 0, 10, 10).offset(5, 5))",
            |tokens| {
                let anpa_result = parse(function_expression(), tokens);
                assert_eq!(anpa_result.state, &[]);
                let result = anpa_result.result;

                let expr = result.unwrap();
                if let Expr::FnCall { call, self_fns, .. } = expr {
                    assert_eq!(call.name, "fx::sequence".to_compact_string());
                    assert_eq!(self_fns.len(), 2);
                    assert_eq!(self_fns[0].name, "filter".to_compact_string());
                    assert_eq!(self_fns[1].name, "with_area".to_compact_string());

                    // Check the nested Rect with its own method chain
                    if let Expr::FnCall { call: rect_call, self_fns: rect_fns, .. } =
                        &self_fns[1].args[0]
                    {
                        assert_eq!(rect_call.name, "Rect::new".to_compact_string());
                        assert_eq!(rect_fns.len(), 1);
                        assert_eq!(rect_fns[0].name, "offset".to_compact_string());
                    }
                } else {
                    panic!("Expected FnCall expression");
                }
            },
        );

        // Test method chaining with different types of methods
        with_tokens(
            "fx::fade_to(Color::Red, 500)
                    .filter(CellFilter::Text)
                    .with_area(Rect::new(0, 0, 10, 10))
                    .reversed()
                    .clone()",
            |tokens| {
                let result = parse(function_expression(), tokens).result;
                assert!(result.is_some());

                let expr = result.unwrap();
                if let Expr::FnCall { call, self_fns, .. } = expr {
                    assert_eq!(call.name, "fx::fade_to".to_compact_string());
                    assert_eq!(self_fns.len(), 4);
                    assert_eq!(self_fns[0].name, "filter".to_compact_string());
                    assert_eq!(self_fns[1].name, "with_area".to_compact_string());
                    assert_eq!(self_fns[2].name, "reversed".to_compact_string());
                    assert_eq!(self_fns[3].name, "clone".to_compact_string());

                    // Verify the no-args methods are empty
                    assert_eq!(self_fns[2].args.len(), 0);
                    assert_eq!(self_fns[3].args.len(), 0);
                } else {
                    panic!("Expected FnCall expression");
                }
            },
        );
    }

    #[test]
    fn test_maybe_qualified_parser() {
        // Test with qualified identifier
        with_tokens("fx::", |tokens| {
            assert_eq!(parse(maybe_qualified("fx"), tokens).result, Some(()));
        });

        // Test with non-matching qualified identifier; should always succeed
        with_tokens("color::", |tokens| {
            assert_eq!(parse(maybe_qualified("fx"), tokens).result, Some(()));
        });
    }

    #[test]
    fn test_some_parser() {
        // Test with a simple value
        with_tokens("Some(42)", |tokens| {
            assert_eq!(
                parse(some(), tokens).result,
                Some(Expr::OptionSome(
                    Box::new(Expr::Literal(Value::U32(42), ExprSpan::new(5, 7))),
                    ExprSpan::new(0, 8)
                ))
            );
        });

        // Test with a complex expression
        with_tokens("Some(Color::Red)", |tokens| {
            assert_eq!(
                parse(some(), tokens).result,
                Some(Expr::OptionSome(
                    Box::new(Expr::Literal(
                        Value::Color(Color::Red),
                        ExprSpan::new(5, 15)
                    )),
                    ExprSpan::new(0, 16)
                ))
            );
        });

        // Test with a nested function call
        with_tokens("Some(fx::dissolve(200))", |tokens| {
            assert_eq!(
                parse(some(), tokens).result,
                Some(Expr::OptionSome(
                    Box::new(
                        expr_fn_call("fx::dissolve", vec![Expr::Literal(
                            Value::U32(200),
                            ExprSpan::new(18, 21)
                        )])
                        .with_span(5, 22)
                    ),
                    ExprSpan::new(0, 23)
                ))
            );
        });
    }

    #[test]
    fn test_sequence_parser() {
        // Test with empty sequence
        with_tokens("fx::sequence()", |tokens| {
            assert_eq!(
                parse(sequence(), tokens).result,
                Some(Expr::Sequence {
                    effects: vec![],
                    self_fns: vec![],
                    span: ExprSpan::new(0, 14)
                })
            );
        });

        // Test with single effect
        with_tokens("fx::sequence(&[fx::dissolve(200)])", |tokens| {
            assert_eq!(
                parse(sequence(), tokens).result,
                Some(Expr::Sequence {
                    effects: vec![expr_fn_call("fx::dissolve", vec![Expr::Literal(
                        Value::U32(200),
                        ExprSpan::new(28, 31)
                    )])
                    .with_span(15, 32)],
                    self_fns: vec![],
                    span: ExprSpan::new(0, 34)
                })
            );
        });

        // Test with multiple effects
        with_tokens(
            "fx::sequence(&[dissolve(200), fx::fade_to(Color::Red, 300)])",
            |tokens| {
                assert_eq!(
                    parse(sequence(), tokens).result,
                    Some(Expr::Sequence {
                        effects: vec![
                            expr_fn_call("dissolve", vec![Expr::Literal(
                                Value::U32(200),
                                ExprSpan::new(24, 27)
                            )])
                            .with_span(15, 28),
                            expr_fn_call("fx::fade_to", vec![
                                Expr::Literal(Value::Color(Color::Red), ExprSpan::new(42, 52)),
                                Expr::Literal(Value::U32(300), ExprSpan::new(54, 57))
                            ])
                            .with_span(30, 58)
                        ],
                        self_fns: vec![],
                        span: ExprSpan::new(0, 60)
                    })
                );
            },
        );

        // Test with method chaining
        with_tokens(
            "fx::sequence(&[fx::dissolve(200)]).filter(CellFilter::Text)",
            |tokens| {
                assert_eq!(
                    parse(sequence(), tokens).result,
                    Some(Expr::Sequence {
                        effects: vec![expr_fn_call("fx::dissolve", vec![Expr::Literal(
                            Value::U32(200),
                            ExprSpan::new(28, 31)
                        )])
                        .with_span(15, 32),],
                        self_fns: vec![fn_info("filter", vec![Expr::Literal(
                            Value::CellFilter(CellFilter::Text),
                            ExprSpan::new(42, 58)
                        )])
                        .with_span(ExprSpan::new(34, 59))],
                        span: ExprSpan::new(0, 59)
                    })
                );
            },
        );

        // Test with array reference
        with_tokens(
            "sequence(&[fx::dissolve(200), fx::fade_to(Color::Red, 300)])",
            |tokens| {
                assert_eq!(
                    parse(sequence(), tokens).result,
                    Some(Expr::Sequence {
                        effects: vec![
                            expr_fn_call("fx::dissolve", vec![Expr::Literal(
                                Value::U32(200),
                                ExprSpan::new(24, 27)
                            )])
                            .with_span(11, 28),
                            expr_fn_call("fx::fade_to", vec![
                                Expr::Literal(Value::Color(Color::Red), ExprSpan::new(42, 52)),
                                Expr::Literal(Value::U32(300), ExprSpan::new(54, 57))
                            ])
                            .with_span(30, 58)
                        ],
                        self_fns: vec![],
                        span: ExprSpan::new(0, 60)
                    })
                );
            },
        );
    }

    #[test]
    fn test_parallel_parser() {
        // Test with empty parallel
        with_tokens("fx::parallel()", |tokens| {
            assert_eq!(
                parse(parallel(), tokens).result,
                Some(Expr::Parallel {
                    effects: vec![],
                    self_fns: vec![],
                    span: ExprSpan::new(0, 14)
                })
            );
        });

        // Test with array reference
        with_tokens(
            "parallel(&[fx::dissolve(200), fx::fade_to(Color::Red, 300)])",
            |tokens| {
                assert_eq!(
                    parse(parallel(), tokens).result,
                    Some(Expr::Parallel {
                        effects: vec![
                            expr_fn_call("fx::dissolve", vec![Expr::Literal(
                                Value::U32(200),
                                ExprSpan::new(24, 27)
                            )])
                            .with_span(11, 28),
                            expr_fn_call("fx::fade_to", vec![
                                Expr::Literal(Value::Color(Color::Red), ExprSpan::new(42, 52)),
                                Expr::Literal(Value::U32(300), ExprSpan::new(54, 57))
                            ])
                            .with_span(30, 58)
                        ],
                        self_fns: vec![],
                        span: ExprSpan::new(0, 60)
                    })
                );
            },
        );
    }

    #[test]
    fn test_id_parser() {
        // Test with matching identifier
        with_tokens("test", |tokens| {
            assert!(parse(id("test"), tokens).result.is_some());
        });

        // Test with non-matching identifier
        with_tokens("other", |tokens| {
            assert_eq!(parse(id("test"), tokens).result, None);
        });

        // Test with non-identifier token
        with_tokens("123", |tokens| {
            assert_eq!(parse(id("test"), tokens).result, None);
        });
    }

    #[test]
    fn test_expression_integration() {
        // Test Some option via main expression parser
        with_tokens("Some(fx::dissolve(200))", |tokens| {
            let result = parse(expression(), tokens).result;
            assert!(result.is_some());

            match result.unwrap() {
                Expr::OptionSome(expr, _) => {
                    assert_eq!(
                        expr,
                        Box::new(
                            expr_fn_call("fx::dissolve", vec![Expr::Literal(
                                Value::U32(200),
                                ExprSpan::new(18, 21)
                            )])
                            .with_span(5, 22)
                        )
                    );
                },
                _ => panic!("Expected OptionSome expression"),
            }
        });
    }

    #[test]
    fn test_struct_instantiation_parser() {
        // Test with empty struct
        with_tokens("Point {}", |tokens| {
            let result = parse(struct_instantiation(), tokens).result;
            assert_eq!(
                result,
                Some(Expr::StructInit {
                    name: "Point".to_compact_string(),
                    fields: vec![],
                    span: ExprSpan::new(0, 8)
                })
            );
        });

        // Test with single field
        with_tokens("Point { x: 10 }", |tokens| {
            let result = parse(struct_instantiation(), tokens).result;
            assert_eq!(
                result,
                Some(Expr::StructInit {
                    name: "Point".to_compact_string(),
                    fields: vec![(
                        "x".to_compact_string(),
                        Expr::Literal(Value::U32(10), ExprSpan::new(11, 13))
                    )],
                    span: ExprSpan::new(0, 15)
                })
            );
        });

        // Test with multiple fields
        with_tokens("Rectangle { width: 100, height: 200 }", |tokens| {
            let result = parse(struct_instantiation(), tokens).result;
            assert_eq!(
                result,
                Some(Expr::StructInit {
                    name: "Rectangle".to_compact_string(),
                    fields: vec![
                        (
                            "width".to_compact_string(),
                            Expr::Literal(Value::U32(100), ExprSpan::new(19, 22))
                        ),
                        (
                            "height".to_compact_string(),
                            Expr::Literal(Value::U32(200), ExprSpan::new(32, 35))
                        )
                    ],
                    span: ExprSpan::new(0, 37)
                })
            );
        });

        // Test with complex field values
        with_tokens("Color { r: 255, g: 128, b: 64 }", |tokens| {
            let result = parse(struct_instantiation(), tokens).result;
            assert_eq!(
                result,
                Some(Expr::StructInit {
                    name: "Color".to_compact_string(),
                    fields: vec![
                        (
                            "r".to_compact_string(),
                            Expr::Literal(Value::U32(255), ExprSpan::new(11, 14))
                        ),
                        (
                            "g".to_compact_string(),
                            Expr::Literal(Value::U32(128), ExprSpan::new(19, 22))
                        ),
                        (
                            "b".to_compact_string(),
                            Expr::Literal(Value::U32(64), ExprSpan::new(27, 29))
                        )
                    ],
                    span: ExprSpan::new(0, 31)
                })
            );
        });

        // Test with nested structs
        with_tokens(
            "Outer { inner: Inner { value: 42 }, name: \"test\" }",
            |tokens| {
                let result = parse(struct_instantiation(), tokens).result;

                if let Some(Expr::StructInit { name, fields, .. }) = result {
                    assert_eq!(name, "Outer");
                    assert_eq!(fields.len(), 2);
                    assert_eq!(fields[0].0, "inner");

                    // Check nested struct
                    if let Expr::StructInit { name, fields, .. } = &fields[0].1 {
                        assert_eq!(name, "Inner");
                        assert_eq!(fields.len(), 1);
                        assert_eq!(fields[0].0, "value");
                        assert_eq!(
                            fields[0].1,
                            Expr::Literal(Value::U32(42), ExprSpan::new(30, 32))
                        );
                    } else {
                        panic!("Expected nested StructInit expression");
                    }

                    assert_eq!(fields[1].0, "name");
                    assert_eq!(
                        fields[1].1,
                        Expr::Literal(
                            Value::String("test".to_compact_string()),
                            ExprSpan::new(42, 48)
                        )
                    );
                } else {
                    panic!("Expected StructInit expression");
                }
            },
        );

        // Test with variable references in fields
        with_tokens("Point { x: x_var, y: y_var }", |tokens| {
            let result = parse(struct_instantiation(), tokens).result;
            assert_eq!(
                result,
                Some(Expr::StructInit {
                    name: "Point".to_compact_string(),
                    fields: vec![
                        ("x".to_compact_string(), Expr::Var {
                            name: "x_var".to_compact_string(),
                            self_fns: vec![],
                            span: ExprSpan::new(11, 16)
                        }),
                        ("y".to_compact_string(), Expr::Var {
                            name: "y_var".to_compact_string(),
                            self_fns: vec![],
                            span: ExprSpan::new(21, 26)
                        })
                    ],
                    span: ExprSpan::new(0, 28)
                })
            );
        });

        // Test with function calls in fields
        with_tokens(
            "Config { color: Color::new(255, 0, 0), size: calculate_size() }",
            |tokens| {
                let result = parse(struct_instantiation(), tokens).result;

                if let Some(Expr::StructInit { name, fields, .. }) = result {
                    assert_eq!(name, "Config");
                    assert_eq!(fields.len(), 2);
                    assert_eq!(fields[0].0, "color");

                    if let Expr::FnCall { call, .. } = &fields[0].1 {
                        assert_eq!(call.name, "Color::new");
                        assert_eq!(call.args.len(), 3);
                    } else {
                        panic!("Expected function call");
                    }

                    assert_eq!(fields[1].0, "size");
                    if let Expr::FnCall { call, .. } = &fields[1].1 {
                        assert_eq!(call.name, "calculate_size");
                        assert_eq!(call.args.len(), 0);
                    } else {
                        panic!("Expected function call");
                    }
                } else {
                    panic!("Expected StructInit expression");
                }
            },
        );

        // Test with trailing comma
        with_tokens("Point { x: 10, y: 20, }", |tokens| {
            let result = parse(struct_instantiation(), tokens).result;
            assert_eq!(
                result,
                Some(Expr::StructInit {
                    name: "Point".to_compact_string(),
                    fields: vec![
                        (
                            "x".to_compact_string(),
                            Expr::Literal(Value::U32(10), ExprSpan::new(11, 13))
                        ),
                        (
                            "y".to_compact_string(),
                            Expr::Literal(Value::U32(20), ExprSpan::new(18, 20))
                        )
                    ],
                    span: ExprSpan::new(0, 23)
                })
            );
        });
    }

    #[test]
    fn test_macro_expression() {
        // Test vec macro with simple values
        with_tokens("vec![1, 2, 3]", |tokens| {
            assert_eq!(
                parse(macro_expr(), tokens).result,
                Some(Expr::Macro {
                    name: "vec".into(),
                    args: vec![
                        Expr::Literal(Value::U32(1), ExprSpan::new(5, 6)),
                        Expr::Literal(Value::U32(2), ExprSpan::new(8, 9)),
                        Expr::Literal(Value::U32(3), ExprSpan::new(11, 12))
                    ],
                    span: ExprSpan::new(0, 13)
                })
            );
        });

        // Test vec macro with CellFilter values - this is specifically for the use case we're
        // implementing
        with_tokens(
            "vec![CellFilter::Text, CellFilter::Inner(Margin::new(1, 1))]",
            |tokens| {
                let result = parse(macro_expr(), tokens).result;
                assert!(result.is_some());
                if let Some(Expr::Macro { name, args, span }) = result {
                    assert_eq!(name, "vec");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(
                        args[0],
                        Expr::Literal(Value::CellFilter(CellFilter::Text), _)
                    ));
                    if let Expr::FnCall { call, .. } = &args[1] {
                        assert_eq!(call.name, "CellFilter::Inner");
                        assert_eq!(call.args.len(), 1);
                    } else {
                        panic!("Expected FnCall expression for the second argument");
                    }
                    assert!(span.start < span.end);
                } else {
                    panic!("Expected Macro expression");
                }
            },
        );
    }
}
