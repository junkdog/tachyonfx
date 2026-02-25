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

/// A completion suggestion for DSL code.
///
/// Represents a single item that can be inserted at the cursor position in DSL source
/// code. Following LSP (Language Server Protocol) conventions, each item has a label for
/// display, a kind indicating what type of completion it is, and detail text providing
/// additional information.
///
/// # Snippet Support
///
/// Functions and methods include snippet-style `insert_text` with `$0` marking the final
/// cursor position. For example, `dissolve` has insert text `"dissolve($0)"` to place the
/// cursor inside the parentheses.
///
/// # Examples
///
/// ```
/// use tachyonfx::dsl::{CompletionItem, CompletionKind};
///
/// // Function with snippet insertion
/// let item = CompletionItem {
///     label: "fade_to".to_string(),
///     kind: CompletionKind::Function,
///     detail: "fade_to_fg(Color, EffectTimer)".to_string(),
///     insert_text: Some("fade_to($0)".to_string()),
///     description: Some("Fade to specified foreground color".to_string()),
/// };
///
/// // Constant with plain insertion
/// let item = CompletionItem {
///     label: "Linear".to_string(),
///     kind: CompletionKind::Constant,
///     detail: "Interpolation".to_string(),
///     insert_text: None,
///     description: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    /// The text to be inserted when this completion is selected.
    ///
    /// This is the primary display text shown to the user and the text that will be
    /// inserted into the source code if `insert_text` is `None`.
    pub label: String,

    /// The category of this completion item.
    ///
    /// Indicates whether this is a method, function, type, constant, etc. Used for
    /// visual distinction and filtering in completion UIs.
    pub kind: CompletionKind,

    /// Additional information about this completion.
    ///
    /// Provides context about the completion such as function signatures, parameter
    /// descriptions, or type information. Always contains a value (never empty for
    /// valid completions from the engine).
    pub detail: String,

    /// Optional snippet-style insertion text with cursor placeholders.
    ///
    /// When present, this text should be inserted instead of `label`. Uses LSP snippet
    /// syntax with `$0` marking the final cursor position. Functions and methods include
    /// parentheses with the cursor positioned inside: `"function_name($0)"`.
    ///
    /// When `None`, the `label` should be inserted as-is.
    pub insert_text: Option<String>,

    /// Optional human-readable description of what this completion does.
    ///
    /// Provides a brief explanation of the effect, type, or method for display in
    /// completion UIs. When `None`, no description is available.
    pub description: Option<String>,
}

/// The category of a completion item.
///
/// Follows LSP conventions for completion item kinds. Used to provide visual hints
/// and enable filtering in completion UIs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionKind {
    /// An instance method that operates on an object (e.g., `rect.inner()`).
    Method,

    /// A standalone function, static method, or constructor (e.g., `fx::dissolve()`,
    /// `Rect::new()`).
    Function,

    /// A constant value (e.g., `Color::Red`, `Interpolation::Linear`).
    Constant,

    /// A function parameter hint showing expected type.
    Parameter,

    /// A variable defined with `let` binding in the DSL.
    Variable,

    /// A type or namespace (e.g., `Color::`, `fx::`).
    Type,

    /// A struct field in initialization syntax (e.g., `x:`, `width:`).
    Field,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CompletionContext {
    TopLevel,
    DotAccess {
        receiver_type: String,
    },
    FnCall {
        fn_name: String,
        namespace: Option<String>,
        arg_index: usize,
    },
    DoubleColon {
        namespace: String,
    },
    StructInit {
        struct_name: String,
        filled_fields: Vec<String>,
    },
}

impl CompletionItem {
    pub(super) fn new_type(label: &str, detail: &str) -> Self {
        Self {
            label: label.to_string(),
            kind: CompletionKind::Type,
            detail: detail.into(),
            insert_text: None,
            description: None,
        }
    }

    pub(super) fn new_param(param_type: &str, arg_index: usize, arg_count: usize) -> Self {
        CompletionItem {
            label: format!("{param_type}::"),
            kind: CompletionKind::Parameter,
            detail: format!("Parameter {} of {arg_count}", arg_index + 1),
            insert_text: None,
            description: None,
        }
    }
}

impl From<&CallableItem> for CompletionItem {
    fn from(callable: &CallableItem) -> Self {
        let name = callable.name();
        CompletionItem {
            label: name.to_string(),
            kind: if callable.is_static() {
                CompletionKind::Function
            } else {
                CompletionKind::Method
            },
            detail: format!("{}({})", name, callable.params().join(", ")),
            insert_text: Some(format!("{name}($0)")),
            description: callable
                .description()
                .map(alloc::string::ToString::to_string),
        }
    }
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
        description: Option<&'static str>,
    },
    #[allow(dead_code)]
    StaticMethod {
        name: &'static str,
        params: &'static [&'static str],
        declaring_type: &'static str,
        description: Option<&'static str>,
    },
    InstanceMethod {
        name: &'static str,
        params: &'static [&'static str],
        #[allow(dead_code)]
        declaring_type: &'static str,
        description: Option<&'static str>,
    },
}

impl CallableItem {
    pub(super) const fn constructor(
        declaring_type: &'static str,
        name: &'static str,
        params: &'static [&'static str],
        description: Option<&'static str>,
    ) -> Self {
        Self::Constructor { name, params, declaring_type, description }
    }

    #[allow(dead_code)]
    pub(super) const fn static_method(
        declaring_type: &'static str,
        name: &'static str,
        params: &'static [&'static str],
        description: Option<&'static str>,
    ) -> Self {
        Self::StaticMethod { name, params, declaring_type, description }
    }

    pub(super) const fn instance_method(
        declaring_type: &'static str,
        name: &'static str,
        params: &'static [&'static str],
        description: Option<&'static str>,
    ) -> Self {
        Self::InstanceMethod { name, params, declaring_type, description }
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

    pub(super) const fn description(&self) -> Option<&'static str> {
        match self {
            Self::Constructor { description, .. }
            | Self::StaticMethod { description, .. }
            | Self::InstanceMethod { description, .. } => *description,
        }
    }

    pub(super) const fn is_static(&self) -> bool {
        matches!(self, Self::Constructor { .. } | Self::StaticMethod { .. })
    }
}
