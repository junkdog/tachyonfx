use std::fmt;
use crate::Effect;

struct EffectTimeline {
    spans: Vec<EffectSpan>
}

struct EffectSpan {
    label: String,
    parent: Option<usize>,
    start: f32,
    end: f32,
}

struct ChartBar {
    start: f32,
    end: f32,
}

impl fmt::Display for EffectSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:.2} - {:.2})", self.label, self.start, self.end)
    }
}


impl EffectTimeline {
    pub fn from(effect: &Effect) {
        // todo: trait Flattenable (?) to flatten nested effects, default impl for non-nested effects
        // todo: effect name/label
        // todo: EffectSpan from Effect
        let effects: Vec<Effect> = effect.flatten();
    }

    fn print_tree(&self) {
        if self.spans.is_empty() {
            println!("Empty timeline");
            return;
        }

        // Find root nodes (those with no parent)
        let root_indices: Vec<usize> = self.spans.iter()
            .enumerate()
            .filter(|(_, span)| span.parent.is_none())
            .map(|(index, _)| index)
            .collect();

        for (i, &root_index) in root_indices.iter().enumerate() {
            self.print_node(root_index, "", i == root_indices.len() - 1);
        }
    }

    fn print_node(&self, index: usize, prefix: &str, is_last: bool) {
        let span = &self.spans[index];
        println!("{}{}{}", prefix, if is_last { "└── " } else { "├── " }, span);

        let child_prefix = format!("{}{}   ", prefix, if is_last { " " } else { "│" });

        let children: Vec<usize> = self.spans.iter()
            .enumerate()
            .filter(|(_, child_span)| child_span.parent == Some(index))
            .map(|(child_index, _)| child_index)
            .collect();

        for (i, &child_index) in children.iter().enumerate() {
            self.print_node(child_index, &child_prefix, i == children.len() - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_tree() {
        let timeline = EffectTimeline {
            spans: vec![
                EffectSpan { label: "Root Effect".to_string(), parent: None, start: 0.0, end: 10.0 },
                EffectSpan { label: "Child Effect 1".to_string(), parent: Some(0), start: 1.0, end: 5.0 },
                EffectSpan { label: "Child Effect 2".to_string(), parent: Some(0), start: 6.0, end: 9.0 },
                EffectSpan { label: "Grandchild Effect 1".to_string(), parent: Some(1), start: 2.0, end: 3.0 },
                EffectSpan { label: "Grandchild Effect 2".to_string(), parent: Some(1), start: 3.5, end: 4.5 },
                EffectSpan { label: "Great-grandchild Effect".to_string(), parent: Some(4), start: 3.75, end: 4.25 },
            ],
        };

        timeline.print_tree();
    }
}