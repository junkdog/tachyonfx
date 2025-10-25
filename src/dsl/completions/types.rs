use crate::dsl::tokenizer::{Token, TokenKind};

/// Macro for compact token pattern matching.
///
/// This macro is used throughout the completion engine for pattern matching on tokens.
/// It provides three matching forms:
/// - `tok!(Keyword == "let")` - Match a specific token kind with exact text
/// - `tok!(Identifier => name)` - Match a token kind and bind the text
/// - `tok!(Dot)` - Match just the token kind
macro_rules! tok {
    // Match token kind with exact text: tok!(Keyword == "let")
    ($kind:ident == $text:literal) => {
        $crate::dsl::tokenizer::Token {
            kind: $crate::dsl::tokenizer::TokenKind::$kind,
            text: $text,
            ..
        }
    };
    // Match token kind and bind text: tok!(Identifier => name)
    ($kind:ident => $binding:ident) => {
        $crate::dsl::tokenizer::Token {
            kind: $crate::dsl::tokenizer::TokenKind::$kind,
            text: $binding,
            ..
        }
    };
    // Match token kind only: tok!(Dot)
    ($kind:ident) => {
        $crate::dsl::tokenizer::Token { kind: $crate::dsl::tokenizer::TokenKind::$kind, .. }
    };
}

pub(super) use tok;

#[derive(Debug, Clone)]
pub struct Completion {
    pub label: String,
    pub kind: CompletionKind,
    pub meta: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompletionKind {
    Method,
    Constructor,
    Function,
    Constant,
    Parameter,
    Type,
    Field,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CompletionContext {
    TopLevel,
    DotAccess { receiver_type: String },
    FnCall { fn_name: String, arg_index: usize },
    DoubleColon { namespace: String },
    StructInit { struct_name: String, filled_fields: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LetBinding {
    pub(super) name: String,
    pub(super) binding_type: String,
}

impl LetBinding {
    pub(super) fn new(name: &str, binding_type: &str) -> Self {
        Self {
            name: name.to_string(),
            binding_type: binding_type.to_string(),
        }
    }
}

pub(super) enum TokenCursor {
    /// Cursor is inside a token at the given character offset
    InToken { token_index: usize, offset: usize },
    /// Cursor is between tokens
    BetweenTokens,
}

impl TokenCursor {
    pub(super) fn from_tokens(tokens: &[Token<'_>], cursor_char_idx: u32) -> Self {
        let mut token_index = tokens
            .iter()
            .position(|t| t.contains_index(cursor_char_idx));

        // if not found, check if cursor is exactly at the end of an identifier token
        // (for completion purposes, being at the end of an identifier means we're still
        // completing it)
        if token_index.is_none() {
            token_index = tokens.iter().position(|t| {
                t.span.1 == cursor_char_idx && matches!(t.kind, TokenKind::Identifier)
            });
        }

        if let Some(idx) = token_index {
            let offset = cursor_char_idx.saturating_sub(tokens[idx].span.0) as usize;
            Self::InToken { token_index: idx, offset }
        } else {
            Self::BetweenTokens
        }
    }

    pub(super) fn extract_partial_token(&self, tokens: &[Token]) -> String {
        match self {
            TokenCursor::InToken { token_index, offset } => {
                let token = tokens[*token_index];
                if matches!(token.kind, TokenKind::Identifier) {
                    token.text.chars().take(*offset).collect()
                } else {
                    String::new()
                }
            },
            TokenCursor::BetweenTokens => String::new(),
        }
    }

    pub(super) fn token_index(&self) -> Option<usize> {
        match self {
            Self::InToken { token_index, .. } => Some(*token_index),
            Self::BetweenTokens => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum CallableItem {
    Constructor {
        name: &'static str,
        params: &'static [&'static str],
        #[allow(dead_code)]
        declaring_type: &'static str,
    },
    #[allow(dead_code)]
    StaticMethod {
        name: &'static str,
        params: &'static [&'static str],
        declaring_type: &'static str,
    },
    InstanceMethod {
        name: &'static str,
        params: &'static [&'static str],
        #[allow(dead_code)]
        declaring_type: &'static str,
    },
}

impl CallableItem {
    pub(super) const fn constructor(
        declaring_type: &'static str,
        name: &'static str,
        params: &'static [&'static str],
    ) -> Self {
        Self::Constructor { name, params, declaring_type }
    }

    #[allow(dead_code)]
    pub(super) const fn static_method(
        declaring_type: &'static str,
        name: &'static str,
        params: &'static [&'static str],
    ) -> Self {
        Self::StaticMethod { name, params, declaring_type }
    }

    pub(super) const fn instance_method(
        declaring_type: &'static str,
        name: &'static str,
        params: &'static [&'static str],
    ) -> Self {
        Self::InstanceMethod { name, params, declaring_type }
    }

    pub(super) const fn name(&self) -> &str {
        match self {
            Self::Constructor { name, .. }
            | Self::StaticMethod { name, .. }
            | Self::InstanceMethod { name, .. } => name,
        }
    }

    pub(super) const fn params(&self) -> &[&'static str] {
        match self {
            Self::Constructor { params, .. }
            | Self::StaticMethod { params, .. }
            | Self::InstanceMethod { params, .. } => params,
        }
    }

    #[allow(dead_code)]
    pub(super) const fn declaring_type(&self) -> &str {
        match self {
            Self::Constructor { declaring_type, .. }
            | Self::StaticMethod { declaring_type, .. }
            | Self::InstanceMethod { declaring_type, .. } => declaring_type,
        }
    }

    pub(super) const fn is_static(&self) -> bool {
        matches!(self, Self::Constructor { .. } | Self::StaticMethod { .. })
    }

    pub(super) const fn is_instance(&self) -> bool {
        matches!(self, Self::InstanceMethod { .. })
    }
}
