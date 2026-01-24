use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use ratatui_core::layout::Rect;

use crate::{CellFilter, Duration, Shader};

/// Represents a span of time for an effect in the effect hierarchy.
///
/// # Deprecation
///
/// This type was used by the now-removed `EffectTimeline` widget and no longer
/// serves any purpose. It is deprecated and scheduled for removal in a future release.
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
