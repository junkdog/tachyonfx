use alloc::collections::BTreeMap;
use std::collections::HashMap;

use super::{
    context::analyze_last_tokens,
    dsl_type::{all_constants, all_constructors, all_methods},
    matcher::CompletionMatcher,
    types::{
        tok, CallableItem, CompletionContext, CompletionItem, CompletionKind, LetBinding,
        TokenCursor,
    },
};
use crate::dsl::{completions::dsl_type::effect_types, tokenizer::Token};

/// Provides intelligent code completion for the tachyonfx DSL.
///
/// The completion engine analyzes DSL source code and cursor position to suggest
/// context-appropriate completions. It understands:
///
/// - Type and Effect constructors and their parameters
/// - Method chaining on typed objects
/// - Struct field initialization
/// - Let bindings and variable references
/// - Constant values and enum variants
///
/// The engine uses fuzzy matching to filter completions as the user types.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use tachyonfx::dsl::CompletionEngine;
///
/// let engine = CompletionEngine::new();
///
/// // Complete at top level
/// let completions = engine.completions("fx::", 4);
/// // Returns all effect constructors: dissolve, fade_to, sweep_in, etc.
///
/// // Complete method chains
/// let completions = engine.completions("Rect::new(0, 0, 10, 10).", 24);
/// // Returns Rect instance methods: inner, intersection, union, etc.
///
/// // Complete with partial input
/// let completions = engine.completions("Color::R", 8);
/// // Returns: Red, Rgb, Reset (fuzzy matched)
/// ```
#[derive(Debug, Clone)]
pub struct CompletionEngine {
    effect_types: HashMap<&'static str, CallableItem>,
    constructors: HashMap<&'static str, &'static [CallableItem]>,
    methods: HashMap<&'static str, &'static [CallableItem]>,
    constants: HashMap<&'static str, &'static [&'static str]>,
}

impl CompletionEngine {
    /// Creates a new completion engine with all DSL types registered.
    ///
    /// The engine is pre-populated with:
    /// - tachyonfx effects (47 effects)
    /// - Ratatui types (Color, Layout, Rect, Style, etc.)
    /// - Enum constants (Interpolation, Motion, Direction, etc.)
    /// - Constructor and instance methods
    ///
    /// # Examples
    ///
    /// ```
    /// use tachyonfx::dsl::CompletionEngine;
    ///
    /// let engine = CompletionEngine::new();
    /// let completions = engine.completions("", 0);
    /// assert!(!completions.is_empty());
    /// ```
    pub fn new() -> Self {
        let methods = all_methods();
        let constructors = all_constructors();
        let constants = all_constants();
        let effect_types = effect_types();

        Self { methods, constructors, constants, effect_types }
    }

    /// Provides context-aware completions for DSL source code.
    ///
    /// Analyzes the source string up to the cursor position, determines the completion
    /// context (top-level, method chain, function call, etc.), and returns appropriate
    /// suggestions. Completions are filtered using fuzzy matching if the cursor is within
    /// or at the end of an identifier.
    ///
    /// # Arguments
    ///
    /// * `source` - The DSL source code string
    /// * `cursor_index` - Character offset of the cursor in the source string
    ///
    /// # Returns
    ///
    /// A vector of [`CompletionItem`]s sorted by relevance (best matches first).
    pub fn completions(&self, source: &str, cursor_index: u32) -> Vec<CompletionItem> {
        use crate::dsl::tokenizer::{sanitize_tokens, tokenize};

        if let Ok(tokens) = tokenize(&source[..cursor_index as usize]) {
            // note: it would be nice to handle more cases around strings
            // and comments, but it requires passing more context after
            // the current cursor position. maybe in the future.

            if matches!(&tokens.last(), Some(tok!(LineComment))) {
                // no completions: cursor inside line comment
                return vec![];
            }

            // removing whitespace and comments
            let tokens = sanitize_tokens(tokens);

            // no completions: cursor right after `let ` or `let <identifier>`
            if tokens.len() >= 2
                && tokens[tokens.len() - 2..]
                    .iter()
                    .any(|t| matches!(t, tok!(Keyword == "let")))
            {
                return vec![];
            }

            return self.completions_from_tokens(&tokens, cursor_index);
        }

        vec![]
    }

    pub fn echo_source(&self, source: &str, cursor_index: u32) -> String {
        source[..cursor_index as usize].to_string()
    }

    /// Low-level completion function that works with pre-tokenized input.
    /// For internal use and testing. External users should use `complete_source` instead.
    fn completions_from_tokens(&self, tokens: &[Token], cursor_index: u32) -> Vec<CompletionItem> {
        let mut context_lookup: BTreeMap<&str, &str> = self
            .effect_types
            .keys()
            .map(|e| (*e, "Effect"))
            .collect();
        let let_bindings = self.extract_let_bindings(tokens);
        for b in &let_bindings {
            context_lookup.insert(&b.name, &b.binding_type);
        }

        let cursor = TokenCursor::from_tokens(tokens, cursor_index);
        let context = analyze_last_tokens(tokens, &cursor, &context_lookup);

        let completions = match context {
            CompletionContext::TopLevel => {
                // Top-level completions: namespaces and types
                vec![
                    CompletionItem::new_type("fx::", "Effects"),
                    CompletionItem::new_type("Color::", "Color constructors"),
                    CompletionItem::new_type("Layout::", "Layout constructors"),
                    CompletionItem::new_type("Style::", "Style constructors"),
                    CompletionItem::new_type("CellFilter::", "Filter within area"),
                    CompletionItem::new_type("Rect::", "Rect constructors"),
                    CompletionItem::new_type("Duration::", "Duration constructors"),
                    CompletionItem::new_type("EffectTimer::", "Timer constructors"),
                    CompletionItem::new_type("Margin::", "Margin constructors"),
                    CompletionItem::new_type("Constraint::", "Constraint constructors"),
                    CompletionItem::new_type("RepeatMode::", "Repeat mode constructors"),
                    CompletionItem::new_type("RefRect::", "RefRect constructors"),
                    CompletionItem::new_type("Size::", "Size constructors"),
                    CompletionItem::new_type("SimpleRng::", "Random number generator"),
                    // Pattern types
                    CompletionItem::new_type("CheckerboardPattern::", "Checkerboard cell reveal"),
                    CompletionItem::new_type("CoalescePattern::", "Random cell reveal"),
                    CompletionItem::new_type("DiagonalPattern::", "Diagonal sweep reveal"),
                    CompletionItem::new_type("DissolvePattern::", "Random dissolve reveal"),
                    CompletionItem::new_type("RadialPattern::", "Radial outward reveal"),
                    CompletionItem::new_type("DiamondPattern::", "Diamond-shaped reveal"),
                    CompletionItem::new_type("SpiralPattern::", "Spiral arm reveal"),
                    CompletionItem::new_type("SweepPattern::", "Linear sweep reveal"),
                    CompletionItem::new_type("WavePattern::", "Wave interference pattern"),
                    CompletionItem::new_type(
                        "CombinedPattern::",
                        "Combine two patterns with an operation",
                    ),
                    CompletionItem::new_type("InvertedPattern::", "Invert pattern output"),
                    CompletionItem::new_type("BlendPattern::", "Blend between two patterns"),
                    // Wave types
                    CompletionItem::new_type("WaveLayer::", "Wave interference layer"),
                    CompletionItem::new_type("Oscillator::", "Trig oscillator"),
                    CompletionItem::new_type("Modulator::", "Oscillator modulation source"),
                    CompletionItem::new_type("Interpolation::", "Easing functions"),
                    CompletionItem::new_type("Motion::", "Movement directions"),
                    CompletionItem::new_type("ColorSpace::", "Color interpolation spaces"),
                    CompletionItem::new_type("Direction::", "Layout directions"),
                    CompletionItem::new_type("Flex::", "Flex layout modes"),
                    CompletionItem::new_type("ExpandDirection::", "Expansion directions"),
                    CompletionItem::new_type("Modifier::", "Cell style modifiers"),
                    CompletionItem::new_type("RepeatMode::", "Effect repeat modes"),
                    CompletionItem::new_type("EvolveSymbolSet::", "Symbol sets for evolve effects"),
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
                        .map(|(effect_name, ctor)| {
                            let meta = format!("{effect_name}({})", ctor.params().join(", "));
                            let insert_text =
                                if *effect_name == "sequence" || *effect_name == "parallel" {
                                    format!("{effect_name}(&[$0])")
                                } else {
                                    format!("{effect_name}($0)")
                                };

                            CompletionItem {
                                label: (*effect_name).to_string(),
                                kind: CompletionKind::Function,
                                detail: meta,
                                insert_text: Some(insert_text),
                                description: ctor
                                    .description()
                                    .map(alloc::string::ToString::to_string),
                            }
                        })
                        .collect(),
                    ns => [self.const_completions(ns), self.constructor_completions(ns)].concat(),
                }
            },

            CompletionContext::FnCall { fn_name, namespace, arg_index } => {
                let fn_name = fn_name.as_str();
                let completable = if let Some(ns) = &namespace {
                    self.constructor_by_type_and_name(ns, fn_name)
                        .or_else(|| self.effect_by_name(fn_name))
                        .or_else(|| self.method_by_name(fn_name))
                } else {
                    self.effect_by_name(fn_name)
                        .or_else(|| self.method_by_name(fn_name))
                        .or_else(|| self.constructor_by_name(fn_name))
                };

                // Now generate completions based on the parameter type
                let mut completions = vec![];
                if let Some(completable) = completable {
                    // Handle &[Effect] - suggest effect constructors directly
                    let arg_type = completable
                        .params()
                        .get(arg_index)
                        .copied()
                        .unwrap_or_default();
                    if arg_type == "&[Effect]" {
                        for (effect_name, ctor) in &self.effect_types {
                            // todo: use existing ctor
                            completions.push(CompletionItem {
                                label: effect_name.to_string(),
                                kind: CompletionKind::Function,
                                detail: format!("{effect_name}({})", ctor.params().join(", ")),
                                insert_text: Some(format!("{effect_name}()")),
                                description: ctor
                                    .description()
                                    .map(alloc::string::ToString::to_string),
                            });
                        }
                    } else {
                        let arg_count = completable.params().len();
                        completions.push(CompletionItem::new_param(arg_type, arg_index, arg_count));
                    }
                }

                completions
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
                    .map(|(field, field_type)| CompletionItem {
                        label: format!("{field}: "),
                        kind: CompletionKind::Field,
                        detail: field_type.to_string(),
                        insert_text: None,
                        description: None,
                    })
                    .collect()
            },
        };

        let mut completions = specialize_completions(completions);
        let types: Vec<String> = completions
            .iter()
            .map(|c| c.label.clone())
            .map(|c| match () {
                _ if c.ends_with("::") => c[0..c.len() - 2].to_string(),
                _ => c,
            })
            .collect();

        // add any matching let bindings to the completions
        let_bindings
            .into_iter()
            .filter(|binding| types.contains(&binding.binding_type))
            .map(|binding| CompletionItem {
                label: binding.name,
                kind: CompletionKind::Variable,
                detail: binding.binding_type,
                insert_text: None,
                description: None,
            })
            .for_each(|completion| completions.push(completion));

        // add any matching const completions
        types
            .iter()
            .flat_map(|t| self.const_completions(t))
            .for_each(|c| completions.push(c));

        // Filter and score completions based on partial input
        // Extract partial token at cursor for filtering
        let partial = cursor.extract_partial_token(tokens);
        CompletionMatcher::new(partial).filter_and_score(completions)
    }

    fn const_completions(&self, identifier: &str) -> Vec<CompletionItem> {
        self.constants
            .get(identifier)
            .copied()
            .unwrap_or_default()
            .iter()
            .map(|name| CompletionItem {
                label: name.to_string(),
                kind: CompletionKind::Constant,
                detail: identifier.to_string(),
                insert_text: None,
                description: None,
            })
            .collect()
    }

    fn constructor_completions(&self, identifier: &str) -> Vec<CompletionItem> {
        self.constructors
            .get(identifier)
            .copied()
            .unwrap_or_default()
            .iter()
            .map(CompletionItem::from)
            .collect()
    }

    fn method_completions(&self, identifier: &str) -> Vec<CompletionItem> {
        self.methods
            .get(identifier)
            .copied()
            .unwrap_or_default()
            .iter()
            .map(CompletionItem::from)
            .collect()
    }

    fn method_by_name(&self, fn_name: &str) -> Option<CallableItem> {
        self.methods
            .values()
            .flat_map(|ctors| ctors.iter())
            .find(|ctor| ctor.name() == fn_name)
            .copied()
    }

    fn constructor_by_type_and_name(&self, type_name: &str, fn_name: &str) -> Option<CallableItem> {
        self.constructors
            .get(type_name)
            .into_iter()
            .flat_map(|fns| fns.iter())
            .find(|f| f.name() == fn_name)
            .copied()
    }

    fn constructor_by_name(&self, fn_name: &str) -> Option<CallableItem> {
        self.constructors
            .values()
            .flat_map(|fns| fns.iter())
            .find(|f| f.name() == fn_name)
            .copied()
    }

    fn effect_by_name(&self, fn_name: &str) -> Option<CallableItem> {
        self.effect_types.get(fn_name).copied()
    }

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
            _ if self.effect_types.contains_key(identifier) => "Effect",
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

fn specialize_completions(completions: Vec<CompletionItem>) -> Vec<CompletionItem> {
    completions
        .into_iter()
        .flat_map(|c| match () {
            _ if c.label == "AnyPattern::" => vec![
                CompletionItem::new_type("BlendPattern::", "Blend between two patterns"),
                CompletionItem::new_type("CheckerboardPattern::", "Checkerboard cell reveal"),
                CompletionItem::new_type("CoalescePattern::", "Random cell reveal"),
                CompletionItem::new_type(
                    "CombinedPattern::",
                    "Combine two patterns with an operation",
                ),
                CompletionItem::new_type("DiagonalPattern::", "Diagonal sweep reveal"),
                CompletionItem::new_type("DiamondPattern::", "Diamond-shaped reveal"),
                CompletionItem::new_type("DissolvePattern::", "Random dissolve reveal"),
                CompletionItem::new_type("InvertedPattern::", "Invert pattern output"),
                CompletionItem::new_type("RadialPattern::", "Radial outward reveal"),
                CompletionItem::new_type("SpiralPattern::", "Spiral arm reveal"),
                CompletionItem::new_type("SweepPattern::", "Linear sweep reveal"),
                CompletionItem::new_type("WavePattern::", "Wave interference pattern"),
            ],
            _ if c.label.starts_with("bool") => vec![
                CompletionItem::new_type("true", &c.detail),
                CompletionItem::new_type("false", &c.detail),
            ],
            _ if c.label.starts_with("u16") => {
                vec![CompletionItem::new_type("<u16>", &c.detail)]
            },
            _ if c.label.starts_with("u32") => {
                vec![CompletionItem::new_type("<u32>", &c.detail)]
            },
            _ if c.label.starts_with("f32") => {
                vec![CompletionItem::new_type("<f32>", &c.detail)]
            },
            _ => vec![c],
        })
        .collect()
}

impl Default for CompletionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeSet;

    use indoc::indoc;

    use super::*;
    use crate::{
        dsl::{
            completions::dsl_type::DslType,
            tokenizer::{sanitize_tokens, tokenize},
        },
        Effect,
    };

    #[test]
    fn test_completion_engine_top_level() {
        let engine = CompletionEngine::new();
        let completions = engine.completions("", 0);

        assert!(completions.iter().any(|c| c.label == "fx::"));
        assert!(completions.iter().any(|c| c.label == "Color::"));
        assert!(completions.iter().any(|c| c.label == "Rect::"));
    }

    #[test]
    fn test_completion_engine_double_colon() {
        let engine = CompletionEngine::new();
        let source = "Color::";
        let completions = engine.completions(source, source.len() as u32);

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
        let source = "Interpolation::Quad";
        let completions = engine.completions(source, source.len() as u32);

        // Should filter to Quad* interpolations based on partial "Quad"
        assert_eq!(completions.len(), 3);
        assert!(completions.iter().any(|c| c.label == "QuadIn"));
        assert!(completions.iter().any(|c| c.label == "QuadOut"));
        assert!(completions.iter().any(|c| c.label == "QuadInOut"));
    }

    #[test]
    fn test_completion_engine_method_chain() {
        let engine = CompletionEngine::new();
        let source = "Rect::new(0, 0, 10, 10).";
        let completions = engine.completions(source, source.len() as u32);

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
        let source = "Rect { x: 0, ";
        let completions = engine.completions(source, source.len() as u32);

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
        let source = "fx::";
        let completions = engine.completions(source, source.len() as u32);

        // Should return exactly 47 effect completions (all registered effects)
        assert_eq!(
            completions.len(),
            47,
            "Should have exactly 47 fx effect completions"
        );

        // All completions should be functions
        assert!(
            completions
                .iter()
                .all(|c| c.kind == CompletionKind::Function),
            "All fx:: completions should be functions"
        );

        // All completions should have detail information
        assert!(
            completions.iter().all(|c| !c.detail.is_empty()),
            "All completions should have detail information"
        );

        // Verify all completion labels are from the effect_types registry
        let effect_names: Vec<&str> = engine.effect_types.keys().copied().collect();
        for completion in &completions {
            assert!(
                effect_names.contains(&completion.label.as_str()),
                "Completion '{}' should be in effect_types registry",
                completion.label
            );
        }

        // Verify no duplicates
        let labels: Vec<_> = completions.iter().map(|c| &c.label).collect();
        let unique_labels: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(
            labels.len(),
            unique_labels.len(),
            "Should have no duplicate completions"
        );
    }

    #[test]
    fn test_completion_engine_interpolations() {
        let engine = CompletionEngine::new();
        let source = "Interpolation::";
        let completions = engine.completions(source, source.len() as u32);

        // Should return all interpolation types
        assert_eq!(completions.len(), 34, "Should have 34 interpolation types");

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

        // Verify detail is present
        assert!(
            completions.iter().all(|c| !c.detail.is_empty()),
            "All completions should have detail information"
        );
    }

    #[test]
    fn test_completing_parameters() {
        let engine = CompletionEngine::new();

        // Test EffectTimer::from_ms(u32, Interpolation) - second parameter
        let src = "EffectTimer::from_ms(1000, ";
        let completions = engine.completions(src, src.len() as u32);

        assert!(
            !completions.is_empty(),
            "Should have completions for Interpolation parameter"
        );
        assert_eq!(completions[0], CompletionItem {
            label: "Interpolation::".to_string(),
            kind: CompletionKind::Parameter,
            detail: "Parameter 2 of 2".to_string(),
            insert_text: None,
            description: None,
        });

        // Test CellFilter::Inner(Margin) - first parameter
        let src = "CellFilter::Inner(";
        let completions = engine.completions(src, src.len() as u32);

        assert!(
            !completions.is_empty(),
            "Should have completions for Margin parameter"
        );
        assert_eq!(completions[0], CompletionItem {
            label: "Margin::".to_string(),
            kind: CompletionKind::Parameter,
            detail: "Parameter 1 of 1".to_string(),
            insert_text: None,
            description: None,
        });
    }

    #[test]
    fn test_completion_engine_top_level_includes_interpolation() {
        let engine = CompletionEngine::new();
        let completions = engine.completions("", 0);

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
        let source = "Motion::";
        let completions = engine.completions(source, source.len() as u32);
        assert_eq!(completions.len(), 4);
        assert!(completions
            .iter()
            .any(|c| c.label == "LeftToRight"));
        assert!(completions
            .iter()
            .all(|c| c.kind == CompletionKind::Constant));

        // Test Direction
        let source = "Direction::";
        let completions = engine.completions(source, source.len() as u32);
        assert_eq!(completions.len(), 2);
        assert!(completions
            .iter()
            .any(|c| c.label == "Horizontal"));
        assert!(completions.iter().any(|c| c.label == "Vertical"));

        // Test Modifier
        let source = "Modifier::";
        let completions = engine.completions(source, source.len() as u32);
        assert_eq!(completions.len(), 9);
        assert!(completions.iter().any(|c| c.label == "BOLD"));
        assert!(completions.iter().any(|c| c.label == "ITALIC"));

        // Test ColorSpace
        let source = "ColorSpace::";
        let completions = engine.completions(source, source.len() as u32);
        assert_eq!(completions.len(), 3);
        assert!(completions.iter().any(|c| c.label == "Rgb"));
        assert!(completions.iter().any(|c| c.label == "Hsl"));
        assert!(completions.iter().any(|c| c.label == "Hsv"));
    }

    #[test]
    fn test_completion_engine_color_mixed() {
        let engine = CompletionEngine::new();
        let source = "Color::";
        let completions = engine.completions(source, source.len() as u32);

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
        let source = "CellFilter::";
        let completions = engine.completions(source, source.len() as u32);

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

        // Test "Quad" partial in "Interpolation::Quad"
        let source = "Interpolation::Quad";
        let completions = engine.completions(source, source.len() as u32);

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
        let completions = engine.completions(source, source.len() as u32);

        // Should get fx:: effect completions
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.label == "dissolve"));

        // Test completion with partial input
        let source = "Color::Re";
        let completions = engine.completions(source, source.len() as u32);

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
        let completions = engine.completions(source, source.len() as u32);

        // Should return empty on tokenization error
        assert!(completions.is_empty(), "was: {completions:?}");
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

    #[test]
    fn test_complete_let_bindings() {
        let engine = CompletionEngine::new();
        let source = indoc! {r#"
            let screen_bg = Color::Red;
            let screen_bg = Color::from_u32(0x1d2021);
            fx::fade_to(screen_bg, s
        "#};

        let completions = engine.completions(source, source.chars().count() as u32);

        assert_eq!(completions[..2], vec![
            CompletionItem {
                label: "Color::".to_string(),
                kind: CompletionKind::Parameter,
                detail: "Parameter 2 of 3".to_string(),
                insert_text: None,
                description: None,
            },
            CompletionItem {
                label: "screen_bg".to_string(),
                kind: CompletionKind::Variable,
                detail: "Color".to_string(),
                insert_text: None,
                description: None,
            }
        ]);

        // we should also get a bunch of color constants
        assert!(completions[2..]
            .iter()
            .all(|c| c.detail == "Color"));
        assert!(completions[2..]
            .iter()
            .all(|c| c.kind == CompletionKind::Constant));
    }

    #[test]
    fn test_complete_dot_access_after_ctor() {
        let engine = CompletionEngine::new();

        let source = "fx::consume_tick().";

        let completions = engine.completions(source, source.len() as u32);
        println!("{completions:?}");
        assert_eq!(completions.len(), Effect::methods().len());
    }

    #[test]
    fn test_effect_with_pattern_completion() {
        let engine = CompletionEngine::new();
        let source = indoc! {r#"
            fx::consume_tick().with_pattern(
        "#};

        let completions = engine.completions(source, source.len() as u32);

        // Expected pattern types (all concrete pattern types)
        let expected_patterns = vec![
            "BlendPattern::",
            "CheckerboardPattern::",
            "CoalescePattern::",
            "CombinedPattern::",
            "DiagonalPattern::",
            "DiamondPattern::",
            "DissolvePattern::",
            "InvertedPattern::",
            "RadialPattern::",
            "SpiralPattern::",
            "SweepPattern::",
            "WavePattern::",
        ];

        let mut actual_patterns = completions
            .iter()
            .filter(|c| c.label.contains("Pattern"))
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>();

        actual_patterns.sort();

        assert_eq!(expected_patterns, actual_patterns);
    }

    #[test]
    fn test_insert_text_for_functions_and_methods() {
        let engine = CompletionEngine::new();

        // Functions should have insert_text with $0
        let completions = engine.completions("fx::", 4);
        let dissolve = completions
            .iter()
            .find(|c| c.label == "dissolve")
            .unwrap();
        assert_eq!(
            dissolve.insert_text,
            Some("dissolve($0)".to_string()),
            "Functions should have insert_text with cursor placeholder"
        );

        // Methods should have insert_text with $0
        let completions = engine.completions("fx::dissolve(500).", 18);
        let with_duration = completions
            .iter()
            .find(|c| c.label == "with_duration")
            .unwrap();
        assert_eq!(
            with_duration.insert_text,
            Some("with_duration($0)".to_string()),
            "Methods should have insert_text with cursor placeholder"
        );

        // Constructors (treated as Function kind) should have insert_text
        let completions = engine.completions("Color::", 7);
        let rgb = completions
            .iter()
            .find(|c| c.label == "Rgb")
            .unwrap();
        assert_eq!(
            rgb.insert_text,
            Some("Rgb($0)".to_string()),
            "Constructors should have insert_text with cursor placeholder"
        );

        // Constants should NOT have insert_text
        let red = completions
            .iter()
            .find(|c| c.label == "Red")
            .unwrap();
        assert_eq!(
            red.insert_text, None,
            "Constants should not have insert_text"
        );

        // Types/namespaces should NOT have insert_text
        let completions = engine.completions("", 0);
        let fx_namespace = completions
            .iter()
            .find(|c| c.label == "fx::")
            .unwrap();
        assert_eq!(
            fx_namespace.insert_text, None,
            "Type/namespace completions should not have insert_text"
        );

        // Variables should NOT have insert_text
        let source = "let my_color = Color::Red; fx::fade_to(my";
        let completions = engine.completions(source, source.len() as u32);
        let my_color = completions
            .iter()
            .find(|c| c.label == "my_color")
            .unwrap();
        assert_eq!(
            my_color.insert_text, None,
            "Variable completions should not have insert_text"
        );
    }

    #[test]
    fn test_completing_sequence_and_parallel() {
        let engine = CompletionEngine::new();
        let src = "fx::sequence(&[";
        let completions: BTreeSet<String> = engine
            .completions(src, src.len() as u32)
            .into_iter()
            .map(|c| c.label)
            .collect();

        let expected: BTreeSet<String> = engine
            .effect_types
            .keys()
            .map(alloc::string::ToString::to_string)
            .collect();

        assert_eq!(completions, expected);
    }

    #[test]
    fn test_sequence_and_parallel_cursor_pos() {
        let engine = CompletionEngine::new();

        [("fx::sequen", "sequence(&[$0])"), ("fx::parall", "parallel(&[$0])")]
            .iter()
            .for_each(|(src, expected)| {
                let completions: Vec<Option<String>> = engine
                    .completions(src, src.len() as u32)
                    .into_iter()
                    .map(|c| c.insert_text)
                    .collect();

                assert_eq!(vec![Some(expected.to_string())], completions);
            });
    }

    #[test]
    fn test_bool_completions() {
        let engine = CompletionEngine::new();
        let source = "fx::freeze_at(0.5, ";
        let completions = engine.completions(source, source.len() as u32);

        // Should suggest true and false
        assert_eq!(completions.len(), 2);
        assert!(completions.iter().any(|c| c.label == "true"));
        assert!(completions.iter().any(|c| c.label == "false"));
    }

    // --- Wave type completion tests ---

    #[test]
    fn test_oscillator_namespace_completions() {
        let engine = CompletionEngine::new();
        let source = "Oscillator::";
        let completions = engine.completions(source, source.len() as u32);

        assert_eq!(completions.len(), 4);
        assert!(completions
            .iter()
            .all(|c| c.kind == CompletionKind::Function));
        assert!(completions.iter().any(|c| c.label == "sin"));
        assert!(completions.iter().any(|c| c.label == "cos"));
        assert!(completions.iter().any(|c| c.label == "triangle"));
        assert!(completions.iter().any(|c| c.label == "sawtooth"));
    }

    #[test]
    fn test_modulator_namespace_completions() {
        let engine = CompletionEngine::new();
        let source = "Modulator::";
        let completions = engine.completions(source, source.len() as u32);

        assert_eq!(completions.len(), 4);
        assert!(completions
            .iter()
            .all(|c| c.kind == CompletionKind::Function));
        assert!(completions.iter().any(|c| c.label == "sin"));
        assert!(completions.iter().any(|c| c.label == "cos"));
        assert!(completions.iter().any(|c| c.label == "triangle"));
        assert!(completions.iter().any(|c| c.label == "sawtooth"));
    }

    #[test]
    fn test_wave_layer_namespace_completions() {
        let engine = CompletionEngine::new();
        let source = "WaveLayer::";
        let completions = engine.completions(source, source.len() as u32);

        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].label, "new");
        assert_eq!(completions[0].kind, CompletionKind::Function);
    }

    #[test]
    fn test_wave_pattern_namespace_completions() {
        let engine = CompletionEngine::new();
        let source = "WavePattern::";
        let completions = engine.completions(source, source.len() as u32);

        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].label, "new");
        assert_eq!(completions[0].kind, CompletionKind::Function);
    }

    #[test]
    fn test_oscillator_method_chain() {
        let engine = CompletionEngine::new();
        let source = "Oscillator::sin(0.1, 0.2, 0.3).";
        let completions = engine.completions(source, source.len() as u32);

        assert_eq!(completions.len(), 3);
        assert!(completions
            .iter()
            .all(|c| c.kind == CompletionKind::Method));
        assert!(completions.iter().any(|c| c.label == "clone"));
        assert!(completions.iter().any(|c| c.label == "phase"));
        assert!(completions
            .iter()
            .any(|c| c.label == "modulated_by"));
        // constructors should not appear in dot access
        assert!(!completions.iter().any(|c| c.label == "sin"));
    }

    #[test]
    fn test_modulator_method_chain() {
        let engine = CompletionEngine::new();
        let source = "Modulator::sin(0.1, 0.2, 0.3).";
        let completions = engine.completions(source, source.len() as u32);

        assert_eq!(completions.len(), 5);
        assert!(completions
            .iter()
            .all(|c| c.kind == CompletionKind::Method));
        assert!(completions.iter().any(|c| c.label == "clone"));
        assert!(completions.iter().any(|c| c.label == "phase"));
        assert!(completions.iter().any(|c| c.label == "intensity"));
        assert!(completions.iter().any(|c| c.label == "on_phase"));
        assert!(completions
            .iter()
            .any(|c| c.label == "on_amplitude"));
    }

    #[test]
    fn test_wave_layer_method_chain() {
        let engine = CompletionEngine::new();
        let source = "WaveLayer::new(Oscillator::sin(0.1, 0.2, 0.3)).";
        let completions = engine.completions(source, source.len() as u32);

        assert_eq!(completions.len(), 7);
        assert!(completions
            .iter()
            .all(|c| c.kind == CompletionKind::Method));
        assert!(completions.iter().any(|c| c.label == "clone"));
        assert!(completions.iter().any(|c| c.label == "multiply"));
        assert!(completions.iter().any(|c| c.label == "average"));
        assert!(completions.iter().any(|c| c.label == "max"));
        assert!(completions.iter().any(|c| c.label == "amplitude"));
        assert!(completions.iter().any(|c| c.label == "power"));
        assert!(completions.iter().any(|c| c.label == "abs"));
        // constructor should not appear
        assert!(!completions.iter().any(|c| c.label == "new"));
    }

    #[test]
    fn test_wave_pattern_method_chain() {
        let engine = CompletionEngine::new();
        let source = "WavePattern::new(WaveLayer::new(Oscillator::sin(0.1, 0.2, 0.3))).";
        let completions = engine.completions(source, source.len() as u32);

        assert_eq!(completions.len(), 4);
        assert!(completions
            .iter()
            .all(|c| c.kind == CompletionKind::Method));
        assert!(completions.iter().any(|c| c.label == "clone"));
        assert!(completions
            .iter()
            .any(|c| c.label == "with_layer"));
        assert!(completions
            .iter()
            .any(|c| c.label == "with_contrast"));
        assert!(completions
            .iter()
            .any(|c| c.label == "with_transition_width"));
    }

    #[test]
    fn test_oscillator_constructor_param_hint() {
        let engine = CompletionEngine::new();
        let source = "Oscillator::sin(";
        let completions = engine.completions(source, source.len() as u32);

        assert!(!completions.is_empty());
        assert_eq!(completions[0], CompletionItem {
            label: "<f32>".to_string(),
            kind: CompletionKind::Type,
            detail: "Parameter 1 of 3".to_string(),
            insert_text: None,
            description: None,
        });
    }

    #[test]
    fn test_wave_layer_new_param_hint() {
        let engine = CompletionEngine::new();
        let source = "WaveLayer::new(";
        let completions = engine.completions(source, source.len() as u32);

        assert!(!completions.is_empty());
        assert_eq!(completions[0], CompletionItem {
            label: "Oscillator::".to_string(),
            kind: CompletionKind::Parameter,
            detail: "Parameter 1 of 1".to_string(),
            insert_text: None,
            description: None,
        });
    }

    #[test]
    fn test_wave_pattern_new_param_hint() {
        let engine = CompletionEngine::new();
        let source = "WavePattern::new(";
        let completions = engine.completions(source, source.len() as u32);

        assert!(!completions.is_empty());
        assert_eq!(completions[0], CompletionItem {
            label: "WaveLayer::".to_string(),
            kind: CompletionKind::Parameter,
            detail: "Parameter 1 of 1".to_string(),
            insert_text: None,
            description: None,
        });
    }

    #[test]
    fn test_oscillator_modulated_by_param_hint() {
        let engine = CompletionEngine::new();
        let source = "Oscillator::sin(0.1, 0.2, 0.3).modulated_by(";
        let completions = engine.completions(source, source.len() as u32);

        assert!(!completions.is_empty());
        assert_eq!(completions[0], CompletionItem {
            label: "Modulator::".to_string(),
            kind: CompletionKind::Parameter,
            detail: "Parameter 1 of 1".to_string(),
            insert_text: None,
            description: None,
        });
    }

    #[test]
    fn test_wave_layer_multiply_param_hint() {
        let engine = CompletionEngine::new();
        let source = "WaveLayer::new(Oscillator::sin(0.1, 0.2, 0.3)).multiply(";
        let completions = engine.completions(source, source.len() as u32);

        assert!(!completions.is_empty());
        assert_eq!(completions[0], CompletionItem {
            label: "Oscillator::".to_string(),
            kind: CompletionKind::Parameter,
            detail: "Parameter 1 of 1".to_string(),
            insert_text: None,
            description: None,
        });
    }

    #[test]
    fn test_wave_let_binding_inference() {
        let engine = CompletionEngine::new();
        let source = indoc! {r#"
            let osc = Oscillator::sin(0.1, 0.2, 0.3);
            let modulator = Modulator::cos(0.5, 0.0, 1.0);
            let layer = WaveLayer::new(osc);
            let pattern = WavePattern::new(layer);
        "#};

        let tokens = tokenize(source).map(sanitize_tokens).unwrap();
        let bindings = engine.extract_let_bindings(&tokens);

        assert_eq!(bindings, &[
            LetBinding::new("osc", "Oscillator"),
            LetBinding::new("modulator", "Modulator"),
            LetBinding::new("layer", "WaveLayer"),
            LetBinding::new("pattern", "WavePattern"),
        ]);
    }

    #[test]
    fn test_wave_let_binding_as_param_suggestion() {
        let engine = CompletionEngine::new();
        let source = indoc! {r#"
            let osc = Oscillator::sin(0.1, 0.2, 0.3);
            WaveLayer::new(
        "#};

        let completions = engine.completions(source, source.len() as u32);

        // should suggest Oscillator:: as param hint AND the 'osc' variable
        assert!(completions
            .iter()
            .any(|c| c.label == "Oscillator::" && c.kind == CompletionKind::Parameter));
        assert!(completions.iter().any(|c| c.label == "osc"
            && c.kind == CompletionKind::Variable
            && c.detail == "Oscillator"));
    }

    #[test]
    fn test_top_level_includes_wave_namespaces() {
        let engine = CompletionEngine::new();
        let completions = engine.completions("", 0);

        for ns in &["WaveLayer::", "Oscillator::", "Modulator::", "WavePattern::"] {
            assert!(
                completions.iter().any(|c| c.label == *ns),
                "Top-level completions should include {ns}"
            );
        }
    }

    #[test]
    fn test_wave_chained_methods_continue_type() {
        let engine = CompletionEngine::new();

        // chaining .amplitude() on WaveLayer should still offer WaveLayer methods
        let source = "WaveLayer::new(Oscillator::sin(0.1, 0.2, 0.3)).amplitude(2.0).";
        let completions = engine.completions(source, source.len() as u32);

        assert_eq!(completions.len(), 7);
        assert!(completions.iter().any(|c| c.label == "power"));
        assert!(completions.iter().any(|c| c.label == "abs"));
    }

    #[test]
    fn test_modulator_chained_methods_continue_type() {
        let engine = CompletionEngine::new();
        let source = "Modulator::sin(0.1, 0.2, 0.3).phase(1.0).";
        let completions = engine.completions(source, source.len() as u32);

        assert_eq!(completions.len(), 5);
        assert!(completions
            .iter()
            .any(|c| c.label == "on_amplitude"));
        assert!(completions.iter().any(|c| c.label == "intensity"));
    }
}
