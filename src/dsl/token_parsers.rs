use crate::dsl::expressions::Value;
use crate::dsl::tokenizer::{Token, TokenKind};
use anpa::combinators::{attempt, many_to_vec, middle, no_separator, separator, succeed};
use anpa::core::ParserExt;
use anpa::parsers::item_if;
use anpa::{create_parser_trait, or, right, tuplify};
use compact_str::{format_compact, CompactString};

create_parser_trait!(TokenParser, [Token<'a>], "effect dsl token parser");

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Expr {
    Literal(Value),
    Var { name: CompactString, self_fns: Vec<FnCallInfo> },
    LetBinding {
        name: CompactString,
        let_expr: Box<Expr>,
    },
    ArrayRef(Vec<Expr>),
    Array(Vec<Expr>),
    FnCall { call: FnCallInfo, self_fns: Vec<FnCallInfo> },
    QualifiedMember(CompactString), // enums, struct fields
    OptionSome(Box<Expr>),
    Sequence {
        effects: Vec<Expr>,
        self_fns: Vec<FnCallInfo>
    },
    Parallel {
        effects: Vec<Expr>,
        self_fns: Vec<FnCallInfo>
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct FnCallInfo {
    pub name: CompactString,
    pub args: Vec<Expr>,
}

impl FnCallInfo {
    pub fn new(
        name: impl Into<CompactString>,
        args: Vec<Expr>
    ) -> Self {
        Self { name: name.into(), args }
    }
}

// main parser //
fn expression<'a>() -> impl TokenParser<'a, Expr> {
    or!(
        literal(),
        let_binding(),
        sequence(),
        parallel(),
        some(),
        function_expression(),
        array(),
        array_reference(),
        qualified_name(),
        variable(),
    )
}

fn some<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    tuplify!(
        id("Some"),
        middle(token(LeftParen), expression(), token(RightParen)),
    ).map(|(_, expr)| Expr::OptionSome(Box::new(expr)))
}

fn arguments<'a>() -> impl TokenParser<'a, Vec<Expr>> {
    use TokenKind::*;

    many_to_vec(expression(), true, separator(token(Comma), true))
}

/// Returns a parser that matches a token with the specified kind.
fn token<'a>(kind: TokenKind) -> impl TokenParser<'a, &'a Token<'a>> {
    item_if(move |t: &'a Token<'a>| t.kind == kind)
}

fn keyword<'a>(id: &str) -> impl TokenParser<'a, &'a Token<'a>> + use<'a, '_> {
    let p = token(TokenKind::Keyword)
        .filter(move |t| t.text == id);

    attempt(p)
}

fn maybe_qualified<'a>(owner: &str) ->impl TokenParser<'a, ()> + use<'a, '_> {
    use TokenKind::*;

    succeed(attempt(right!(id(owner), token(DoubleColon))))
        .map(|_| ())
}

fn sequence<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    tuplify!(
        maybe_qualified("fx"),
        id("sequence"),
        middle(token(LeftParen), arguments(), token(RightParen)),
        method_chain(),
    ).map(|(_, _, args, self_fns)| Expr::Sequence { effects: args, self_fns })
}

fn parallel<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    tuplify!(
        maybe_qualified("fx"),
        id("parallel"),
        middle(token(LeftParen), arguments(), token(RightParen)),
        method_chain(),
    ).map(|(_, _, args, self_fns)| Expr::Parallel { effects: args, self_fns })
}

fn identifier<'a>() -> impl TokenParser<'a, &'a str> {
    token(TokenKind::Identifier).map(|t| t.text)
}

fn id<'a>(identifier: &str) -> impl TokenParser<'a, &'a Token<'a>> + use<'a, '_> {
    let p = token(TokenKind::Identifier).filter(move |t| t.text == identifier);
    attempt(p)
}

fn literal<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    item_if(|_| true)
        .map_if(|t: &'a Token<'a>| Some(Expr::Literal(match t.kind {
            FloatLiteral  => Value::F32(t.text.parse().unwrap()),
            HexLiteral    => Value::I32(i32::from_str_radix(&t.text[2..], 16).unwrap()),
            IntLiteral    => Value::I32(t.text.parse().unwrap()),
            StringLiteral => Value::String(t.text.into()),
            _             => None?,
        })))
}

fn let_binding<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    tuplify!(
        keyword("let"),
        identifier(),
        token(Equals),
        expression(),
    ).map(|(_, name, _, expr)| {
        Expr::LetBinding {
            name: name.into(),
            let_expr: Box::new(expr),
        }
    })
}

fn variable<'a>() -> impl TokenParser<'a, Expr> {
    identifier()
        .map(|t| Expr::Var { name: t.into(), self_fns: vec![] })
}

fn qualified_name<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    tuplify!(
        identifier(),
        token(DoubleColon),
        identifier()
    ).map(|(owner, _, member)| {
        Expr::QualifiedMember(format_compact!("{owner}::{member}"))
    })
}

fn function_expression<'a>() -> impl TokenParser<'a, Expr> {
    tuplify!(
        function_call(),
        method_chain()
    ).map(|(call, self_fns)| Expr::FnCall { call, self_fns })
}

fn function_call<'a>() -> impl TokenParser<'a, FnCallInfo> {
    use TokenKind::*;

    let qualified = tuplify!(
        identifier(),
        token(DoubleColon),
        identifier(),
        middle(token(LeftParen), arguments(), token(RightParen)),
    ).map(|(owner, _, fun, args)| {
        FnCallInfo::new(format_compact!("{owner}::{fun}"), args)
    });

    let unqualified = tuplify!(
        identifier(),
        middle(token(LeftParen), arguments(), token(RightParen)),
    ).map(|(fun, args)| FnCallInfo::new(fun, args));

    or!(qualified, unqualified)
}

fn method_chain<'a>() -> impl TokenParser<'a, Vec<FnCallInfo>> {
    use TokenKind::*;

    let chained_fn = //right!(token(TokenKind::Dot), defer_parser!(fn_call()));

    tuplify!(
        token(Dot),
        identifier(),
        middle(token(LeftParen), arguments(), token(RightParen)),
    ).map(|(_, fun, args)| FnCallInfo::new(fun, args));

    many_to_vec(chained_fn, true, no_separator())
}

fn array<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    tuplify!(
        token(LeftBracket),
        arguments(),
        token(RightBracket)
    ).map(|(_, args, _)| Expr::Array(args))
}

fn array_reference<'a>() -> impl TokenParser<'a, Expr> {
    use TokenKind::*;

    tuplify!(
        token(Ampersand),
        token(LeftBracket),
        arguments(),
        token(RightBracket),
    ).map(|(_, _, args, _)| Expr::ArrayRef(args))
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::tokenizer::tokenize;
    use anpa::core::parse;
    use compact_str::ToCompactString;


    // Helper function to create a Expr::FnCall expression
    fn expr_fn_call(
        name: &str,
        args: Vec<Expr>,
    ) -> Expr {
        Expr::FnCall {
            call: FnCallInfo::new(name, args),
            self_fns: vec![]
        }
    }

    impl Expr {
        /// Add chained methods to the expression
        fn with_self_fns(self, self_fns: Vec<FnCallInfo>) -> Self {
            match self {
                Expr::FnCall { call, .. } => Expr::FnCall { call, self_fns },
                _ => panic!("Expected FnCall expression")
            }
        }
    }

    /// Helper function to create a FnCallInfo struct
    fn fn_info(name: &str, args: Vec<Expr>) -> FnCallInfo {
        FnCallInfo {
            name: name.into(),
            args
        }
    }

    /// Helper function to run tests on tokenized input
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

        println!("{:?}", tokens);

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

        // Test with integer literal
        with_tokens("0x20", |tokens| {
            assert_eq!(
                parse(literal(), tokens).result,
                Some(Expr::Literal(Value::I32(32)))
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
    fn test_qualified_member_parser() {
        // Test with valid qualified member
        with_tokens("Color::Red", |tokens| {
            assert_eq!(
                parse(qualified_name(), tokens).result,
                Some(Expr::QualifiedMember("Color::Red".into()))
            );
        });

        // Test with unqualified member
        with_tokens("Red", |tokens| {
            assert_eq!(
                parse(qualified_name(), tokens).result,
                None
            );
        });

        // Test with invalid format
        with_tokens("Color.Red", |tokens| {
            assert_eq!(
                parse(qualified_name(), tokens).result,
                None
            );
        });
    }

    #[test]
    fn test_argument_parser() {
        // Test with literal
        with_tokens("42", |tokens| {
            assert_eq!(
                parse(expression(), tokens).result,
                Some(Expr::Literal(Value::I32(42)))
            );
        });

        // Test with variable
        with_tokens("my_var", |tokens| {
            assert_eq!(
                parse(expression(), tokens).result,
                Some(Expr::Var {
                    name: "my_var".into(),
                    self_fns: vec![]
                })
            );
        });

        // Test with qualified member
        with_tokens("Color::Red", |tokens| {
            assert_eq!(
                parse(expression(), tokens).result,
                Some(Expr::QualifiedMember("Color::Red".into()))
            );
        });

        // Test with function call
        with_tokens("fx::fade_to()", |tokens| {
            assert_eq!(
                parse(expression(), tokens).result,
                Some(expr_fn_call("fx::fade_to", vec![]))
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

        // Test with complex arguments including qualified members
        with_tokens("Color::Red, 500, fx::dissolve(200)", |tokens| {
            assert_eq!(
                parse(arguments(), tokens).result,
                Some(vec![
                    Expr::QualifiedMember("Color::Red".into()),
                    Expr::Literal(Value::I32(500)),
                    expr_fn_call("fx::dissolve", vec![Expr::Literal(Value::I32(200))])
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
                    let_expr: Box::new(Expr::Literal(Value::I32(42)))
                })
            );
        });

        // Test let binding with complex expression
        with_tokens("let color = Color::Red;", |tokens| {
            assert_eq!(
                parse(let_binding(), tokens).result,
                Some(Expr::LetBinding {
                    name: "color".into(),
                    let_expr: Box::new(Expr::QualifiedMember("Color::Red".into()))
                })
            );
        });

        // Test let binding with function call
        with_tokens("let effect = fx::fade_to(Color::Red, 500);", |tokens| {
            assert_eq!(
                parse(let_binding(), tokens).result,
                Some(Expr::LetBinding {
                    name: "effect".into(),
                    let_expr: Box::new(expr_fn_call("fx::fade_to", vec![
                        Expr::QualifiedMember("Color::Red".into()),
                        Expr::Literal(Value::I32(500))
                    ]))
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
                    args: vec![]
                })
            );
        });

        // Test with arguments
        with_tokens("fx::fade_to(42, \"hello\", my_var)", |tokens| {
            assert_eq!(
                parse(function_call(), tokens).result,
                Some(fn_info("fx::fade_to", vec![
                    Expr::Literal(Value::I32(42)),
                    Expr::Literal(Value::String("hello".into())),
                    Expr::Var { name: "my_var".into(), self_fns: vec![] }
                ]))
            );
        });

        // Test with qualified member in arguments
        with_tokens("fx::fade_to(Color::Red, 500)", |tokens| {
            assert_eq!(
                parse(function_call(), tokens).result,
                Some(fn_info("fx::fade_to", vec![
                    Expr::QualifiedMember("Color::Red".into()),
                    Expr::Literal(Value::I32(500))
                ]))
            );
        });

        // Test with nested function calls
        with_tokens("fx::sequence(fx::dissolve(200), fx::fade_to(Color::Red, 300))", |tokens| {
            assert_eq!(
                parse(function_call(), tokens).result,
                Some(fn_info("fx::sequence", vec![
                    expr_fn_call("fx::dissolve", vec![Expr::Literal(Value::I32(200))]),
                    expr_fn_call("fx::fade_to", vec![
                        Expr::QualifiedMember("Color::Red".into()),
                        Expr::Literal(Value::I32(300))
                    ])
                ]))
            );
        });

        with_tokens("fade_to()", |tokens| {
            assert_eq!(
                parse(function_call(), tokens).result,
                Some(fn_info("fade_to", vec![]))
            );
        });
    }

    #[test]
    fn test_chained_fns_parser() {
        // Test with no chained methods
        with_tokens("", |tokens| {
            assert_eq!(
                parse(method_chain(), tokens).result,
                Some(vec![])
            );
        });

        // Test with single chained method
        with_tokens(".filter(CellFilter::Text)", |tokens| {
            assert_eq!(
                parse(method_chain(), tokens).result,
                Some(vec![
                    FnCallInfo {
                        name: "filter".into(),
                        args: vec![Expr::QualifiedMember("CellFilter::Text".into())]
                    }
                ])
            );
        });

        // Test with multiple chained methods
        with_tokens(".filter(CellFilter::Text).with_area(Rect::new(0, 0, 10, 10))", |tokens| {
            assert_eq!(
                parse(method_chain(), tokens).result,
                Some(vec![
                    fn_info("filter", vec![Expr::QualifiedMember("CellFilter::Text".into())]),
                    fn_info("with_area", vec![
                        expr_fn_call("Rect::new", vec![
                            Expr::Literal(Value::I32(0)),
                            Expr::Literal(Value::I32(0)),
                            Expr::Literal(Value::I32(10)),
                            Expr::Literal(Value::I32(10))
                        ])
                    ])
                ])
            );
        });
    }

    #[test]
    fn test_fn_call_expr_parser() {
        // Test function call with method chaining
        with_tokens("fx::fade_to(Color::Red, 500).filter(CellFilter::Text)", |tokens| {
            assert_eq!(
                parse(function_expression(), tokens).result,
                Some(
                    expr_fn_call("fx::fade_to", vec![
                        Expr::QualifiedMember("Color::Red".into()),
                        Expr::Literal(Value::I32(500))
                    ]).with_self_fns(vec![
                        fn_info("filter", vec![Expr::QualifiedMember("CellFilter::Text".into())])
                    ])
                )
            );
        });

        // Test function call with multiple method chains
        with_tokens("fx::dissolve(200).filter(CellFilter::Text).with_area(Rect::new(0, 0, 10, 10))", |tokens| {
            assert_eq!(
                parse(function_expression(), tokens).result,
                Some(expr_fn_call("fx::dissolve", vec![Expr::Literal(Value::I32(200))])
                    .with_self_fns(vec![
                        fn_info("filter", vec![Expr::QualifiedMember("CellFilter::Text".into())]),
                        fn_info("with_area", vec![
                            expr_fn_call("Rect::new", vec![
                                Expr::Literal(Value::I32(0)),
                                Expr::Literal(Value::I32(0)),
                                Expr::Literal(Value::I32(10)),
                                Expr::Literal(Value::I32(10))
                            ])
                        ])
                    ])
                )
            );
        });

        // Test nested function calls with method chaining
        with_tokens("fx::sequence(fx::dissolve(200).reversed(), fx::fade_to(Color::Red, 300)).filter(CellFilter::Text)", |tokens| {
            assert_eq!(
                parse(function_expression(), tokens).result,
                Some(
                    expr_fn_call("fx::sequence", vec![
                        expr_fn_call("fx::dissolve", vec![Expr::Literal(Value::I32(200))])
                            .with_self_fns(vec![fn_info("reversed", vec![])]),
                        expr_fn_call("fx::fade_to", vec![
                            Expr::QualifiedMember("Color::Red".into()),
                            Expr::Literal(Value::I32(300))
                        ])
                    ]).with_self_fns(vec![
                        fn_info("filter", vec![Expr::QualifiedMember("CellFilter::Text".into())])
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
                Some(Expr::Array(vec![]))
            );
        });

        // Test array with single element
        with_tokens("[42]", |tokens| {
            assert_eq!(
                parse(array(), tokens).result,
                Some(Expr::Array(vec![Expr::Literal(Value::I32(42))]))
            );
        });

        // Test array with multiple elements
        with_tokens("[42, \"hello\", Color::Red]", |tokens| {
            assert_eq!(
                parse(array(), tokens).result,
                Some(Expr::Array(vec![
                    Expr::Literal(Value::I32(42)),
                    Expr::Literal(Value::String("hello".into())),
                    Expr::QualifiedMember("Color::Red".into())
                ]))
            );
        });

        // Test array with function calls
        with_tokens("[fx::dissolve(200), fx::fade_to(Color::Red, 300)]", |tokens| {
            assert_eq!(
                parse(array(), tokens).result,
                Some(Expr::Array(vec![
                    expr_fn_call("fx::dissolve", vec![Expr::Literal(Value::I32(200))]),
                    expr_fn_call("fx::fade_to", vec![
                        Expr::QualifiedMember("Color::Red".into()),
                        Expr::Literal(Value::I32(300))
                    ])
                ]))
            );
        });
    }

    #[test]
    fn test_array_ref_parser() {
        // Test empty array reference
        with_tokens("&[]", |tokens| {
            assert_eq!(
                parse(array_reference(), tokens).result,
                Some(Expr::ArrayRef(vec![]))
            );
        });

        // Test array reference with single element
        with_tokens("&[42]", |tokens| {
            assert_eq!(
                parse(array_reference(), tokens).result,
                Some(Expr::ArrayRef(vec![Expr::Literal(Value::I32(42))]))
            );
        });

        // Test array reference with multiple elements
        with_tokens("&[42, \"hello\", Color::Red]", |tokens| {
            assert_eq!(
                parse(array_reference(), tokens).result,
                Some(Expr::ArrayRef(vec![
                    Expr::Literal(Value::I32(42)),
                    Expr::Literal(Value::String("hello".into())),
                    Expr::QualifiedMember("Color::Red".into())
                ]))
            );
        });

        // Test array reference with function calls
        with_tokens("&[fx::dissolve(200), fx::fade_to(Color::Red, 300)]", |tokens| {
            assert_eq!(
                parse(array_reference(), tokens).result,
                Some(Expr::ArrayRef(vec![
                    expr_fn_call("fx::dissolve", vec![Expr::Literal(Value::I32(200))]),
                    expr_fn_call("fx::fade_to", vec![
                        Expr::QualifiedMember("Color::Red".into()),
                        Expr::Literal(Value::I32(300))
                    ])
                ]))
            );
        });
    }

    #[test]
    fn test_complex_method_chaining() {
        // Test complex method chaining with nested function calls
        with_tokens("fx::sequence(&[fx::dissolve(200), fx::fade_to(Color::Red, 300)])
                    .filter(CellFilter::Text)
                    .with_area(Rect::new(0, 0, 10, 10).offset(5, 5))", |tokens| {
            let anpa_result = parse(function_expression(), tokens);
            assert_eq!(anpa_result.state, &[]);
            let result = anpa_result.result;

            let expr = result.unwrap();
            if let Expr::FnCall { call, self_fns } = expr {
                assert_eq!(call.name, "fx::sequence".to_compact_string());
                assert_eq!(self_fns.len(), 2);
                assert_eq!(self_fns[0].name, "filter".to_compact_string());
                assert_eq!(self_fns[1].name, "with_area".to_compact_string());

                // Check the nested Rect with its own method chain
                if let Expr::FnCall { call: rect_call, self_fns: rect_fns } = &self_fns[1].args[0] {
                    assert_eq!(rect_call.name, "Rect::new".to_compact_string());
                    assert_eq!(rect_fns.len(), 1);
                    assert_eq!(rect_fns[0].name, "offset".to_compact_string());
                }
            } else {
                panic!("Expected FnCall expression");
            }
        });

        // Test method chaining with different types of methods
        with_tokens("fx::fade_to(Color::Red, 500)
                    .filter(CellFilter::Text)
                    .with_area(Rect::new(0, 0, 10, 10))
                    .reversed()
                    .clone()", |tokens| {
            let result = parse(function_expression(), tokens).result;
            assert!(result.is_some());

            let expr = result.unwrap();
            if let Expr::FnCall { call, self_fns } = expr {
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
        });
    }

    #[test]
    fn test_maybe_qualified_parser() {
        // Test with qualified identifier
        with_tokens("fx::", |tokens| {
            assert_eq!(
                parse(maybe_qualified("fx"), tokens).result,
                Some(())
            );
        });

        // Test with non-matching qualified identifier; should always succeed
        with_tokens("color::", |tokens| {
            assert_eq!(
                parse(maybe_qualified("fx"), tokens).result,
                Some(())
            );
        });
    }

    #[test]
    fn test_some_parser() {
        // Test with a simple value
        with_tokens("Some(42)", |tokens| {
            assert_eq!(
                parse(some(), tokens).result,
                Some(Expr::OptionSome(Box::new(Expr::Literal(Value::I32(42)))))
            );
        });

        // Test with a complex expression
        with_tokens("Some(Color::Red)", |tokens| {
            assert_eq!(
                parse(some(), tokens).result,
                Some(Expr::OptionSome(Box::new(Expr::QualifiedMember("Color::Red".into()))))
            );
        });

        // Test with a nested function call
        with_tokens("Some(fx::dissolve(200))", |tokens| {
            assert_eq!(
                parse(some(), tokens).result,
                Some(Expr::OptionSome(Box::new(
                    expr_fn_call("fx::dissolve", vec![Expr::Literal(Value::I32(200))])
                )
            )));
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
                    self_fns: vec![]
                })
            );
        });

        // Test with single effect
        with_tokens("fx::sequence(fx::dissolve(200))", |tokens| {
            assert_eq!(
                parse(sequence(), tokens).result,
                Some(Expr::Sequence {
                    effects: vec![
                        expr_fn_call("fx::dissolve", vec![Expr::Literal(Value::I32(200))])
                    ],
                    self_fns: vec![]
                })
            );
        });

        // Test with multiple effects
        with_tokens("fx::sequence(dissolve(200), fx::fade_to(Color::Red, 300))", |tokens| {
            assert_eq!(
                parse(sequence(), tokens).result,
                Some(Expr::Sequence {
                    effects: vec![
                        expr_fn_call("dissolve", vec![Expr::Literal(Value::I32(200))]),
                        expr_fn_call("fx::fade_to", vec![
                            Expr::QualifiedMember("Color::Red".into()),
                            Expr::Literal(Value::I32(300))
                        ])
                    ],
                    self_fns: vec![]
                })
            );
        });

        // Test with method chaining
        with_tokens("fx::sequence(fx::dissolve(200)).filter(CellFilter::Text)", |tokens| {
            assert_eq!(
                parse(sequence(), tokens).result,
                Some(Expr::Sequence {
                    effects: vec![
                        expr_fn_call("fx::dissolve", vec![Expr::Literal(Value::I32(200))]),
                    ],
                    self_fns: vec![
                        fn_info("filter", vec![Expr::QualifiedMember("CellFilter::Text".into())])
                    ]
                })
            );
        });

        // Test with array reference
        with_tokens("sequence(&[fx::dissolve(200), fx::fade_to(Color::Red, 300)])", |tokens| {
            assert_eq!(
                parse(sequence(), tokens).result,
                Some(Expr::Sequence {
                    effects: vec![
                        Expr::ArrayRef(vec![
                            expr_fn_call("fx::dissolve", vec![Expr::Literal(Value::I32(200))]),
                            expr_fn_call("fx::fade_to", vec![
                                Expr::QualifiedMember("Color::Red".into()),
                                Expr::Literal(Value::I32(300))
                            ])
                        ])
                    ],
                    self_fns: vec![]
                })
            );
        });
    }

    #[test]
    fn test_parallel_parser() {
        // Test with empty parallel
        with_tokens("fx::parallel()", |tokens| {
            assert_eq!(
                parse(parallel(), tokens).result,
                Some(Expr::Parallel {
                    effects: vec![],
                    self_fns: vec![]
                })
            );
        });

        // Test with single effect
        with_tokens("fx::parallel(fx::dissolve(200))", |tokens| {
            assert_eq!(
                parse(parallel(), tokens).result,
                Some(Expr::Parallel {
                    effects: vec![
                        expr_fn_call("fx::dissolve", vec![Expr::Literal(Value::I32(200))])
                    ],
                    self_fns: vec![]
                })
            );
        });

        // Test with multiple effects
        with_tokens("fx::parallel(fx::dissolve(200), fx::fade_to(Color::Red, 300))", |tokens| {
            assert_eq!(
                parse(parallel(), tokens).result,
                Some(Expr::Parallel {
                    effects: vec![
                        expr_fn_call("fx::dissolve", vec![Expr::Literal(Value::I32(200))]),
                        expr_fn_call("fx::fade_to", vec![
                            Expr::QualifiedMember("Color::Red".into()),
                            Expr::Literal(Value::I32(300))
                        ])
                    ],
                    self_fns: vec![]
                })
            );
        });

        // Test with method chaining
        with_tokens("fx::parallel(fx::dissolve(200)).filter(CellFilter::Text)", |tokens| {
            assert_eq!(
                parse(parallel(), tokens).result,
                Some(Expr::Parallel {
                    effects: vec![
                        expr_fn_call("fx::dissolve", vec![Expr::Literal(Value::I32(200))])
                    ],
                    self_fns: vec![
                        fn_info("filter", vec![Expr::QualifiedMember("CellFilter::Text".into())])
                    ]
                })
            );
        });

        // Test with array reference
        with_tokens("parallel(&[fx::dissolve(200), fx::fade_to(Color::Red, 300)])", |tokens| {
            assert_eq!(
                parse(parallel(), tokens).result,
                Some(Expr::Parallel {
                    effects: vec![
                        Expr::ArrayRef(vec![
                            expr_fn_call("fx::dissolve", vec![Expr::Literal(Value::I32(200))]),
                            expr_fn_call("fx::fade_to", vec![
                                Expr::QualifiedMember("Color::Red".into()),
                                Expr::Literal(Value::I32(300))
                            ])
                        ])
                    ],
                    self_fns: vec![]
                })
            );
        });
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
        // Test sequence expression via main expression parser
        with_tokens("fx::sequence(fx::dissolve(200), fx::fade_to(Color::Red, 300))", |tokens| {
            let result = parse(expression(), tokens).result;
            assert!(result.is_some());

            match result.unwrap() {
                Expr::Sequence { effects, self_fns } => {
                    assert_eq!(
                        effects,
                        vec![
                            expr_fn_call("fx::dissolve", vec![
                                Expr::Literal(Value::I32(200))
                            ]),
                            expr_fn_call("fx::fade_to", vec![
                                Expr::QualifiedMember("Color::Red".to_compact_string()),
                                Expr::Literal(Value::I32(300))
                            ])
                        ]
                    );
                    assert_eq!(self_fns.len(), 0);
                },
                e => panic!("Expected FnCall expression, got {:?}", e)
            }
        });

        // Test parallel expression via main expression parser
        with_tokens("fx::parallel(fx::dissolve(200), fx::fade_to(Color::Red, 300))", |tokens| {
            let result = parse(expression(), tokens).result;
            assert!(result.is_some());

            match result.unwrap() {
                Expr::Parallel { effects, .. } => {
                    assert_eq!(
                        effects,
                        vec![
                            expr_fn_call("fx::dissolve", vec![
                                Expr::Literal(Value::I32(200))
                            ]),
                            expr_fn_call("fx::fade_to", vec![
                                Expr::QualifiedMember("Color::Red".to_compact_string()),
                                Expr::Literal(Value::I32(300))
                            ])
                        ]
                    ); // Should have two arguments
                },
                e => panic!("Expected Parallel expression, got {:?}", e)
            }
        });

        // Test Some option via main expression parser
        with_tokens("Some(fx::dissolve(200))", |tokens| {
            let result = parse(expression(), tokens).result;
            assert!(result.is_some());

            match result.unwrap() {
                Expr::OptionSome(expr) => {
                    assert_eq!(
                        expr,
                        Box::new(expr_fn_call("fx::dissolve", vec![
                            Expr::Literal(Value::I32(200))
                        ]))
                    );
                },
                _ => panic!("Expected OptionSome expression")
            }
        });
    }
}