use crate::dsl::{
    expressions::ExprSpan,
    tokenizer::{Token, TokenKind},
    DslError,
};

pub(super) fn verify_tokens(tokens: Vec<Token>) -> Result<Vec<Token>, DslError> {
    verify_brackets(tokens)
        .and_then(verify_semicolons)
        .and_then(verify_commas)
}

fn verify_brackets(tokens: Vec<Token>) -> Result<Vec<Token>, DslError> {
    let mut stack: Vec<&Token> = Vec::new();

    const LEFT_BRACKETS: [TokenKind; 3] =
        [TokenKind::LeftParen, TokenKind::LeftBracket, TokenKind::LeftBrace];

    const RIGHT_BRACKETS: [TokenKind; 3] =
        [TokenKind::RightParen, TokenKind::RightBracket, TokenKind::RightBrace];

    let rhs = |t: &TokenKind| match t {
        TokenKind::LeftParen => TokenKind::RightParen,
        TokenKind::LeftBracket => TokenKind::RightBracket,
        TokenKind::LeftBrace => TokenKind::RightBrace,
        _ => unreachable!(),
    };

    let bracket_mismatch =
        |t: &Token, bracket_type: &'static str| -> Result<Vec<Token>, DslError> {
            let bracket = t.text.chars().next().unwrap();
            Err(DslError::BracketMismatch {
                bracket,
                location: ExprSpan::new(t.span.0, t.span.1),
                bracket_type,
            })
        };

    for token in &tokens {
        if LEFT_BRACKETS.contains(&token.kind) {
            // push to stack
            stack.push(token);
        } else if RIGHT_BRACKETS.contains(&token.kind) {
            if let Some(top) = stack.pop() {
                if token.kind != rhs(&top.kind) {
                    // mismatched brackets
                    return bracket_mismatch(token, "closing");
                }
            } else {
                // unmatched closing bracket
                return bracket_mismatch(token, "closing");
            }
        }
    }

    if let Some(trailing) = stack.last() {
        // unmatched opening bracket
        return bracket_mismatch(trailing, "opening");
    }

    Ok(tokens)
}

fn verify_semicolons(tokens: Vec<Token>) -> Result<Vec<Token>, DslError> {
    if tokens.is_empty() {
        return Ok(tokens);
    }

    // Check for consecutive semicolons
    for i in 1..tokens.len() {
        if tokens[i].kind == TokenKind::Semicolon && tokens[i - 1].kind == TokenKind::Semicolon {
            return Err(DslError::SyntaxError {
                message: "Multiple consecutive semicolons".into(),
                location: ExprSpan::new(tokens[i].span.0, tokens[i].span.1),
            });
        }
    }

    // find statement boundaries (let statements); since we don't have flow
    // control, we just need to check for let statements followed by other statements
    // This is a good-enough heuristic for most cases in the DSL
    for i in 0..tokens.len() - 1 {
        if tokens[i].kind == TokenKind::Keyword && tokens[i].text == "let" {
            // find the end of this let statement
            let mut j = i + 1;
            let mut depth = 0;

            while j < tokens.len() {
                match tokens[j].kind {
                    TokenKind::LeftParen | TokenKind::LeftBrace | TokenKind::LeftBracket => {
                        depth += 1;
                    },
                    TokenKind::RightParen | TokenKind::RightBrace | TokenKind::RightBracket => {
                        if depth > 0 {
                            depth -= 1;
                        }
                    },
                    TokenKind::Semicolon if depth == 0 => break,
                    _ => {},
                }
                j += 1;
            }

            // if we didn't find a semicolon at depth 0, and we're not
            // at the end of the file, we're missing a semicolon
            if j == tokens.len() && i < tokens.len() - 2 {
                return Err(DslError::MissingSemicolon {
                    location: ExprSpan::new(tokens[j - 1].span.1, tokens[j - 1].span.1 + 1),
                });
            }
        }
    }

    Ok(tokens)
}

fn verify_commas(tokens: Vec<Token>) -> Result<Vec<Token>, DslError> {
    if tokens.is_empty() {
        return Ok(tokens);
    }

    let mut stack: Vec<(TokenKind, usize)> = Vec::new(); // Stack of (delimiter, position)
    let mut last_token_kind = TokenKind::Whitespace; // Placeholder initial value

    for (i, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::LeftBracket | TokenKind::LeftParen => {
                // push opening delimiter onto stack
                stack.push((token.kind, i));
            },

            TokenKind::RightBracket | TokenKind::RightParen | TokenKind::RightBrace => {
                if let Some((opening, _)) = stack.last() {
                    // check if matching opening delimiter
                    let is_matching = matches!(
                        (opening, token.kind),
                        (TokenKind::LeftBracket, TokenKind::RightBracket)
                            | (TokenKind::LeftParen, TokenKind::RightParen)
                            | (TokenKind::LeftBrace, TokenKind::RightBrace)
                    );

                    if is_matching {
                        stack.pop();
                    }
                }
            },

            // A token that might need a comma before it
            TokenKind::Identifier
            | TokenKind::StringLiteral
            | TokenKind::IntLiteral
            | TokenKind::FloatLiteral
            | TokenKind::HexLiteral => {
                // if we're inside brackets/parens, we might need a comma
                if !stack.is_empty() {
                    let (delimiter_kind, last_delimiter_pos) = *stack.last().unwrap();

                    // only check for missing commas in arrays and parameter lists, not in struct
                    // initializations
                    if delimiter_kind == TokenKind::LeftBracket
                        || delimiter_kind == TokenKind::LeftParen
                    {
                        // If the last token was also an expression-like token, we might be missing
                        // a comma
                        if is_expression_token(last_token_kind)
                            && !is_opening_delimiter(last_token_kind)
                        {
                            // but we don't need a comma right after an opening delimiter
                            if last_delimiter_pos != i - 1 {
                                // Not immediately after the opening delimiter
                                return Err(DslError::MissingComma {
                                    location: ExprSpan::new(tokens[i - 1].span.1, token.span.0),
                                });
                            }
                        }
                    }
                }
            },

            _ => {},
        }

        // Update for next iteration
        last_token_kind = token.kind;
    }

    Ok(tokens)
}

fn is_expression_token(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Identifier
            | TokenKind::StringLiteral
            | TokenKind::IntLiteral
            | TokenKind::FloatLiteral
            | TokenKind::HexLiteral
            | TokenKind::RightBrace
            | TokenKind::RightBracket
            | TokenKind::RightParen
    )
}

fn is_opening_delimiter(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::LeftBracket | TokenKind::LeftParen | TokenKind::LeftBrace
    )
}
