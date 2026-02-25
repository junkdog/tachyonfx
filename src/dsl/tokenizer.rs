use core::{fmt, fmt::Formatter, ops::Range};

use anpa::{
    combinators::{
        attempt, count_consumed, get_parsed, many, many_to_vec, middle, no_separator, not_empty,
        or_diff, right, succeed, times,
    },
    core::{ParserExt, StrParser},
    greedy_or,
    number::float,
    or,
    parsers::{item_if, item_while, until},
    right, skip, take,
};

use crate::dsl::{expressions::ExprSpan, DslError};

/// Represents the type of a token in the DSL
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum TokenKind {
    // literals
    Identifier,
    StringLiteral,
    IntLiteral,
    HexLiteral,
    FloatLiteral,

    // keywords
    Keyword,

    // structural
    LeftParen,    // (
    RightParen,   // )
    LeftBracket,  // [
    RightBracket, // ]
    LeftBrace,    // {
    RightBrace,   // }
    Comma,        // ,
    Dot,          // .
    Colon,        // :
    Semicolon,    // ;
    Equals,       // =
    Ampersand,    // &
    DoubleColon,  // ::
    Minus,        // -
    Bang,         // !

    // comments
    LineComment,  // discarded
    BlockComment, // discarded

    // special
    Whitespace, // discarded
}

/// A token in the DSL with its kind, value, and source position
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Token<'a> {
    /// The type of token
    pub kind: TokenKind,
    /// The text content of the token
    pub text: &'a str,
    /// The byte range in the source text
    pub span: (u32, u32),
}

impl<'a> Token<'a> {
    /// Creates a new token with the given kind, text, and span
    pub(super) fn new(kind: TokenKind, text: &'a str, span: Range<usize>) -> Self {
        Self { kind, text, span: (span.start as _, span.end as _) }
    }

    /// Checks if the token's span contains the given index
    pub(super) fn contains_index(&self, idx: u32) -> bool {
        let (start, end) = self.span;
        idx >= start && idx < end
    }
}

pub(super) fn sanitize_tokens(tokens: Vec<Token>) -> Vec<Token> {
    const DISCARD: &[TokenKind] =
        &[TokenKind::Whitespace, TokenKind::LineComment, TokenKind::BlockComment];

    tokens
        .into_iter()
        .filter(|t| !DISCARD.contains(&t.kind))
        .collect::<Vec<_>>()
}

pub(super) fn tokenize(input: &str) -> Result<Vec<Token<'_>>, DslError> {
    let result = anpa::core::parse(tokens(), input);
    if !result.state.is_empty() {
        let consumed = input.len() - result.state.len();
        return Err(DslError::TokenizationError {
            location: ExprSpan::new(consumed.saturating_sub(1) as _, input.len() as _),
        });
    };

    result
        .result
        .map(|tokens| {
            let mut tokens = tokens;
            let mut offset = 0;
            for token in &mut tokens {
                let (start, end) = token.span;
                token.span = (start + offset, end + offset);
                offset = token.span.1;
            }

            tokens
        })
        .ok_or(DslError::OhNoError)
}

fn tokens<'a>() -> impl StrParser<'a, Vec<Token<'a>>> {
    let p = or!(
        whitespace(),
        double_colon(),
        hex_literal(),
        greedy_or!(int_literal(), float_literal()),
        structural(),
        string_literal(),
        keyword(),
        line_comment(),
        block_comment(),
        identifier(),
    );

    many_to_vec(p, true, no_separator())
}

fn snake_case_str<'a>() -> impl StrParser<'a, &'a str> {
    let p = not_empty(item_while(|c: char| c.is_ascii_alphanumeric() || c == '_'));
    get_parsed(p)
}

fn identifier<'a>() -> impl StrParser<'a, Token<'a>> {
    token(TokenKind::Identifier, snake_case_str())
}

fn keyword<'a>() -> impl StrParser<'a, Token<'a>> {
    let p = attempt(snake_case_str().map_if(|s| {
        Some(match s {
            "let" => TokenKind::Keyword,
            "return" => TokenKind::Keyword,
            "if" => TokenKind::Keyword,
            "else" => TokenKind::Keyword,
            "while" => TokenKind::Keyword,
            "for" => TokenKind::Keyword,
            "in" => TokenKind::Keyword,
            "break" => TokenKind::Keyword,
            "continue" => TokenKind::Keyword,
            "true" => TokenKind::Keyword,
            "false" => TokenKind::Keyword,
            _ => None?,
        })
    }));

    token(TokenKind::Keyword, get_parsed(p))
}

fn float_literal<'a>() -> impl StrParser<'a, Token<'a>> {
    let sign = attempt(succeed(or!(skip!('-'), skip!('+'))));
    let plain = float::<f32, char, &str, ()>();
    token(TokenKind::FloatLiteral, get_parsed(right!(sign, plain)))
}

fn int_literal<'a>() -> impl StrParser<'a, Token<'a>> {
    let sign = attempt(succeed(or!(skip!('-'), skip!('+'))));
    let plain = many(item_if(|c: char| c.is_ascii_digit()), false, no_separator());
    token(TokenKind::IntLiteral, get_parsed(right!(sign, plain)))
}

fn hex_literal<'a>() -> impl StrParser<'a, Token<'a>> {
    let hex = get_parsed(right(
        skip!("0x"),
        item_while(|c: char| c.is_ascii_hexdigit()),
    ));
    token(TokenKind::HexLiteral, hex)
}

fn string_literal<'a>() -> impl StrParser<'a, Token<'a>> {
    let unicode = right(
        skip!('u'),
        times(4, item_if(|c: char| c.is_ascii_hexdigit())),
    );
    let escaped = right(
        skip!('\\'),
        or_diff(unicode, item_if(|c: char| "\"\\/bfnrt".contains(c))),
    );
    let valid_char = item_if(|c: char| c != '"' && c != '\\' && !c.is_control());
    let not_end = or_diff(valid_char, escaped);

    let string_literal = middle(skip!('"'), many(not_end, true, no_separator()), skip!('"'));
    token(TokenKind::StringLiteral, string_literal)
}

fn line_comment<'a>() -> impl StrParser<'a, Token<'a>> {
    let line_comment = get_parsed(right!(skip!("//"), item_while(|c: char| c != '\n')));
    token(TokenKind::LineComment, line_comment)
}

fn block_comment<'a>() -> impl StrParser<'a, Token<'a>> {
    let block_comment = get_parsed(right!(skip!("/*"), until("*/")));
    token(TokenKind::BlockComment, block_comment)
}

fn double_colon<'a>() -> impl StrParser<'a, Token<'a>> {
    token(TokenKind::DoubleColon, take!("::"))
}

fn structural<'a>() -> impl StrParser<'a, Token<'a>> {
    let p = get_parsed(item_if(|c: char| "()[]{},.:;=&-!".contains(c))).map(|t: &str| {
        (t, match t {
            "(" => TokenKind::LeftParen,
            ")" => TokenKind::RightParen,
            "[" => TokenKind::LeftBracket,
            "]" => TokenKind::RightBracket,
            "{" => TokenKind::LeftBrace,
            "}" => TokenKind::RightBrace,
            "," => TokenKind::Comma,
            "." => TokenKind::Dot,
            ":" => TokenKind::Colon,
            ";" => TokenKind::Semicolon,
            "=" => TokenKind::Equals,
            "&" => TokenKind::Ampersand,
            "-" => TokenKind::Minus,
            "!" => TokenKind::Bang,
            _ => unreachable!(),
        })
    });

    count_consumed(p).map(|(len, (s, kind))| Token::new(kind, s, 0..len))
}

fn whitespace<'a>() -> impl StrParser<'a, Token<'a>> {
    token(
        TokenKind::Whitespace,
        not_empty(anpa::whitespace::whitespace()),
    )
}

fn token<'a>(kind: TokenKind, p: impl StrParser<'a, &'a str>) -> impl StrParser<'a, Token<'a>> {
    count_consumed(p).map(move |(len, s): (_, &str)| Token::new(kind, s, 0..len))
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} ", self.text)
    }
}

#[cfg(test)]
mod tests {
    use TokenKind::*;

    use super::*;

    // Enhanced helper function to test both token kinds and text
    fn test_tokens(input: &str, expected_tokens: &[(TokenKind, &str)]) {
        let result = tokenize(input);
        assert!(result.is_ok(), "Failed to parse tokens from input: {input}");

        let tokens = result.unwrap();

        for (token, (expected_kind, expected_text)) in tokens.iter().zip(expected_tokens.iter()) {
            assert_eq!(
                token.kind, *expected_kind,
                "Expected token kind {expected_kind:?}, but got {:?} for token text: '{}'",
                token.kind, token.text
            );

            assert_eq!(
                token.text,
                *expected_text,
                "Expected token text '{expected_text}', but got '{}' for token kind: {kind:?}",
                token.text,
                kind = token.kind
            );
        }

        assert_eq!(
            tokens.len(),
            expected_tokens.len(),
            "Expected {} tokens, but got {} for input: {}",
            expected_tokens.len(),
            tokens.len(),
            input
        );
    }

    #[test]
    fn test_identifiers() {
        test_tokens("identifier snake_case_id _leading_underscore", &[
            (Identifier, "identifier"),
            (Whitespace, " "),
            (Identifier, "snake_case_id"),
            (Whitespace, " "),
            (Identifier, "_leading_underscore"),
        ]);
    }

    #[test]
    fn test_keywords() {
        test_tokens("let if else while for in break continue true false", &[
            (Keyword, "let"),
            (Whitespace, " "),
            (Keyword, "if"),
            (Whitespace, " "),
            (Keyword, "else"),
            (Whitespace, " "),
            (Keyword, "while"),
            (Whitespace, " "),
            (Keyword, "for"),
            (Whitespace, " "),
            (Keyword, "in"),
            (Whitespace, " "),
            (Keyword, "break"),
            (Whitespace, " "),
            (Keyword, "continue"),
            (Whitespace, " "),
            (Keyword, "true"),
            (Whitespace, " "),
            (Keyword, "false"),
        ]);
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_literals() {
        test_tokens("123 -456 3.14 - -2.718 0x1a2b \"string literal\"", &[
            (IntLiteral, "123"),
            (Whitespace, " "),
            (IntLiteral, "-456"),
            (Whitespace, " "),
            (FloatLiteral, "3.14"),
            (Whitespace, " "),
            (Minus, "-"),
            (Whitespace, " "),
            (FloatLiteral, "-2.718"),
            (Whitespace, " "),
            (HexLiteral, "0x1a2b"),
            (Whitespace, " "),
            (StringLiteral, "string literal"),
        ]);
    }

    #[test]
    fn test_string_literals_with_escapes() {
        test_tokens(
            r#""simple" "with \"escaped quotes\"" "with \\backslash" "with \u1234 unicode""#,
            &[
                (StringLiteral, r#"simple"#),
                (Whitespace, " "),
                (StringLiteral, r#"with \"escaped quotes\""#),
                (Whitespace, " "),
                (StringLiteral, r#"with \\backslash"#),
                (Whitespace, " "),
                (StringLiteral, r#"with \u1234 unicode"#),
            ],
        );
    }

    #[test]
    fn test_operators_and_punctuation() {
        test_tokens("( ) [ ] { } , . : ; = & :: -", &[
            (LeftParen, "("),
            (Whitespace, " "),
            (RightParen, ")"),
            (Whitespace, " "),
            (LeftBracket, "["),
            (Whitespace, " "),
            (RightBracket, "]"),
            (Whitespace, " "),
            (LeftBrace, "{"),
            (Whitespace, " "),
            (RightBrace, "}"),
            (Whitespace, " "),
            (Comma, ","),
            (Whitespace, " "),
            (Dot, "."),
            (Whitespace, " "),
            (Colon, ":"),
            (Whitespace, " "),
            (Semicolon, ";"),
            (Whitespace, " "),
            (Equals, "="),
            (Whitespace, " "),
            (Ampersand, "&"),
            (Whitespace, " "),
            (DoubleColon, "::"),
            (Whitespace, " "),
            (Minus, "-"),
        ]);
    }

    #[test]
    fn test_comments() {
        let line_comment = "// This is a line comment";
        let block_comment = "/* This is a block comment */";
        test_tokens(&format!("{line_comment}\n{block_comment}"), &[
            (LineComment, line_comment),
            (Whitespace, "\n"),
            (BlockComment, block_comment),
        ]);
    }

    #[test]
    fn test_whitespace() {
        let ws = "  \t\n\r  ";
        test_tokens(ws, &[(Whitespace, ws)]);
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_mixed_tokens() {
        let input = "let x = 42; // assign value\nfn::call(true, 3.14);";
        test_tokens(input, &[
            (Keyword, "let"),
            (Whitespace, " "),
            (Identifier, "x"),
            (Whitespace, " "),
            (Equals, "="),
            (Whitespace, " "),
            (IntLiteral, "42"),
            (Semicolon, ";"),
            (Whitespace, " "),
            (LineComment, "// assign value"),
            (Whitespace, "\n"),
            (Identifier, "fn"),
            (DoubleColon, "::"),
            (Identifier, "call"),
            (LeftParen, "("),
            (Keyword, "true"),
            (Comma, ","),
            (Whitespace, " "),
            (FloatLiteral, "3.14"),
            (RightParen, ")"),
            (Semicolon, ";"),
        ]);
    }

    #[test]
    fn test_token_spans() {
        let input = "x = 42";
        let result = tokenize(input);
        let tokens = result.unwrap();

        assert_eq!(tokens[0].span, (0, 1)); // "x"
        assert_eq!(tokens[1].span, (1, 2)); // " "
        assert_eq!(tokens[2].span, (2, 3)); // "="
        assert_eq!(tokens[3].span, (3, 4)); // " "
        assert_eq!(tokens[4].span, (4, 6)); // "42"

        // Check that token text matches the spans
        for token in &tokens {
            let (start, end) = token.span;
            assert_eq!(token.text, &input[start as _..end as _]);
        }
    }

    #[test]
    fn test_complex_expression() {
        let input = "fx::fade_to(Color::Red, (500, CircOut))";
        test_tokens(input, &[
            (Identifier, "fx"),
            (DoubleColon, "::"),
            (Identifier, "fade_to"),
            (LeftParen, "("),
            (Identifier, "Color"),
            (DoubleColon, "::"),
            (Identifier, "Red"),
            (Comma, ","),
            (Whitespace, " "),
            (LeftParen, "("),
            (IntLiteral, "500"),
            (Comma, ","),
            (Whitespace, " "),
            (Identifier, "CircOut"),
            (RightParen, ")"),
            (RightParen, ")"),
        ]);
    }

    #[test]
    fn test_effect_declaration() {
        let input =
            r#"let fade /* yolo */ = fx::fade_to_fg(Color::from_u32(0x504945), (1000, CircOut));"#;
        test_tokens(input, &[
            (Keyword, "let"),
            (Whitespace, " "),
            (Identifier, "fade"),
            (Whitespace, " "),
            (BlockComment, "/* yolo */"),
            (Whitespace, " "),
            (Equals, "="),
            (Whitespace, " "),
            (Identifier, "fx"),
            (DoubleColon, "::"),
            (Identifier, "fade_to_fg"),
            (LeftParen, "("),
            (Identifier, "Color"),
            (DoubleColon, "::"),
            (Identifier, "from_u32"),
            (LeftParen, "("),
            (HexLiteral, "0x504945"),
            (RightParen, ")"),
            (Comma, ","),
            (Whitespace, " "),
            (LeftParen, "("),
            (IntLiteral, "1000"),
            (Comma, ","),
            (Whitespace, " "),
            (Identifier, "CircOut"),
            (RightParen, ")"),
            (RightParen, ")"),
            (Semicolon, ";"),
        ]);
    }

    #[test]
    fn test_edge_cases() {
        // Empty input
        let result = tokenize("");
        assert_eq!(result.unwrap().len(), 0);

        // Only whitespace
        test_tokens(" \t\n", &[(Whitespace, " \t\n")]);

        // Only comments
        let line_comment = "// comment";
        test_tokens(line_comment, &[(LineComment, line_comment)]);

        let block_comment = "/* comment */";
        test_tokens(block_comment, &[(BlockComment, block_comment)]);

        // Unicode characters in string literals
        test_tokens("\"Unicode: \u{1234} \u{5678}\"", &[(
            StringLiteral,
            "Unicode: \u{1234} \u{5678}",
        )]);
    }

    #[test]
    fn test_debug_implementation() {
        // Verify that TokenKind implements Debug correctly
        assert_eq!(format!("{Identifier:?}"), "Identifier");
        assert_eq!(format!("{StringLiteral:?}"), "StringLiteral");
        assert_eq!(format!("{LeftBrace:?}"), "LeftBrace");
    }
}
