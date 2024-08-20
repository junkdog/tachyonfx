use std::fmt;
use std::time::Duration;
use ratatui::layout::Rect;
use ratatui::prelude::Line;
use ratatui::style::Style;
use ratatui::text::Span;
use crate::widget::ColorRegistry;
use crate::{CellFilter, Shader};

pub(crate) fn effect_span_tree<'a>(
    colorizer: &ColorRegistry,
    span: &EffectSpan
) -> Vec<Line<'a>> {
    build_effect_span_tree(colorizer, span, Vec::new(), 0, span.is_leaf)
}

fn build_effect_span_tree<'a>(
    colorizer: &ColorRegistry,
    span: &EffectSpan,
    indent_spans: Vec<Style>,
    indent: u128,
    is_last: bool,
) -> Vec<Line<'a>> {
    let mut result = Vec::new();
    let mut indent_styles: Vec<Style> = indent_spans;
    indent_styles.push(Style::default().fg(colorizer.color_of(&span.label)));

    let depth = indent_styles.len();
    let mut spans = Vec::new();

    // tree structure
    spans.extend((0..depth).skip(1).map(|i| {
        let indent = if i == depth - 1 {
            if is_last { "└ " } else { "├ " }
        } else if indent & (1 << i) != 0  {
            "│ "
        } else {
            "  "
        };
        Span::styled(indent, indent_styles[i - 1])
    }));



    // label
    spans.push(Span::styled(span.label.clone(), indent_styles[depth - 1]));
    result.push(Line::from(spans));

    let child_count = span.children.len();

    for (index, child) in span.children.iter().enumerate() {
        let new_indent = if index != child_count - 1 { indent | (1 << depth) } else { indent };
        let is_last = index == child_count - 1;
        result.extend(build_effect_span_tree(colorizer, child, indent_styles.clone(), new_indent, is_last));
    }

    result
}


/// Represents a span of time for an effect in the effect hierarchy.
///
/// `EffectSpan` is used to describe the structure and timing of effects within a tachyonfx
/// effect chain. It contains information about the effect's label, duration, cell filter,
/// and any child effects. This struct is primarily used for visualization and analysis
/// purposes, such as in the `EffectTimeline` widget.
///
/// # Notes
///
/// - The `EffectSpan` structure is typically created automatically when calling `as_effect_span()`
///   on an `Effect` or `Shader` implementation.
/// - For composite effects (like parallel or sequential effects), the `children` field will
///   contain `EffectSpan`s for each child effect.
/// - The `start` and `end` times are relative to the parent effect's start time.
#[derive(Clone)]
pub struct EffectSpan {
    pub(crate) label: String,
    pub(crate) cell_filter: CellFilter,
    pub(crate) area: Option<Rect>,
    pub(crate) start: f32,
    pub(crate) end: f32,
    pub(crate) children: Vec<EffectSpan>,
    pub(crate) is_leaf: bool,
}

impl EffectSpan {
    pub fn new<S: Shader + ?Sized>(
        effect: &S,
        offset: Duration,
        children: Vec<EffectSpan>,
    ) -> Self {
        let mut children = children;

        if let Some(last) = children.last_mut().filter(|last| last.children.is_empty()) {
            last.is_leaf = true;
        }

        let end = effect
            .timer()
            .map(|timer| timer.duration())
            .unwrap_or(Duration::default())
            .as_secs_f32();

        let start = offset.as_secs_f32();
        Self {
            label: effect.name().to_string(),
            cell_filter: effect.cell_selection().unwrap_or_default(),
            area: effect.area(),
            start,
            end: start + end,
            children,
            is_leaf: false,
        }
    }

    pub fn new_leaf_node<S: Shader + ?Sized>(
        effect: &S,
        offset: Duration,
        children: Vec<EffectSpan>,
    ) -> Self {
        let mut span = Self::new(effect, offset, children);
        span.is_leaf = true;
        span
    }

    pub(crate) fn iter(&self) -> EffectSpanIterator {
        EffectSpanIterator::new(self)
    }
}

impl fmt::Display for EffectSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

pub struct EffectSpanIterator<'a> {
    stack: Vec<&'a EffectSpan>,
}

impl<'a> EffectSpanIterator<'a> {
    pub fn new(root: &'a EffectSpan) -> Self {
        EffectSpanIterator { stack: vec![root] }
    }
}

impl<'a> Iterator for EffectSpanIterator<'a> {
    type Item = &'a EffectSpan;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.stack.pop() {
            if !node.children.is_empty() {
                let child_nodes: Vec<&'a EffectSpan> = node.children.iter().rev().collect();
                self.stack.extend(child_nodes);
            }
            Some(node)
        } else {
            None
        }
    }
}