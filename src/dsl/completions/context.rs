use super::types::{tok, CompletionContext, TokenCursor};
use crate::dsl::tokenizer::{Token, TokenKind};

/// Extracts the partial token at the cursor position for completion matching.
pub(super) fn extract_partial_token(tokens: &[Token], cursor: &TokenCursor) -> String {
    match cursor {
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

/// Try to infer the return type from a function call by looking at the namespace
/// e.g., Color::from_u32(...) returns Color, fx::dissolve(...) returns Effect
fn infer_return_type(tokens: &[Token], paren_idx: usize) -> String {
    // Look backwards from the opening paren to find Namespace::function pattern
    if paren_idx >= 3 {
        if let [.., tok!(Identifier => namespace), tok!(DoubleColon), tok!(Identifier)] =
            &tokens[paren_idx.saturating_sub(3)..paren_idx]
        {
            // Map common namespaces to their types
            return match *namespace {
                "fx" => "Effect",
                other => other, // Color, Layout, Style, etc. use their namespace as type
            }
            .to_string();
        }
    }

    // Default to generic chained type
    String::from("Chained")
}

pub(super) fn analyze_last_tokens(tokens: &[Token], cursor: &TokenCursor) -> CompletionContext {
    // Find the token at or before the cursor
    let cursor_token_idx = cursor.token_index().unwrap_or(tokens.len());

    // Pattern match on the last few tokens
    match &tokens[..cursor_token_idx] {
        // Pattern: identifier.  (e.g., "foo.")
        [.., tok!(Identifier => obj), tok!(Dot)] => {
            CompletionContext::DotAccess { receiver_type: obj.to_string() }
        },

        // Pattern: identifier.identifier  (e.g., "foo.bar")
        [.., tok!(Identifier => obj), tok!(Dot), tok!(Identifier)] => {
            CompletionContext::DotAccess { receiver_type: obj.to_string() }
        },

        // Pattern: ).identifier  (method chain after function call)
        [.., tok!(RightParen), tok!(Dot)] | [.., tok!(RightParen), tok!(Dot), tok!(Identifier)] => {
            // Find the matching opening paren to infer return type
            let paren_idx = tokens[..cursor_token_idx]
                .iter()
                .rposition(|t| t.kind == TokenKind::LeftParen)
                .unwrap_or(0);

            CompletionContext::DotAccess {
                receiver_type: infer_return_type(tokens, paren_idx),
            }
        },

        // Pattern: ].identifier  (method chain after array index)
        [.., tok!(RightBracket), tok!(Dot)]
        | [.., tok!(RightBracket), tok!(Dot), tok!(Identifier)] => {
            CompletionContext::DotAccess { receiver_type: String::from("Array") }
        },

        // Pattern: Namespace::  (e.g., "fx::" or "Color::")
        [.., tok!(Identifier => ns), tok!(DoubleColon)] => {
            CompletionContext::DoubleColon { namespace: ns.to_string() }
        },

        // Pattern: Namespace::partial  (e.g., "Color::Red" or "Interpolation::Quad")
        [.., tok!(Identifier => ns), tok!(DoubleColon), tok!(Identifier)] => {
            CompletionContext::DoubleColon { namespace: ns.to_string() }
        },

        // Pattern: function_name(  (e.g., "fade_to(")
        [.., tok!(Identifier => fn_name), tok!(LeftParen)] => {
            CompletionContext::FnCall { fn_name: fn_name.to_string(), arg_index: 0 }
        },

        // Pattern: function_name(arg1, arg2,  (count commas for arg index)
        _tokens_slice => {
            // Check if we're inside a function call by finding the last opening paren
            // We need to search in ALL tokens, not just the slice, to handle complex cases
            if let Some(paren_idx) = tokens[..cursor_token_idx]
                .iter()
                .rposition(|t| t.kind == TokenKind::LeftParen)
            {
                // Count commas after the paren to determine argument index
                let comma_count = tokens[paren_idx..cursor_token_idx]
                    .iter()
                    .filter(|t| t.kind == TokenKind::Comma)
                    .count();

                // Try to find the function name before the paren
                if paren_idx > 0 {
                    if let Some(tok!(Identifier => fn_name)) = tokens.get(paren_idx - 1) {
                        return CompletionContext::FnCall {
                            fn_name: fn_name.to_string(),
                            arg_index: comma_count,
                        };
                    }
                }
            }

            // Check if we're inside a struct initialization
            if let Some(brace_idx) = tokens[..cursor_token_idx]
                .iter()
                .rposition(|t| t.kind == TokenKind::LeftBrace)
            {
                // Look for the struct name before the brace
                if brace_idx > 0 {
                    if let Some(tok!(Identifier => struct_name)) = tokens.get(brace_idx - 1) {
                        // Collect already filled fields (identifiers before colons after the brace)
                        let filled_fields = tokens[brace_idx..cursor_token_idx]
                            .windows(2)
                            .filter_map(|w| match w {
                                [tok!(Identifier => field), tok!(Colon)] => Some(field.to_string()),
                                _ => None,
                            })
                            .collect();

                        return CompletionContext::StructInit {
                            struct_name: struct_name.to_string(),
                            filled_fields,
                        };
                    }
                }
            }

            // Default to top level context
            CompletionContext::TopLevel
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::tokenizer::{sanitize_tokens, tokenize};

    /// Helper to tokenize input and analyze context at the end
    fn analyze(input: &str) -> CompletionContext {
        let tokens = tokenize(input).unwrap();
        let tokens = sanitize_tokens(tokens);
        let cursor = TokenCursor::from_tokens(&tokens, input.len() as _);
        analyze_last_tokens(&tokens, &cursor)
    }

    fn assert_context_eq(input: &str, expected: CompletionContext) {
        let ctx = analyze(input);
        assert_eq!(ctx, expected, "For input: {}", input);
    }

    #[test]
    fn test_top_level_context() {
        assert_context_eq("", CompletionContext::TopLevel);
        assert_context_eq("fx", CompletionContext::TopLevel);
    }

    #[test]
    fn test_dot_access() {
        assert_context_eq("a.", CompletionContext::DotAccess {
            receiver_type: "a".to_string(),
        });
        assert_context_eq("effect.", CompletionContext::DotAccess {
            receiver_type: "effect".to_string(),
        });
        assert_context_eq("a.clon", CompletionContext::DotAccess {
            receiver_type: "a".to_string(),
        });
        assert_context_eq("effect.with_cell", CompletionContext::DotAccess {
            receiver_type: "effect".to_string(),
        });
    }

    #[test]
    fn test_double_colon() {
        assert_context_eq("fx::", CompletionContext::DoubleColon {
            namespace: "fx".to_string(),
        });

        assert_context_eq("Color::", CompletionContext::DoubleColon {
            namespace: "Color".to_string(),
        });
    }

    #[test]
    fn test_function_call_no_args() {
        assert_context_eq("fade_to(", CompletionContext::FnCall {
            fn_name: "fade_to".to_string(),
            arg_index: 0,
        });
    }

    #[test]
    fn test_function_call_with_args() {
        assert_context_eq("fade_to(Color::Red,", CompletionContext::FnCall {
            fn_name: "fade_to".to_string(),
            arg_index: 1,
        });

        assert_context_eq("dissolve(500, CircOut,", CompletionContext::FnCall {
            fn_name: "dissolve".to_string(),
            arg_index: 2,
        });
    }

    #[test]
    fn test_struct_init() {
        assert_context_eq("Rect {", CompletionContext::StructInit {
            struct_name: "Rect".to_string(),
            filled_fields: vec![],
        });

        assert_context_eq("Rect { x: 0,", CompletionContext::StructInit {
            struct_name: "Rect".to_string(),
            filled_fields: vec!["x".to_string()],
        });

        assert_context_eq("Rect { x: 0, y: 5,", CompletionContext::StructInit {
            struct_name: "Rect".to_string(),
            filled_fields: vec!["x".to_string(), "y".to_string()],
        });

        assert_context_eq("Yolo { foo: 0, ba", CompletionContext::StructInit {
            struct_name: "Yolo".to_string(),
            filled_fields: vec!["foo".to_string()],
        });
    }

    #[test]
    fn test_nested_function_calls() {
        // When cursor is inside nested call, should detect the innermost context
        assert_context_eq("outer(inner(", CompletionContext::FnCall {
            fn_name: "inner".to_string(),
            arg_index: 0,
        });
    }

    #[test]
    fn test_method_chain() {
        // Method chains infer return type from the namespace

        // fx:: functions return Effect
        assert_context_eq("fx::dissolve(500).with_", CompletionContext::DotAccess {
            receiver_type: "Effect".to_string(),
        });

        // Color:: functions return Color
        assert_context_eq("Color::from_u32(0xff0000).", CompletionContext::DotAccess {
            receiver_type: "Color".to_string(),
        });

        // Layout:: functions return Layout
        assert_context_eq("Layout::horizontal([]).", CompletionContext::DotAccess {
            receiver_type: "Layout".to_string(),
        });

        // Style:: functions return Style
        assert_context_eq("Style::new().", CompletionContext::DotAccess {
            receiver_type: "Style".to_string(),
        });

        // Non-qualified function calls default to "Chained"
        assert_context_eq("some_function().", CompletionContext::DotAccess {
            receiver_type: "Chained".to_string(),
        });
    }

    #[test]
    fn test_qualified_function_call() {
        assert_context_eq("Color::from_u32(", CompletionContext::FnCall {
            fn_name: "from_u32".to_string(),
            arg_index: 0,
        });
    }

    #[test]
    fn test_extract_partial_token() {
        let tokens = tokenize("Motion::Left").unwrap();
        let tokens = sanitize_tokens(tokens);

        // Cursor at end of "Left"
        let cursor_pos = tokens.last().unwrap().span.1;
        let cursor = TokenCursor::from_tokens(&tokens, cursor_pos);
        let partial = extract_partial_token(&tokens, &cursor);
        assert_eq!(partial, "Left");

        // Cursor in middle of "Left" (after "Le")
        let cursor_pos = tokens.last().unwrap().span.0 + 2;
        let cursor = TokenCursor::from_tokens(&tokens, cursor_pos);
        let partial = extract_partial_token(&tokens, &cursor);
        assert_eq!(partial, "Le");
    }
}
