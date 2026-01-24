use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use ratatui_core::layout::Rect;

use crate::{CellFilter, Duration, Shader};

/// Represents a span of time for an effect in the effect hierarchy.
///
/// `EffectSpan` is used to describe the structure and timing of effects within a
/// tachyonfx effect chain. It contains information about the effect's label, duration,
/// cell filter, and any child effects. This struct is primarily used for visualization
/// and analysis purposes, such as in the `EffectTimeline` widget.
///
/// # Notes
///
/// - The `EffectSpan` structure is typically created automatically when calling
///   `as_effect_span()` on an `Effect` or `Shader` implementation.
/// - For composite effects (like parallel or sequential effects), the `children` field
///   will contain `EffectSpan`s for each child effect.
/// - The `start` and `end` times are relative to the parent effect's start time.
#[derive(Clone)]
#[allow(dead_code)]
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

        if let Some(last) = children
            .last_mut()
            .filter(|last| last.children.is_empty())
        {
            last.is_leaf = true;
        }

        let end = effect
            .timer()
            .map(|timer| timer.duration())
            .unwrap_or_default()
            .as_secs_f32();

        let start = offset.as_secs_f32();
        Self {
            label: effect.name().to_string(),
            cell_filter: effect.cell_filter().cloned().unwrap_or_default(),
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
}

impl fmt::Display for EffectSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}
