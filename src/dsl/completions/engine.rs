use std::collections::HashMap;

use super::{
    context::{analyze_last_tokens, extract_partial_token},
    dsl_type::{all_constants, all_constructors, all_methods},
    matcher::CompletionMatcher,
    types::{
        tok, CallableItem, Completion, CompletionContext, CompletionKind, LetBinding, TokenCursor,
    },
};
use crate::dsl::{tokenizer::Token, EffectDsl};

#[derive(Debug, Clone)]
pub struct CompletionEngine {
    effect_types: Vec<&'static str>,
    constructors: HashMap<&'static str, &'static [CallableItem]>,
    methods: HashMap<&'static str, &'static [CallableItem]>,
    constants: HashMap<&'static str, &'static [&'static str]>,
}

impl From<&CallableItem> for Completion {
    fn from(callable: &CallableItem) -> Self {
        Completion {
            label: callable.name().to_string(),
            kind: if callable.is_static() {
                CompletionKind::Function
            } else {
                CompletionKind::Method
            },
            meta: Some(callable.params().join(", ")),
        }
    }
}

impl CompletionEngine {
    pub fn new() -> Self {
        let methods = all_methods();
        let constructors = all_constructors();
        let constants = all_constants();
        let effect_types = EffectDsl::new().registered_effects();

        Self { methods, constructors, constants, effect_types }
    }

    fn const_completions(&self, identifier: &str) -> Vec<Completion> {
        self.constants
            .get(identifier)
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|name| Completion {
                label: name.to_string(),
                kind: CompletionKind::Constant,
                meta: Some(identifier.to_string()),
            })
            .collect()
    }

    fn constructor_completions(&self, identifier: &str) -> Vec<Completion> {
        self.constructors
            .get(identifier)
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(Completion::from)
            .collect()
    }

    fn method_completions(&self, identifier: &str) -> Vec<Completion> {
        self.methods
            .get(identifier)
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(Completion::from)
            .collect()
    }

    /// Provides completions for the given source string at the specified cursor position.
    /// Handles tokenization and sanitization internally.
    ///
    /// # Arguments
    /// * `source` - The source code string
    /// * `cursor_index` - Byte offset of the cursor in the source string
    ///
    /// # Returns
    /// A vector of completions sorted by relevance
    pub fn complete_source(&self, source: &str, cursor_index: u32) -> Vec<Completion> {
        use crate::dsl::tokenizer::{sanitize_tokens, tokenize};

        tokenize(source)
            .map(sanitize_tokens)
            .map(|tokens| self.completions(&tokens, cursor_index))
            .unwrap_or_else(|_| vec![])
    }

    /// Low-level completion function that works with pre-tokenized input.
    /// For internal use and testing. External users should use `complete_source` instead.
    pub(super) fn completions(&self, tokens: &[Token], cursor_index: u32) -> Vec<Completion> {
        let cursor = TokenCursor::from_tokens(tokens, cursor_index);

        // Extract partial token at cursor for filtering
        let partial = extract_partial_token(tokens, &cursor);
        let matcher = CompletionMatcher::new(partial);

        let context = analyze_last_tokens(tokens, &cursor);
        let _let_bindings = self.extract_let_bindings(tokens);

        let completions = match context {
            CompletionContext::TopLevel => {
                // Top-level completions: namespaces and types
                vec![
                    Completion {
                        label: "fx::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Effect constructors".to_string()),
                    },
                    Completion {
                        label: "Color::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Color constructors".to_string()),
                    },
                    Completion {
                        label: "Layout::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Layout constructors".to_string()),
                    },
                    Completion {
                        label: "Style::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Style constructors".to_string()),
                    },
                    Completion {
                        label: "CellFilter::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Cell filter constructors".to_string()),
                    },
                    Completion {
                        label: "Rect::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Rect constructors".to_string()),
                    },
                    Completion {
                        label: "Duration::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Duration constructors".to_string()),
                    },
                    Completion {
                        label: "EffectTimer::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Timer constructors".to_string()),
                    },
                    Completion {
                        label: "Margin::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Margin constructors".to_string()),
                    },
                    Completion {
                        label: "Constraint::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Constraint constructors".to_string()),
                    },
                    Completion {
                        label: "RepeatMode::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Repeat mode constructors".to_string()),
                    },
                    Completion {
                        label: "RefRect::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("RefRect constructors".to_string()),
                    },
                    Completion {
                        label: "Size::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Size constructors".to_string()),
                    },
                    // Pattern types
                    Completion {
                        label: "CheckerboardPattern::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Checkerboard pattern constructors".to_string()),
                    },
                    Completion {
                        label: "CoalescePattern::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Coalesce pattern constructors".to_string()),
                    },
                    Completion {
                        label: "DiagonalPattern::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Diagonal pattern constructors".to_string()),
                    },
                    Completion {
                        label: "DissolvePattern::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Dissolve pattern constructors".to_string()),
                    },
                    Completion {
                        label: "RadialPattern::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Radial pattern constructors".to_string()),
                    },
                    Completion {
                        label: "SweepPattern::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Sweep pattern constructors".to_string()),
                    },
                    Completion {
                        label: "Interpolation::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Animation easing functions".to_string()),
                    },
                    Completion {
                        label: "Motion::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Movement directions".to_string()),
                    },
                    Completion {
                        label: "ColorSpace::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Color interpolation spaces".to_string()),
                    },
                    Completion {
                        label: "Direction::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Layout directions".to_string()),
                    },
                    Completion {
                        label: "Flex::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Flex layout modes".to_string()),
                    },
                    Completion {
                        label: "ExpandDirection::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Expansion directions".to_string()),
                    },
                    Completion {
                        label: "Modifier::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Cell style modifiers".to_string()),
                    },
                    Completion {
                        label: "RepeatMode::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Effect repeat modes".to_string()),
                    },
                    Completion {
                        label: "EvolveSymbolSet::".to_string(),
                        kind: CompletionKind::Type,
                        meta: Some("Symbol sets for evolve effect".to_string()),
                    },
                ]
            },

            CompletionContext::DotAccess { receiver_type } => {
                // Instance method completions for the receiver type
                self.method_completions(receiver_type.as_str())
            },

            CompletionContext::DoubleColon { namespace } => {
                // Static method/constructor completions for the namespace
                match namespace.as_str() {
                    "fx" => self
                        .effect_types
                        .iter()
                        .map(|effect_name| Completion {
                            label: effect_name.to_string(),
                            kind: CompletionKind::Function,
                            meta: Some(format!("{}(...)", effect_name)),
                        })
                        .collect(),
                    ns => [self.const_completions(ns), self.constructor_completions(ns)].concat(),
                }
            },

            CompletionContext::FnCall { fn_name, arg_index } => {
                // Argument type hints based on function signature
                // Look up the function in methods and return type hint for the specific argument
                for items in self.methods.values() {
                    if let Some(item) = items.iter().find(|i| i.name() == fn_name) {
                        if let Some(arg_type) = item.params().get(arg_index) {
                            return vec![Completion {
                                label: format!("<{}>", arg_type),
                                kind: CompletionKind::Variable,
                                meta: Some(format!("Parameter {} of {}", arg_index + 1, fn_name)),
                            }];
                        }
                    }
                }
                vec![]
            },

            CompletionContext::StructInit { struct_name, filled_fields } => {
                // Field completions for struct initialization
                // Return fields that haven't been filled yet
                let all_fields: Vec<(&str, &str)> = match struct_name.as_str() {
                    "Rect" => vec![("x", "u16"), ("y", "u16"), ("width", "u16"), ("height", "u16")],
                    "Size" => vec![("width", "u16"), ("height", "u16")],
                    "Offset" => vec![("x", "i32"), ("y", "i32")],
                    _ => vec![],
                };

                all_fields
                    .into_iter()
                    .filter(|(field, _)| !filled_fields.iter().any(|f| f == field))
                    .map(|(field, field_type)| Completion {
                        label: format!("{}: ", field),
                        kind: CompletionKind::Field,
                        meta: Some(field_type.to_string()),
                    })
                    .collect()
            },
        };

        // Filter and score completions based on partial input
        matcher.filter_and_score(completions)
    }

    #[allow(clippy::needless_return)]
    fn extract_let_bindings(&self, tokens: &[Token]) -> Vec<LetBinding> {
        let mut seen = std::collections::HashSet::new();

        tokens
            .windows(5)
            .filter_map(|window| {
                match window {
                    // `let <name> = fx::`
                    [
                        tok!(Keyword == "let"),
                        tok!(Identifier => name),
                        tok!(Equals),
                        tok!(Identifier == "fx"),
                        tok!(DoubleColon),
                    ] => Some(LetBinding::new(name, "Effect")),

                    // `let <name> = <type>::`
                    [
                        tok!(Keyword == "let"),
                        tok!(Identifier => name),
                        tok!(Equals),
                        tok!(Identifier => binding_type),
                        tok!(DoubleColon),
                    ] => Some(LetBinding::new(name, binding_type)),

                    // `let <name> = <fn_call>(`
                    [
                        tok!(Keyword == "let"),
                        tok!(Identifier => name),
                        tok!(Equals),
                        tok!(Identifier => binding_type),
                        tok!(LeftParen),
                    ] => self.resolve_shortform_fns(binding_type).map(|t| LetBinding::new(name, t)),

                    // `let <name> = (<timer>` - matches (duration, interpolation) tuples
                    [
                        tok!(Keyword == "let"),
                        tok!(Identifier => name),
                        tok!(Equals),
                        tok!(LeftParen),
                        _, // Can be IntLiteral, Identifier, or other expression
                    ] => Some(LetBinding::new(name, "EffectTimer")),

                    // `let <name> = <identifier>` - bare constant assignment
                    [
                        tok!(Keyword == "let"),
                        tok!(Identifier => name),
                        tok!(Equals),
                        tok!(Identifier => binding_type),
                        ..,
                    ] => self.resolve_shortform_constants(binding_type).map(|t| LetBinding::new(name, t)),

                    _ => None,
                }
            })
            .filter(|binding| seen.insert(binding.name.clone()))
            .collect()
    }

    fn resolve_shortform_fns(&self, identifier: &str) -> Option<&'static str> {
        let cell_filter_constants = self
            .constants
            .get("CellFilter")
            .copied()
            .unwrap_or(&[]);
        Some(match () {
            _ if cell_filter_constants.contains(&identifier) => "CellFilter",
            _ if self.effect_types.contains(&identifier) => "Effect",
            _ => None?,
        })
    }

    fn resolve_shortform_constants(&self, identifier: &str) -> Option<&'static str> {
        self.constants
            .iter()
            .find(|(_, &v)| v.contains(&identifier))
            .map(|(&k, _)| k)
    }
}

impl Default for CompletionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::tokenizer::{sanitize_tokens, tokenize};

    #[test]
    fn test_completion_engine_top_level() {
        let engine = CompletionEngine::new();
        let tokens = tokenize("").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, 0);

        assert!(completions.iter().any(|c| c.label == "fx::"));
        assert!(completions.iter().any(|c| c.label == "Color::"));
        assert!(completions.iter().any(|c| c.label == "Rect::"));
    }

    #[test]
    fn test_completion_engine_double_colon() {
        let engine = CompletionEngine::new();
        let tokens = tokenize("Color::").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, tokens.last().unwrap().span.1);

        // Color:: now returns both constants (17) and constructors (3) = 20 total
        assert_eq!(completions.len(), 20);
        assert!(completions.iter().any(|c| c.label == "from_u32"));
        assert!(completions.iter().any(|c| c.label == "Rgb"));
        assert!(completions.iter().any(|c| c.label == "Indexed"));
        // Also has color constants
        assert!(completions.iter().any(|c| c.label == "Red"));
        assert!(completions.iter().any(|c| c.label == "Blue"));
    }

    #[test]
    fn test_completion_engine_double_colon_2() {
        let engine = CompletionEngine::new();
        let tokens = tokenize("Interpolation::Quad").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, tokens.last().unwrap().span.1);

        // Should filter to Quad* interpolations based on partial "Quad"
        assert_eq!(completions.len(), 3);
        assert!(completions.iter().any(|c| c.label == "QuadIn"));
        assert!(completions.iter().any(|c| c.label == "QuadOut"));
        assert!(completions.iter().any(|c| c.label == "QuadInOut"));
    }

    #[test]
    fn test_completion_engine_dot_access() {
        let engine = CompletionEngine::new();
        let tokens = tokenize("rect.").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, tokens.last().unwrap().span.1);

        // Even though "rect" is unknown, we infer from the pattern
        // The completion should suggest Rect methods since receiver_type is "rect"
        // But our HashMap uses "Rect" not "rect", so this test shows the limitation
        // In practice, you'd need type inference or the user to use qualified names
        assert!(completions.is_empty() || completions.iter().any(|c| c.label == "clone"));
    }

    #[test]
    fn test_completion_engine_method_chain() {
        let engine = CompletionEngine::new();
        let tokens = tokenize("Rect::new(0, 0, 10, 10).").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, tokens.last().unwrap().span.1);

        // After Rect::new(...), we should get Rect instance methods (not constructors)
        assert_eq!(completions.len(), 6);
        assert!(completions.iter().any(|c| c.label == "clone"));
        assert!(completions.iter().any(|c| c.label == "inner"));
        assert!(completions
            .iter()
            .any(|c| c.label == "intersection"));
        // Constructor "new" should NOT appear in instance method context
        assert!(!completions.iter().any(|c| c.label == "new"));
    }

    #[test]
    fn test_completion_engine_struct_init() {
        let engine = CompletionEngine::new();
        let tokens = tokenize("Rect { x: 0, ").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, tokens.last().unwrap().span.1);

        // Should suggest remaining fields
        assert_eq!(completions.len(), 3);
        assert!(!completions.iter().any(|c| c.label.contains("x:"))); // x already filled
        assert!(completions.iter().any(|c| c.label.contains("y:")));
        assert!(completions
            .iter()
            .any(|c| c.label.contains("width:")));
        assert!(completions
            .iter()
            .any(|c| c.label.contains("height:")));
    }

    #[test]
    fn test_completion_engine_fx_effects() {
        let engine = CompletionEngine::new();
        let tokens = tokenize("fx::").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, tokens.last().unwrap().span.1);

        // Should return all registered effects
        assert!(
            !completions.is_empty(),
            "Should have at least some effect completions"
        );

        // Check for some common effects that we know exist
        assert!(
            completions.iter().any(|c| c.label == "dissolve"),
            "dissolve effect should be available"
        );
        assert!(
            completions.iter().any(|c| c.label == "fade_to"),
            "fade_to effect should be available"
        );
        assert!(
            completions.iter().any(|c| c.label == "sweep_in"),
            "sweep_in effect should be available"
        );

        // All completions should be functions
        assert!(
            completions
                .iter()
                .all(|c| c.kind == CompletionKind::Function),
            "All fx:: completions should be functions"
        );

        // Verify meta is present
        assert!(
            completions.iter().all(|c| c.meta.is_some()),
            "All completions should have meta information"
        );
    }

    #[test]
    fn test_completion_engine_interpolations() {
        let engine = CompletionEngine::new();
        let tokens = tokenize("Interpolation::").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, tokens.last().unwrap().span.1);

        // Should return all interpolation types
        assert_eq!(completions.len(), 32, "Should have 32 interpolation types");

        // Check for some common interpolations
        assert!(
            completions.iter().any(|c| c.label == "Linear"),
            "Linear interpolation should be available"
        );
        assert!(
            completions.iter().any(|c| c.label == "QuadIn"),
            "QuadIn interpolation should be available"
        );
        assert!(
            completions.iter().any(|c| c.label == "QuadOut"),
            "QuadOut interpolation should be available"
        );
        assert!(
            completions.iter().any(|c| c.label == "BounceOut"),
            "BounceOut interpolation should be available"
        );
        assert!(
            completions.iter().any(|c| c.label == "ElasticIn"),
            "ElasticIn interpolation should be available"
        );

        // All completions should be constants
        assert!(
            completions
                .iter()
                .all(|c| c.kind == CompletionKind::Constant),
            "All Interpolation:: completions should be constants"
        );

        // Verify meta is present
        assert!(
            completions.iter().all(|c| c.meta.is_some()),
            "All completions should have meta information"
        );
    }

    #[test]
    fn test_completion_engine_top_level_includes_interpolation() {
        let engine = CompletionEngine::new();
        let tokens = tokenize("").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, 0);

        assert!(
            completions
                .iter()
                .any(|c| c.label == "Interpolation::"),
            "Top-level should include Interpolation namespace"
        );
        assert!(
            completions.iter().any(|c| c.label == "Motion::"),
            "Top-level should include Motion namespace"
        );
        assert!(
            completions
                .iter()
                .any(|c| c.label == "Modifier::"),
            "Top-level should include Modifier namespace"
        );
    }

    #[test]
    fn test_completion_engine_enum_constants() {
        let engine = CompletionEngine::new();

        // Test Motion
        let tokens = tokenize("Motion::").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, tokens.last().unwrap().span.1);
        assert_eq!(completions.len(), 4);
        assert!(completions
            .iter()
            .any(|c| c.label == "LeftToRight"));
        assert!(completions
            .iter()
            .all(|c| c.kind == CompletionKind::Constant));

        // Test Direction
        let tokens = tokenize("Direction::").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, tokens.last().unwrap().span.1);
        assert_eq!(completions.len(), 2);
        assert!(completions
            .iter()
            .any(|c| c.label == "Horizontal"));
        assert!(completions.iter().any(|c| c.label == "Vertical"));

        // Test Modifier
        let tokens = tokenize("Modifier::").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, tokens.last().unwrap().span.1);
        assert_eq!(completions.len(), 9);
        assert!(completions.iter().any(|c| c.label == "BOLD"));
        assert!(completions.iter().any(|c| c.label == "ITALIC"));

        // Test ColorSpace
        let tokens = tokenize("ColorSpace::").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, tokens.last().unwrap().span.1);
        assert_eq!(completions.len(), 3);
        assert!(completions.iter().any(|c| c.label == "Rgb"));
        assert!(completions.iter().any(|c| c.label == "Hsl"));
        assert!(completions.iter().any(|c| c.label == "Hsv"));
    }

    #[test]
    fn test_completion_engine_color_mixed() {
        let engine = CompletionEngine::new();
        let tokens = tokenize("Color::").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, tokens.last().unwrap().span.1);

        // Color should have both constants and constructors
        assert!(
            completions.len() > 17,
            "Should have both constants and constructors"
        );

        // Check for color constants
        assert!(completions
            .iter()
            .any(|c| c.label == "Red" && c.kind == CompletionKind::Constant));
        assert!(completions
            .iter()
            .any(|c| c.label == "Blue" && c.kind == CompletionKind::Constant));

        // Check for color constructors
        assert!(completions
            .iter()
            .any(|c| c.label == "from_u32" && c.kind == CompletionKind::Function));
        assert!(completions
            .iter()
            .any(|c| c.label == "Rgb" && c.kind == CompletionKind::Function));
    }

    #[test]
    fn test_completion_engine_cell_filter_mixed() {
        let engine = CompletionEngine::new();
        let tokens = tokenize("CellFilter::").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, tokens.last().unwrap().span.1);

        // CellFilter should have both constants and constructors
        assert!(
            completions.len() > 2,
            "Should have both constants and constructors"
        );

        // Check for constants
        assert!(completions
            .iter()
            .any(|c| c.label == "All" && c.kind == CompletionKind::Constant));
        assert!(completions
            .iter()
            .any(|c| c.label == "Text" && c.kind == CompletionKind::Constant));

        // Check for constructors
        assert!(completions
            .iter()
            .any(|c| c.label == "Area" && c.kind == CompletionKind::Function));
        assert!(completions
            .iter()
            .any(|c| c.label == "FgColor" && c.kind == CompletionKind::Function));
    }

    #[test]
    fn test_completion_with_partial_input() {
        let engine = CompletionEngine::new();

        // Test "fade" partial in "Interpolation::fade"
        let tokens = tokenize("Interpolation::Quad").unwrap();
        let tokens = sanitize_tokens(tokens);
        let completions = engine.completions(&tokens, tokens.last().unwrap().span.1);

        // Should filter to Quad* interpolations
        assert_eq!(completions.len(), 3);
        assert!(
            completions
                .iter()
                .all(|c| c.label.to_lowercase().contains("quad")),
            "All results should contain 'quad'"
        );

        // QuadIn/Out/InOut should be at the top (prefix matches)
        assert!(completions[0].label.starts_with("Quad"));
    }

    #[test]
    fn test_complete_source() {
        let engine = CompletionEngine::new();

        // Test basic completion from source string
        let source = "fx::";
        let completions = engine.complete_source(source, source.len() as u32);

        // Should get fx:: effect completions
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.label == "dissolve"));

        // Test completion with partial input
        let source = "Color::Re";
        let completions = engine.complete_source(source, source.len() as u32);

        // Should filter to colors starting with "Re"
        assert!(completions.iter().any(|c| c.label == "Red"));
        assert!(completions.iter().any(|c| c.label == "Reset"));
        assert!(!completions.iter().any(|c| c.label == "Blue"));
    }

    #[test]
    fn test_complete_source_tokenization_error() {
        let engine = CompletionEngine::new();

        // Test with invalid source that would cause tokenization error
        let source = "fx:: \"unterminated string";
        let completions = engine.complete_source(source, source.len() as u32);

        // Should return empty on tokenization error
        assert!(completions.is_empty(), "was: {:?}", completions);
    }

    #[test]
    fn test_declared_variables_complex_types() {
        let engine = CompletionEngine::new();

        let source = r#"
            let color = Red;
            let effect = fx::consume_tick();
            let effect_b = dissolve(500);
            let interpolation = Interpolation::QuadOut;
            let motion = LeftToRight;
            let rect = Rect::new();
            let timer = EffectTimer::new();
            let timer_b = (1000, Linear);
        "#;
        let tokens = tokenize(source).map(sanitize_tokens).unwrap();
        let bindings = engine.extract_let_bindings(&tokens);

        assert_eq!(bindings, &[
            LetBinding::new("color", "Color"),
            LetBinding::new("effect", "Effect"),
            LetBinding::new("effect_b", "Effect"),
            LetBinding::new("interpolation", "Interpolation"),
            LetBinding::new("motion", "Motion"),
            LetBinding::new("rect", "Rect"),
            LetBinding::new("timer", "EffectTimer"),
            LetBinding::new("timer_b", "EffectTimer"),
        ]);
    }

    #[test]
    fn test_resolve_shortforms() {
        let engine = CompletionEngine::new();

        // Test effect types
        assert_eq!(engine.resolve_shortform_fns("dissolve"), Some("Effect"));
        assert_eq!(engine.resolve_shortform_fns("fade_to"), Some("Effect"));
        assert_eq!(engine.resolve_shortform_fns("sweep_in"), Some("Effect"));

        // Test cell filter constants
        assert_eq!(engine.resolve_shortform_fns("All"), Some("CellFilter"));
        assert_eq!(engine.resolve_shortform_fns("Text"), Some("CellFilter"));

        // Test non-matching identifier
        assert_eq!(engine.resolve_shortform_fns("unknown"), None);
        assert_eq!(engine.resolve_shortform_fns("Red"), None); // Color constant, not a function

        // Test color constants
        assert_eq!(engine.resolve_shortform_constants("Red"), Some("Color"));
        assert_eq!(engine.resolve_shortform_constants("Blue"), Some("Color"));
        assert_eq!(
            engine.resolve_shortform_constants("LightGreen"),
            Some("Color")
        );

        // Test color spaces
        assert_eq!(
            engine.resolve_shortform_constants("Rgb"),
            Some("ColorSpace")
        );
        assert_eq!(
            engine.resolve_shortform_constants("Hsl"),
            Some("ColorSpace")
        );
        assert_eq!(
            engine.resolve_shortform_constants("Hsv"),
            Some("ColorSpace")
        );

        // Test interpolations
        assert_eq!(
            engine.resolve_shortform_constants("Linear"),
            Some("Interpolation")
        );
        assert_eq!(
            engine.resolve_shortform_constants("QuadOut"),
            Some("Interpolation")
        );
        assert_eq!(
            engine.resolve_shortform_constants("BounceIn"),
            Some("Interpolation")
        );

        // Test motions
        assert_eq!(
            engine.resolve_shortform_constants("LeftToRight"),
            Some("Motion")
        );
        assert_eq!(
            engine.resolve_shortform_constants("UpToDown"),
            Some("Motion")
        );

        // Test directions (note: Horizontal and Vertical are ambiguous between Direction and
        // ExpandDirection) This ambiguity is acceptable and will be handled elsewhere
        let horizontal_result = engine.resolve_shortform_constants("Horizontal");
        assert!(
            horizontal_result == Some("Direction") || horizontal_result == Some("ExpandDirection")
        );
        let vertical_result = engine.resolve_shortform_constants("Vertical");
        assert!(vertical_result == Some("Direction") || vertical_result == Some("ExpandDirection"));

        // Test flexes
        assert_eq!(engine.resolve_shortform_constants("Center"), Some("Flex"));
        assert_eq!(
            engine.resolve_shortform_constants("SpaceBetween"),
            Some("Flex")
        );

        // Test modifiers
        assert_eq!(engine.resolve_shortform_constants("BOLD"), Some("Modifier"));
        assert_eq!(
            engine.resolve_shortform_constants("ITALIC"),
            Some("Modifier")
        );

        // Test repeat modes
        assert_eq!(
            engine.resolve_shortform_constants("Forever"),
            Some("RepeatMode")
        );

        // Test evolve symbol sets
        assert_eq!(
            engine.resolve_shortform_constants("Circles"),
            Some("EvolveSymbolSet")
        );
        assert_eq!(
            engine.resolve_shortform_constants("BlocksHorizontal"),
            Some("EvolveSymbolSet")
        );

        // Test cell filter constants
        assert_eq!(
            engine.resolve_shortform_constants("All"),
            Some("CellFilter")
        );
        assert_eq!(
            engine.resolve_shortform_constants("Text"),
            Some("CellFilter")
        );

        // Test non-matching identifier
        assert_eq!(engine.resolve_shortform_constants("unknown"), None);
    }

    #[test]
    fn test_extract_let_bindings_with_shortform_fns() {
        let engine = CompletionEngine::new();

        // Test effect function call
        let source = "let effect = dissolve(500);";
        let tokens = tokenize(source).map(sanitize_tokens).unwrap();
        let bindings = engine.extract_let_bindings(&tokens);

        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0], LetBinding::new("effect", "Effect"));

        // Test cell filter function call
        let source = "let filter = All(rect);";
        let tokens = tokenize(source).map(sanitize_tokens).unwrap();
        let bindings = engine.extract_let_bindings(&tokens);

        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0], LetBinding::new("filter", "CellFilter"));
    }
}
