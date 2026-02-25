use alloc::collections::BTreeMap;

use super::types::{tok, CompletionContext, TokenCursor};
use crate::dsl::tokenizer::{Token, TokenKind};

pub(super) fn analyze_last_tokens(
    tokens: &[Token],
    cursor: &TokenCursor,
    effect_fns: &BTreeMap<&str, &str>,
) -> CompletionContext {
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

        // Pattern: ).  (method chain after function call, cursor after dot)
        [.., tok!(RightParen), tok!(Dot)] => {
            // The closing paren is at cursor_token_idx - 2 (before the dot)
            let closing_paren_idx = cursor_token_idx - 2;
            let paren_idx = find_matching_opening_paren(tokens, closing_paren_idx).unwrap_or(0);

            CompletionContext::DotAccess {
                receiver_type: infer_return_type(tokens, paren_idx, effect_fns),
            }
        },

        // Pattern: ).identifier  (method chain after function call, cursor in identifier)
        [.., tok!(RightParen), tok!(Dot), tok!(Identifier)] => {
            // The closing paren is at cursor_token_idx - 3 (before the dot and identifier)
            let closing_paren_idx = cursor_token_idx - 3;
            let paren_idx = find_matching_opening_paren(tokens, closing_paren_idx).unwrap_or(0);

            CompletionContext::DotAccess {
                receiver_type: infer_return_type(tokens, paren_idx, effect_fns),
            }
        },

        // Pattern: Namespace::  (e.g., "fx::" or "Color::")
        [.., tok!(Identifier => ns), tok!(DoubleColon)] => {
            CompletionContext::DoubleColon { namespace: ns.to_string() }
        },

        // Pattern: Namespace::partial  (e.g., "Color::Red" or "Interpolation::Quad")
        [.., tok!(Identifier => ns), tok!(DoubleColon), tok!(Identifier)] => {
            CompletionContext::DoubleColon { namespace: ns.to_string() }
        },

        // Pattern: Namespace::function_name(  (e.g., "WaveLayer::new(")
        [.., tok!(Identifier => ns), tok!(DoubleColon), tok!(Identifier => fn_name), tok!(LeftParen)] => {
            CompletionContext::FnCall {
                fn_name: fn_name.to_string(),
                namespace: Some(ns.to_string()),
                arg_index: 0,
            }
        },

        // Pattern: function_name(  (e.g., "fade_to(")
        [.., tok!(Identifier => fn_name), tok!(LeftParen)] => CompletionContext::FnCall {
            fn_name: fn_name.to_string(),
            namespace: None,
            arg_index: 0,
        },

        // Pattern: function_name(arg1, arg2,  (count commas for arg index)
        _tokens_slice => {
            // Find the last unclosed opening paren by walking backwards and tracking depth
            let mut paren_idx: Option<usize> = None;
            let mut depth = 0;

            for (idx, token) in tokens[..cursor_token_idx]
                .iter()
                .enumerate()
                .rev()
            {
                match token.kind {
                    TokenKind::RightParen => depth += 1,
                    TokenKind::LeftParen => {
                        if depth == 0 {
                            // Found an unclosed opening paren
                            paren_idx = Some(idx);
                            break;
                        }
                        depth -= 1;
                    },
                    _ => {},
                }
            }

            // If we found an unclosed paren, treat it as a function call
            if let Some(paren_idx) = paren_idx {
                // Count commas after the paren to determine argument index
                // Only count commas at depth 1 (direct arguments, not nested in brackets/parens)
                let mut comma_count = 0;
                let mut depth = 1;
                for token in &tokens[paren_idx + 1..cursor_token_idx] {
                    match token.kind {
                        TokenKind::LeftParen | TokenKind::LeftBracket => depth += 1,
                        TokenKind::RightParen | TokenKind::RightBracket => depth -= 1,
                        TokenKind::Comma if depth == 1 => comma_count += 1,
                        _ => {},
                    }
                }

                // Try to find the function name (and optional namespace) before the paren
                if paren_idx > 0 {
                    if let Some(tok!(Identifier => fn_name)) = tokens.get(paren_idx - 1) {
                        let namespace = if paren_idx >= 3 {
                            match (&tokens[paren_idx - 3], &tokens[paren_idx - 2]) {
                                (tok!(Identifier => ns), tok!(DoubleColon)) => Some(ns.to_string()),
                                _ => None,
                            }
                        } else {
                            None
                        };

                        return CompletionContext::FnCall {
                            fn_name: fn_name.to_string(),
                            namespace,
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
                // Check if this brace is closed - count brace depth from the opening brace
                let mut depth = 1;
                for token in &tokens[brace_idx + 1..cursor_token_idx] {
                    match token.kind {
                        TokenKind::LeftBrace => depth += 1,
                        TokenKind::RightBrace => {
                            depth -= 1;
                            if depth == 0 {
                                // This opening brace is closed, not inside struct init
                                break;
                            }
                        },
                        _ => {},
                    }
                }

                // Only treat as struct init if brace is still open (depth > 0)
                if depth > 0 {
                    // Look for the struct name before the brace
                    if brace_idx > 0 {
                        if let Some(tok!(Identifier => struct_name)) = tokens.get(brace_idx - 1) {
                            // Collect already filled fields (identifiers before colons after the
                            // brace)
                            let filled_fields = tokens[brace_idx..cursor_token_idx]
                                .windows(2)
                                .filter_map(|w| match w {
                                    [tok!(Identifier => field), tok!(Colon)] => {
                                        Some(field.to_string())
                                    },
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
            }

            // Default to top level context
            CompletionContext::TopLevel
        },
    }
}

/// Find the matching opening paren for a closing paren at the given index.
/// Scans backwards from closing_paren_idx tracking paren depth.
fn find_matching_opening_paren(tokens: &[Token], closing_paren_idx: usize) -> Option<usize> {
    let mut depth = 1;

    for (idx, token) in tokens[..closing_paren_idx]
        .iter()
        .enumerate()
        .rev()
    {
        match token.kind {
            TokenKind::RightParen => depth += 1,
            TokenKind::LeftParen => {
                depth -= 1;
                if depth == 0 {
                    return Some(idx);
                }
            },
            _ => {},
        }
    }

    None
}

/// Try to infer the return type from a function call by looking at the namespace
/// e.g., Color::from_u32(...) returns Color, fx::dissolve(...) returns Effect
fn infer_return_type(
    tokens: &[Token],
    paren_idx: usize,
    effect_fns: &BTreeMap<&str, &str>,
) -> String {
    // Check the 3 tokens immediately before the opening paren for Namespace::function pattern
    if paren_idx >= 3 {
        if let [tok!(Identifier => namespace), tok!(DoubleColon), tok!(Identifier)] =
            &tokens[paren_idx - 3..paren_idx]
        {
            // Map common namespaces to their types
            return match *namespace {
                "fx" => "Effect",
                other => other, // Color, Layout, Style, etc. use their namespace as type
            }
            .to_string();
        }
    }

    // If not a qualified call, check if it's a method chain (pattern: ).method()
    // Walk backwards through the method chain to find the original qualified call
    if paren_idx >= 2 {
        if let [tok!(Dot), tok!(Identifier)] = &tokens[paren_idx - 2..paren_idx] {
            // This is a method call - find the closing paren before the dot
            if paren_idx >= 3 {
                if let tok!(RightParen) = tokens[paren_idx - 3] {
                    // Find the matching opening paren and recurse
                    if let Some(matching_paren) = find_matching_opening_paren(tokens, paren_idx - 3)
                    {
                        return infer_return_type(tokens, matching_paren, effect_fns);
                    }
                }
            }
        }
    }

    // Check for unqualified function call (pattern: function_name()
    if paren_idx >= 1 {
        if let tok!(Identifier => fn_name) = &tokens[paren_idx - 1] {
            return effect_fns
                .get(fn_name)
                .copied()
                .unwrap_or("<Unknown>")
                .to_string();
        }
    }

    "<Unknown>".to_string()
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;
    use crate::dsl::tokenizer::{sanitize_tokens, tokenize};

    fn assert_context_eq(input: &str, expected: &CompletionContext) {
        fn analyze(input: &str) -> CompletionContext {
            let tokens = tokenize(input).map(sanitize_tokens).unwrap();
            let cursor = TokenCursor::from_tokens(&tokens, input.len() as _);

            // Common effect functions for testing
            let effect_fns: BTreeMap<&str, &str> = BTreeMap::from([
                ("fade_from_fg", "Effect"),
                ("fade_to", "Effect"),
                ("dissolve", "Effect"),
                ("sweep_in", "Effect"),
                ("sweep_out", "Effect"),
                ("slide_in", "Effect"),
                ("slide_out", "Effect"),
                ("coalesce", "Effect"),
                ("paint", "Effect"),
                ("saturate", "Effect"),
                ("lighten", "Effect"),
                ("darken", "Effect"),
                ("hsl_shift", "Effect"),
            ])
            .into_iter()
            .collect();

            analyze_last_tokens(&tokens, &cursor, &effect_fns)
        }

        let cursor_index = input
            .char_indices()
            .position(|(_, c)| c == '^')
            .unwrap_or(input.len());

        let ctx = analyze(&input[..cursor_index]);
        assert_eq!(&ctx, expected, "For input: {input}");
    }

    #[test]
    fn test_top_level_context() {
        assert_context_eq("", &CompletionContext::TopLevel);
        assert_context_eq("fx", &CompletionContext::TopLevel);

        // cursor is after semicolon, hence top-level
        assert_context_eq(
            "let c = Color::from_u32(0x1d2021);",
            &CompletionContext::TopLevel,
        );
    }

    #[test]
    fn test_dot_access() {
        assert_context_eq("a.", &CompletionContext::DotAccess {
            receiver_type: "a".to_string(),
        });
        assert_context_eq("effect.", &CompletionContext::DotAccess {
            receiver_type: "effect".to_string(),
        });
        assert_context_eq("a.clon", &CompletionContext::DotAccess {
            receiver_type: "a".to_string(),
        });
        assert_context_eq("effect.with_cell", &CompletionContext::DotAccess {
            receiver_type: "effect".to_string(),
        });

        assert_context_eq(
            "Color::from_u32(0x1d2021).",
            &CompletionContext::DotAccess { receiver_type: "Color".to_string() },
        );

        assert_context_eq(
            indoc! {"
                fx::fade_from(bg, bg, 1000). ^ // chevron is cursor pos
                    .with_color_space(ColorSpace::Rgb)
            "},
            &CompletionContext::DotAccess { receiver_type: "Effect".to_string() },
        );
    }

    #[test]
    fn test_double_colon() {
        assert_context_eq("fx::", &CompletionContext::DoubleColon {
            namespace: "fx".to_string(),
        });

        assert_context_eq("Color::", &CompletionContext::DoubleColon {
            namespace: "Color".to_string(),
        });

        assert_context_eq(
            indoc! {"
                fx::^
                fx::fade_from(Black, Black, 1000)
            "},
            &CompletionContext::DoubleColon { namespace: "fx".to_string() },
        );
    }

    #[test]
    fn test_function_call_no_args() {
        assert_context_eq("fade_to(", &CompletionContext::FnCall {
            fn_name: "fade_to".to_string(),
            namespace: None,
            arg_index: 0,
        });
    }

    #[test]
    fn test_function_call_with_args() {
        assert_context_eq("fade_to(Color::Red,", &CompletionContext::FnCall {
            fn_name: "fade_to".to_string(),
            namespace: None,
            arg_index: 1,
        });

        assert_context_eq("dissolve(500, CircOut,", &CompletionContext::FnCall {
            fn_name: "dissolve".to_string(),
            namespace: None,
            arg_index: 2,
        });
    }

    #[test]
    fn test_struct_init() {
        assert_context_eq("Rect {", &CompletionContext::StructInit {
            struct_name: "Rect".to_string(),
            filled_fields: vec![],
        });

        assert_context_eq("Rect { x: 0,", &CompletionContext::StructInit {
            struct_name: "Rect".to_string(),
            filled_fields: vec!["x".to_string()],
        });

        assert_context_eq("Rect { x: 0, y: 5,", &CompletionContext::StructInit {
            struct_name: "Rect".to_string(),
            filled_fields: vec!["x".to_string(), "y".to_string()],
        });

        assert_context_eq("Yolo { foo: 0, ba", &CompletionContext::StructInit {
            struct_name: "Yolo".to_string(),
            filled_fields: vec!["foo".to_string()],
        });
    }

    #[test]
    fn test_nested_function_calls() {
        // When cursor is inside nested call, should detect the innermost context
        assert_context_eq("outer(inner(", &CompletionContext::FnCall {
            fn_name: "inner".to_string(),
            namespace: None,
            arg_index: 0,
        });
    }

    #[test]
    fn test_method_chain() {
        // Method chains infer return type from the namespace

        // fx:: functions return Effect
        assert_context_eq("fx::dissolve(500).with_", &CompletionContext::DotAccess {
            receiver_type: "Effect".to_string(),
        });

        // Color:: functions return Color
        assert_context_eq(
            "Color::from_u32(0xff0000).",
            &CompletionContext::DotAccess { receiver_type: "Color".to_string() },
        );

        // Layout:: functions return Layout
        assert_context_eq("Layout::horizontal([]).", &CompletionContext::DotAccess {
            receiver_type: "Layout".to_string(),
        });

        // Style:: functions return Style
        assert_context_eq("Style::new().", &CompletionContext::DotAccess {
            receiver_type: "Style".to_string(),
        });

        // Non-qualified function calls default to "Chained"
        assert_context_eq("some_function().", &CompletionContext::DotAccess {
            receiver_type: "<Unknown>".to_string(),
        });

        let src = indoc! {"
            fx::fade_from_fg(Black, 1000)
                .with_filter(CellFilter::All)
                .with_pattern(DissolvePattern::default())
                .with_color_space(ColorSpace::Rgb)
                .re
        "};

        assert_context_eq(src, &CompletionContext::DotAccess {
            receiver_type: "Effect".to_string(),
        });

        let src = indoc! {"
            fade_from_fg(Black, 1000)
                .with_filter(CellFilter::All)
                .with_pattern(DissolvePattern::default())
                .with_color_space(ColorSpace::Rgb)
                .re
        "};

        assert_context_eq(src, &CompletionContext::DotAccess {
            receiver_type: "Effect".to_string(),
        });
    }

    #[test]
    fn test_qualified_function_call() {
        assert_context_eq("Color::from_u32(", &CompletionContext::FnCall {
            fn_name: "from_u32".to_string(),
            namespace: Some("Color".to_string()),
            arg_index: 0,
        });
    }

    #[test]
    fn test_nested_function_calls_infer_type() {
        // Nested function calls - should infer from the outer call, not the inner
        assert_context_eq(
            "fx::sequence(&[fx::dissolve(500)]).",
            &CompletionContext::DotAccess { receiver_type: "Effect".to_string() },
        );

        // Multiple levels of nesting
        assert_context_eq(
            "fx::parallel(&[fx::sequence(&[fx::dissolve(500)])]).",
            &CompletionContext::DotAccess { receiver_type: "Effect".to_string() },
        );
    }

    #[test]
    fn test_nested_effects_inside_slice() {
        assert_context_eq("fx::sequence(&[", &CompletionContext::FnCall {
            fn_name: "sequence".to_string(),
            namespace: Some("fx".to_string()),
            arg_index: 0,
        });

        assert_context_eq(
            "fx::sequence(&[fx::dissolve(500), ",
            &CompletionContext::FnCall {
                fn_name: "sequence".to_string(),
                namespace: Some("fx".to_string()),
                arg_index: 0,
            },
        );

        assert_context_eq(
            "fx::sequence(&[fx::dissolve(500), fx::",
            &CompletionContext::DoubleColon { namespace: "fx".to_string() },
        );

        assert_context_eq(
            "fx::sequence(&[fx::dissolve(500), fx::consume_tick()]).",
            &CompletionContext::DotAccess { receiver_type: "Effect".to_string() },
        );
    }

    #[test]
    fn test_extract_partial_token() {
        let tokens = tokenize("Motion::Left").unwrap();
        let tokens = sanitize_tokens(tokens);

        // Cursor at end of "Left"
        let cursor_pos = tokens.last().unwrap().span.1;
        let cursor = TokenCursor::from_tokens(&tokens, cursor_pos);
        let partial = cursor.extract_partial_token(&tokens);
        assert_eq!(partial, "Left");

        // Cursor in middle of "Left" (after "Le")
        let cursor_pos = tokens.last().unwrap().span.0 + 2;
        let cursor = TokenCursor::from_tokens(&tokens, cursor_pos);
        let partial = cursor.extract_partial_token(&tokens);
        assert_eq!(partial, "Le");
    }
}
