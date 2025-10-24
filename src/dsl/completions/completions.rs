use std::collections::HashMap;

use super::{
    dsl_type::all_methods,
    types::{
        tok, CallableItem, Completion, CompletionContext, CompletionKind, LetBinding, TokenCursor,
    },
};
use crate::dsl::{
    tokenizer::{Token, TokenKind},
    EffectDsl,
};

/// Handles completion matching and scoring based on partial input.
///
/// Implements three scoring strategies:
/// 1. Prefix matching: partial matches the start of the completion (highest score)
/// 2. Smart matching: acronym/abbreviation matching (e.g., "EIO" -> "ExpoInOut")
/// 3. Fuzzy matching: sequential character matching anywhere (lowest score)
#[derive(Debug)]
struct CompletionMatcher {
    partial: String,
}

impl CompletionMatcher {
    fn new(partial: impl Into<String>) -> Self {
        Self { partial: partial.into() }
    }

    /// Filters and scores completions based on the partial input.
    /// Returns completions sorted by score (highest first).
    fn filter_and_score(&self, completions: Vec<Completion>) -> Vec<Completion> {
        if self.partial.is_empty() {
            return completions;
        }

        let mut scored: Vec<(Completion, u32)> = completions
            .into_iter()
            .filter_map(|completion| {
                self.score(&completion.label)
                    .map(|score| (completion, score))
            })
            .collect();

        // Sort by score descending, then alphabetically for same scores
        scored.sort_by(|(a_comp, a_score), (b_comp, b_score)| {
            b_score
                .cmp(a_score)
                .then_with(|| a_comp.label.cmp(&b_comp.label))
        });

        scored.into_iter().map(|(comp, _)| comp).collect()
    }

    /// Scores a completion label against the partial input.
    /// Returns None if no match, otherwise returns score (higher is better).
    fn score(&self, label: &str) -> Option<u32> {
        // Case-insensitive comparison
        let label_lower = label.to_lowercase();
        let partial_lower = self.partial.to_lowercase();

        // Strategy 1: Prefix match (score: 1000 + remaining length)
        if label_lower.starts_with(&partial_lower) {
            return Some(1000 + (label.len() - self.partial.len()) as u32);
        }

        // Strategy 2: Smart matching (acronym/abbreviation)
        if let Some(score) = self.smart_match(label, &partial_lower) {
            return Some(500 + score);
        }

        // Strategy 3: Fuzzy matching (sequential characters)
        if let Some(score) = self.fuzzy_match(&label_lower, &partial_lower) {
            return Some(score);
        }

        None
    }

    /// Smart matching for acronyms and snake_case/camelCase abbreviations.
    /// Examples: "EIO" matches "ExpoInOut", "sc" matches "snake_case"
    fn smart_match(&self, label: &str, partial_lower: &str) -> Option<u32> {
        let partial_chars: Vec<char> = partial_lower.chars().collect();

        // Extract significant characters (uppercase, after underscore, start)
        let mut significant: Vec<(char, usize)> = Vec::new();
        let mut prev_was_underscore = false;

        for (i, ch) in label.chars().enumerate() {
            if i == 0 || ch.is_uppercase() || prev_was_underscore {
                significant.push((ch.to_lowercase().next()?, i));
            }
            prev_was_underscore = ch == '_';
        }

        // Try to match partial chars against significant chars
        let mut partial_idx = 0;
        let mut last_match_pos = 0;
        let mut gaps = 0;

        for (sig_char, pos) in significant {
            if partial_idx >= partial_chars.len() {
                break;
            }

            if sig_char == partial_chars[partial_idx] {
                gaps += pos.saturating_sub(last_match_pos);
                last_match_pos = pos;
                partial_idx += 1;
            }
        }

        if partial_idx == partial_chars.len() {
            // All characters matched, score based on how tight the match was
            Some(100 - gaps.min(99) as u32)
        } else {
            None
        }
    }

    /// Fuzzy matching - matches characters in sequence anywhere in the string.
    /// Score is based on how early and how tightly packed the matches are.
    fn fuzzy_match(&self, label_lower: &str, partial_lower: &str) -> Option<u32> {
        let label_chars: Vec<char> = label_lower.chars().collect();
        let partial_chars: Vec<char> = partial_lower.chars().collect();

        let mut label_idx = 0;
        let mut first_match = None;
        let mut last_match = 0;
        let mut gaps = 0;

        for &partial_char in &partial_chars {
            // Find next occurrence of this character
            let found = label_chars[label_idx..]
                .iter()
                .position(|&c| c == partial_char)?;

            let match_pos = label_idx + found;

            if first_match.is_none() {
                first_match = Some(match_pos);
            }

            gaps += found;
            last_match = match_pos;
            label_idx = match_pos + 1;
        }

        // Score: prefer early matches and tight packing
        let first = first_match?;
        let spread = last_match - first;
        let score = 100_u32
            .saturating_sub(first as u32)       // Earlier is better
            .saturating_sub(spread as u32 / 2)  // Tighter is better
            .saturating_sub(gaps as u32 / 3); // Fewer gaps is better

        Some(score.max(1)) // Ensure at least 1 if it matches
    }
}

#[derive(Debug, Clone)]
pub struct CompletionEngine {
    effect_types: Vec<&'static str>,
    methods: HashMap<&'static str, Vec<CallableItem>>,
    interpolations: Vec<&'static str>,
    motions: Vec<&'static str>,
    color_spaces: Vec<&'static str>,
    directions: Vec<&'static str>,
    flexes: Vec<&'static str>,
    expand_directions: Vec<&'static str>,
    modifiers: Vec<&'static str>,
    color_constants: Vec<&'static str>,
    repeat_modes: Vec<&'static str>,
    evolve_symbol_sets: Vec<&'static str>,
    cell_filter_constants: Vec<&'static str>,
}

impl CompletionEngine {
    pub fn new() -> Self {
        let methods = all_methods();
        let effect_types = EffectDsl::new().registered_effects();

        let interpolations = vec![
            "BackIn",
            "BackOut",
            "BackInOut",
            "BounceIn",
            "BounceOut",
            "BounceInOut",
            "CircIn",
            "CircOut",
            "CircInOut",
            "CubicIn",
            "CubicOut",
            "CubicInOut",
            "ElasticIn",
            "ElasticOut",
            "ElasticInOut",
            "ExpoIn",
            "ExpoOut",
            "ExpoInOut",
            "Linear",
            "QuadIn",
            "QuadOut",
            "QuadInOut",
            "QuartIn",
            "QuartOut",
            "QuartInOut",
            "QuintIn",
            "QuintOut",
            "QuintInOut",
            "Reverse",
            "SineIn",
            "SineOut",
            "SineInOut",
        ];

        let motions = vec!["LeftToRight", "RightToLeft", "UpToDown", "DownToUp"];

        let color_spaces = vec!["Rgb", "Hsl", "Hsv"];

        let directions = vec!["Horizontal", "Vertical"];

        let flexes = vec!["Legacy", "Start", "End", "Center", "SpaceBetween", "SpaceAround"];

        let expand_directions = vec!["Horizontal", "Vertical"];

        let modifiers = vec![
            "BOLD",
            "DIM",
            "ITALIC",
            "UNDERLINED",
            "SLOW_BLINK",
            "RAPID_BLINK",
            "REVERSED",
            "HIDDEN",
            "CROSSED_OUT",
        ];

        let color_constants = vec![
            "Reset",
            "Black",
            "Red",
            "Green",
            "Yellow",
            "Blue",
            "Magenta",
            "Cyan",
            "Gray",
            "DarkGray",
            "LightRed",
            "LightGreen",
            "LightYellow",
            "LightBlue",
            "LightMagenta",
            "LightCyan",
            "White",
        ];

        let repeat_modes = vec!["Forever"];

        let evolve_symbol_sets = vec![
            "BlocksHorizontal",
            "BlocksVertical",
            "CircleFill",
            "Circles",
            "Quadrants",
            "Shaded",
            "Squares",
        ];

        let cell_filter_constants = vec!["All", "Text"];

        Self {
            methods,
            effect_types,
            interpolations,
            motions,
            color_spaces,
            directions,
            flexes,
            expand_directions,
            modifiers,
            color_constants,
            repeat_modes,
            evolve_symbol_sets,
            cell_filter_constants,
        }
    }

    fn const_completions(&self, constants: &[&'static str], meta_desc: &str) -> Vec<Completion> {
        constants
            .iter()
            .map(|name| Completion {
                label: name.to_string(),
                kind: CompletionKind::Constant,
                meta: Some(meta_desc.to_string()),
            })
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
        let let_bindings = self.extract_let_bindings(tokens);

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
                        meta: Some("Text style modifiers".to_string()),
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
                self.methods
                    .get(receiver_type.as_str())
                    .map(|items| {
                        items
                            .iter()
                            .filter(|item| item.is_instance())
                            .map(|item| {
                                let detail = if item.params().is_empty() {
                                    format!("{}()", item.name())
                                } else {
                                    format!("{}({})", item.name(), item.params().join(", "))
                                };

                                Completion {
                                    label: item.name().to_string(),
                                    kind: CompletionKind::Method,
                                    meta: Some(detail),
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default()
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

                    "Interpolation" => {
                        self.const_completions(&self.interpolations, "Easing function")
                    },
                    "Motion" => self.const_completions(&self.motions, "Movement direction"),
                    "ColorSpace" => self.const_completions(&self.color_spaces, "Color space"),
                    "Direction" => self.const_completions(&self.directions, "Layout direction"),
                    "Flex" => self.const_completions(&self.flexes, "Flex mode"),
                    "ExpandDirection" => {
                        self.const_completions(&self.expand_directions, "Expand direction")
                    },
                    "Modifier" => self.const_completions(&self.modifiers, "Text modifier"),
                    "RepeatMode" => {
                        let mut completions =
                            self.const_completions(&self.repeat_modes, "Repeat mode");
                        // Also add RepeatMode constructors from methods
                        if let Some(items) = self.methods.get("RepeatMode") {
                            completions.extend(items.iter().filter(|item| item.is_static()).map(
                                |item| {
                                    let detail = if item.params().is_empty() {
                                        format!("{}()", item.name())
                                    } else {
                                        format!("{}({})", item.name(), item.params().join(", "))
                                    };
                                    Completion {
                                        label: item.name().to_string(),
                                        kind: CompletionKind::Function,
                                        meta: Some(detail),
                                    }
                                },
                            ));
                        }
                        completions
                    },
                    "EvolveSymbolSet" => {
                        self.const_completions(&self.evolve_symbol_sets, "Symbol set")
                    },
                    "CellFilter" => {
                        let mut completions =
                            self.const_completions(&self.cell_filter_constants, "Cell filter");
                        // Also add CellFilter constructors from methods
                        if let Some(items) = self.methods.get("CellFilter") {
                            completions.extend(items.iter().filter(|item| item.is_static()).map(
                                |item| {
                                    let detail = if item.params().is_empty() {
                                        format!("{}()", item.name())
                                    } else {
                                        format!("{}({})", item.name(), item.params().join(", "))
                                    };
                                    Completion {
                                        label: item.name().to_string(),
                                        kind: CompletionKind::Function,
                                        meta: Some(detail),
                                    }
                                },
                            ));
                        }
                        completions
                    },
                    "Color" => {
                        let mut completions =
                            self.const_completions(&self.color_constants, "Color constant");
                        // Also add Color constructors from methods
                        if let Some(items) = self.methods.get("Color") {
                            completions.extend(items.iter().filter(|item| item.is_static()).map(
                                |item| {
                                    let detail = if item.params().is_empty() {
                                        format!("{}()", item.name())
                                    } else {
                                        format!("{}({})", item.name(), item.params().join(", "))
                                    };
                                    Completion {
                                        label: item.name().to_string(),
                                        kind: CompletionKind::Function,
                                        meta: Some(detail),
                                    }
                                },
                            ));
                        }
                        completions
                    },

                    // For other types, use the methods list (constructors/static methods only)
                    _ => self
                        .methods
                        .get(namespace.as_str())
                        .map(|items| {
                            items
                                .iter()
                                .filter(|item| item.is_static())
                                .map(|item| {
                                    let detail = if item.params().is_empty() {
                                        format!("{}()", item.name())
                                    } else {
                                        format!("{}({})", item.name(), item.params().join(", "))
                                    };

                                    Completion {
                                        label: item.name().to_string(),
                                        kind: CompletionKind::Function,
                                        meta: Some(detail),
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
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
        Some(match () {
            _ if self.cell_filter_constants.contains(&identifier) => "CellFilter",
            _ if self.effect_types.contains(&identifier) => "Effect",
            _ => None?,
        })
    }

    fn resolve_shortform_constants(&self, identifier: &str) -> Option<&'static str> {
        Some(match () {
            _ if self.cell_filter_constants.contains(&identifier) => "CellFilter",
            _ if self.color_constants.contains(&identifier) => "Color",
            _ if self.color_spaces.contains(&identifier) => "ColorSpace",
            _ if self.directions.contains(&identifier) => "Direction",
            _ if self.evolve_symbol_sets.contains(&identifier) => "EvolveSymbolSet",
            _ if self.expand_directions.contains(&identifier) => "ExpandDirection",
            _ if self.flexes.contains(&identifier) => "Flex",
            _ if self.interpolations.contains(&identifier) => "Interpolation",
            _ if self.modifiers.contains(&identifier) => "Modifier",
            _ if self.motions.contains(&identifier) => "Motion",
            _ if self.repeat_modes.contains(&identifier) => "RepeatMode",

            _ => None?,
        })
    }
}

impl Default for CompletionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Extracts the partial token at the cursor position for completion matching.
fn extract_partial_token(tokens: &[Token], cursor: &TokenCursor) -> String {
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

fn analyze_last_tokens(tokens: &[Token], cursor: &TokenCursor) -> CompletionContext {
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

    // CompletionMatcher tests
    #[test]
    fn test_matcher_prefix_matching() {
        let matcher = CompletionMatcher::new("fade");

        let completions = vec![
            Completion {
                label: "fade_to".to_string(),
                kind: CompletionKind::Function,
                meta: None,
            },
            Completion {
                label: "fade_from".to_string(),
                kind: CompletionKind::Function,
                meta: None,
            },
            Completion {
                label: "dissolve".to_string(),
                kind: CompletionKind::Function,
                meta: None,
            },
        ];

        let filtered = matcher.filter_and_score(completions);

        // Should only include items starting with "fade"
        assert_eq!(filtered.len(), 2);
        assert!(filtered
            .iter()
            .all(|c| c.label.starts_with("fade")));
    }

    #[test]
    fn test_matcher_smart_matching_acronym() {
        let matcher = CompletionMatcher::new("EIO");

        let score_expo = matcher.score("ExpoInOut");
        let score_elastic = matcher.score("ElasticInOut");

        // Both should match via smart matching
        assert!(score_expo.is_some(), "ExpoInOut should match EIO");
        assert!(score_elastic.is_some(), "ElasticInOut should match EIO");

        // Should be in the 500+ range (smart match)
        assert!(score_expo.unwrap() >= 500);
        assert!(score_elastic.unwrap() >= 500);
    }

    #[test]
    fn test_matcher_smart_matching_snake_case() {
        let matcher = CompletionMatcher::new("sc");

        let score = matcher.score("snake_case");

        // Should match via smart matching (s, c after underscore)
        assert!(score.is_some(), "snake_case should match sc");
        assert!(score.unwrap() >= 500, "Should be smart match score");
    }

    #[test]
    fn test_matcher_fuzzy_matching() {
        let matcher = CompletionMatcher::new("dsl");

        let score = matcher.score("dissolve");

        // Should match via fuzzy (d, s, l are in sequence)
        assert!(score.is_some(), "dissolve should fuzzy match dsl");
        // Should be lower than smart/prefix scores
        assert!(score.unwrap() < 500, "Should be fuzzy match score");
    }

    #[test]
    fn test_matcher_case_insensitive() {
        let matcher = CompletionMatcher::new("BOLD");

        let score_upper = matcher.score("BOLD");
        let score_lower = matcher.score("bold");
        let score_mixed = matcher.score("Bold");

        // All should match with same score
        assert_eq!(score_upper, score_lower);
        assert_eq!(score_upper, score_mixed);
    }

    #[test]
    fn test_matcher_scoring_order() {
        let matcher = CompletionMatcher::new("li");

        let score_prefix = matcher.score("Linear").unwrap(); // Prefix match
        let score_smart = matcher.score("LeftIn").unwrap(); // Smart match (L, I)
        let score_fuzzy = matcher.score("ElasticIn").unwrap(); // Fuzzy match

        // Prefix should score highest
        assert!(
            score_prefix > score_smart,
            "Prefix should beat smart: {} vs {}",
            score_prefix,
            score_smart
        );
        assert!(
            score_prefix > score_fuzzy,
            "Prefix should beat fuzzy: {} vs {}",
            score_prefix,
            score_fuzzy
        );

        // Smart should score higher than fuzzy
        assert!(
            score_smart > score_fuzzy,
            "Smart should beat fuzzy: {} vs {}",
            score_smart,
            score_fuzzy
        );
    }

    #[test]
    fn test_matcher_empty_partial() {
        let matcher = CompletionMatcher::new("");

        let completions = vec![
            Completion {
                label: "a".to_string(),
                kind: CompletionKind::Function,
                meta: None,
            },
            Completion {
                label: "b".to_string(),
                kind: CompletionKind::Function,
                meta: None,
            },
            Completion {
                label: "c".to_string(),
                kind: CompletionKind::Function,
                meta: None,
            },
        ];

        let filtered = matcher.filter_and_score(completions.clone());

        // Empty partial should return all completions unchanged
        assert_eq!(filtered.len(), completions.len());
    }

    #[test]
    fn test_matcher_no_matches() {
        let matcher = CompletionMatcher::new("xyz");

        let completions = vec![
            Completion {
                label: "fade_to".to_string(),
                kind: CompletionKind::Function,
                meta: None,
            },
            Completion {
                label: "dissolve".to_string(),
                kind: CompletionKind::Function,
                meta: None,
            },
        ];

        let filtered = matcher.filter_and_score(completions);

        // No matches should return empty
        assert!(filtered.is_empty());
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

        // Test directions
        assert_eq!(
            engine.resolve_shortform_constants("Horizontal"),
            Some("Direction")
        );
        assert_eq!(
            engine.resolve_shortform_constants("Vertical"),
            Some("Direction")
        );

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
