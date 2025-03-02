use anpa::core::ParserExt;
use std::ops::Range;
use anpa::combinators::{attempt, count_consumed, get_parsed, many, middle, no_separator, or_diff, right, times};
use anpa::core::StrParser;
use anpa::parsers::{item_if, item_while, until};
use anpa::{or, right, skip};
use anpa::number::float;
use crate::dsl::expressions::Expr;

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
    LeftParen,        // (
    RightParen,       // )
    LeftBracket,      // [
    RightBracket,     // ]
    LeftBrace,        // {
    RightBrace,       // }
    Comma,            // ,
    Dot,              // .
    Colon,            // :
    Semicolon,        // ;
    Equals,           // =
    Ampersand,        // &
    DoubleColon,      // ::
    Minus,            // -

    // comments
    LineComment,
    BlockComment,

    // special
    Whitespace,
    Unknown,
}


/// A token in the DSL with its kind, value, and source position
#[derive(Clone, PartialEq)]
pub(super) struct Token<'a> {
    /// The type of token
    pub kind: TokenKind,
    /// The text content of the token
    pub text: &'a str,
    /// The byte range in the source text
    pub span: Range<usize>,
}

impl<'a> Token<'a> {
    /// Creates a new token with the given kind, text, and span
    pub(super) fn new(
        kind: TokenKind,
        text: &'a str,
        span: Range<usize>
    ) -> Self {
        Self { kind, text, span }
    }
}

fn snake_case_str<'a>() -> impl StrParser<'a, &'a str>  {
    let start = item_if(|c: char| c.is_ascii_lowercase() || c == '_');
    let rest = many(item_if(|c: char| c.is_ascii_alphanumeric() || c == '_'), false, no_separator());
    get_parsed(right!(start, rest))
}

fn identifier<'a>() -> impl StrParser<'a, Token<'a>> {
    token(TokenKind::Identifier, snake_case_str())
}

fn keyword<'a>() -> impl StrParser<'a, Token<'a>> {
    let p = attempt(snake_case_str()
        .map_if(|s| Some(match s {
            "let"    => TokenKind::Keyword,
            "return" => TokenKind::Keyword,
            _        => None?,
        })));

    token(TokenKind::Keyword, get_parsed(p))
}

fn f32_literal<'a>() -> impl StrParser<'a, Token<'a>> {
    token(TokenKind::FloatLiteral, get_parsed(float::<f32, char, &str, ()>()))
}

fn int_literal<'a>() -> impl StrParser<'a, Token<'a>> {
    let sign = or!(skip!('-'), skip!('+'));
    let plain = many(item_if(|c: char| c.is_ascii_digit()), false, no_separator());
    token(TokenKind::IntLiteral, get_parsed(right!(sign, plain)))
}

fn hex_literal<'a>() -> impl StrParser<'a, Token<'a>> {
    let hex = right(skip!("0x"), item_while(|c: char| c.is_ascii_hexdigit()));
    token(TokenKind::HexLiteral, hex)
}

fn string_literal<'a>() -> impl StrParser<'a, Token<'a>> {
    let unicode = right(skip!('u'), times(4, item_if(|c: char| c.is_ascii_hexdigit())));
    let escaped = right(skip!('\\'), or_diff(unicode, item_if(|c: char| "\"\\/bfnrt".contains(c))));
    let valid_char = item_if(|c: char| c != '"' && c != '\\' && !c.is_control());
    let not_end = or_diff(valid_char, escaped);

    let string_literal = middle(skip!('"'), many(not_end, true, no_separator()), skip!('"'));
    token(TokenKind::StringLiteral, string_literal)
}

fn line_comment<'a>() -> impl StrParser<'a, Token<'a>> {
    let line_comment = right!(skip!("//"), item_while(|c: char| c != '\n'));
    token(TokenKind::LineComment, line_comment)
}

fn block_comment<'a>() -> impl StrParser<'a, Token<'a>> {
    let block_comment = right!(skip!("/*"), until("*/"));
    token(TokenKind::BlockComment, block_comment)
}

fn token<'a>(
    kind: TokenKind,
    p: impl StrParser<'a, &'a str>,
) -> impl StrParser<'a, Token<'a>> {
    count_consumed(p)
        .map(move |(c, s): (_, &str)| Token::new(kind, s, 0..c))
}