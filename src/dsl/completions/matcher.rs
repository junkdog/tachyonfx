use super::types::CompletionItem;

/// Handles completion matching and scoring based on partial input.
///
/// Implements three scoring strategies:
/// 1. Prefix matching: partial matches the start of the completion (highest score)
/// 2. Smart matching: acronym/abbreviation matching (e.g., "EIO" -> "ExpoInOut")
/// 3. Fuzzy matching: sequential character matching anywhere (lowest score)
#[derive(Debug)]
pub(super) struct CompletionMatcher {
    partial: String,
}

impl CompletionMatcher {
    pub(super) fn new(partial: impl Into<String>) -> Self {
        Self { partial: partial.into() }
    }

    /// Filters and scores completions based on the partial input.
    /// Returns completions sorted by score (highest first).
    pub(super) fn filter_and_score(&self, completions: Vec<CompletionItem>) -> Vec<CompletionItem> {
        if self.partial.is_empty() {
            return completions;
        }

        let mut scored: Vec<(CompletionItem, u32)> = completions
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
        if let Some(score) = smart_match(label, &partial_lower) {
            return Some(500 + score);
        }

        // Strategy 3: Fuzzy matching (sequential characters)
        if let Some(score) = fuzzy_match(&label_lower, &partial_lower) {
            return Some(score);
        }

        None
    }
}

/// Smart matching for acronyms and snake_case/camelCase abbreviations.
/// Examples: "EIO" matches "ExpoInOut", "sc" matches "snake_case"
fn smart_match(label: &str, partial_lower: &str) -> Option<u32> {
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
fn fuzzy_match(label_lower: &str, partial_lower: &str) -> Option<u32> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::completions::types::CompletionKind;

    #[test]
    fn test_matcher_prefix_matching() {
        let matcher = CompletionMatcher::new("fade");

        let completions = vec![
            CompletionItem {
                label: "fade_to".to_string(),
                kind: CompletionKind::Function,
                detail: String::new(),
                insert_text: Some("fade_to($0)".to_string()),
                description: None,
            },
            CompletionItem {
                label: "fade_from".to_string(),
                kind: CompletionKind::Function,
                detail: String::new(),
                insert_text: Some("fade_from($0)".to_string()),
                description: None,
            },
            CompletionItem {
                label: "dissolve".to_string(),
                kind: CompletionKind::Function,
                detail: String::new(),
                insert_text: Some("dissolve($0)".to_string()),
                description: None,
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
            "Prefix should beat smart: {score_prefix} vs {score_smart}"
        );
        assert!(
            score_prefix > score_fuzzy,
            "Prefix should beat fuzzy: {score_prefix} vs {score_fuzzy}"
        );

        // Smart should score higher than fuzzy
        assert!(
            score_smart > score_fuzzy,
            "Smart should beat fuzzy: {score_smart} vs {score_fuzzy}"
        );
    }

    #[test]
    fn test_matcher_empty_partial() {
        let matcher = CompletionMatcher::new("");

        let completions = vec![
            CompletionItem {
                label: "a".to_string(),
                kind: CompletionKind::Function,
                detail: String::new(),
                insert_text: Some("a($0)".to_string()),
                description: None,
            },
            CompletionItem {
                label: "b".to_string(),
                kind: CompletionKind::Function,
                detail: String::new(),
                insert_text: Some("b($0)".to_string()),
                description: None,
            },
            CompletionItem {
                label: "c".to_string(),
                kind: CompletionKind::Function,
                detail: String::new(),
                insert_text: Some("c($0)".to_string()),
                description: None,
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
            CompletionItem {
                label: "fade_to".to_string(),
                kind: CompletionKind::Function,
                detail: String::new(),
                insert_text: Some("fade_to($0)".to_string()),
                description: None,
            },
            CompletionItem {
                label: "dissolve".to_string(),
                kind: CompletionKind::Function,
                detail: String::new(),
                insert_text: Some("dissolve($0)".to_string()),
                description: None,
            },
        ];

        let filtered = matcher.filter_and_score(completions);

        // No matches should return empty
        assert!(filtered.is_empty());
    }
}
