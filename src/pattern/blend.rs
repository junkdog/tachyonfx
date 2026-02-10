use alloc::boxed::Box;

use ratatui_core::layout::Rect;

use crate::pattern::{
    any_pattern::AnyPatternContext, AnyPattern, InstancedPattern, Pattern, PreparedPattern,
};

/// A pattern that linearly interpolates between two sub-patterns.
///
/// At each cell position the final alpha is computed as:
///
/// ```text
/// alpha = (1 - t) * pattern_a(pos) + t * pattern_b(pos)
/// ```
///
/// where `t` is the global animation progress. This produces a smooth
/// crossfade from `pattern_a` to `pattern_b` over the effect's lifetime.
///
/// # Examples
///
/// ```
/// use tachyonfx::pattern::{BlendPattern, RadialPattern, DiagonalPattern};
///
/// // crossfade from a radial wipe to a diagonal wipe
/// let pattern = BlendPattern::new(
///     RadialPattern::center(),
///     DiagonalPattern::top_left_to_bottom_right(),
/// );
/// ```
#[derive(Clone, PartialEq, Debug)]
pub struct BlendPattern {
    inner: Box<BlendPatternInner>,
}

#[derive(Clone, PartialEq, Debug)]
struct BlendPatternInner {
    pattern_a: AnyPattern,
    pattern_b: AnyPattern,
}

/// Per-frame evaluation context for [`BlendPattern`].
pub struct BlendPatternContext {
    global_alpha: f32,
    inner: Box<BlendPatternContextInner>,
}

struct BlendPatternContextInner {
    context_a: PreparedPattern<AnyPatternContext, AnyPattern>,
    context_b: PreparedPattern<AnyPatternContext, AnyPattern>,
}

impl BlendPattern {
    /// Creates a blend pattern that crossfades from `pattern_a` to `pattern_b`.
    ///
    /// At global alpha 0 the output matches `pattern_a`; at alpha 1 it
    /// matches `pattern_b`.
    pub fn new(pattern_a: impl Into<AnyPattern>, pattern_b: impl Into<AnyPattern>) -> Self {
        Self {
            inner: Box::new(BlendPatternInner {
                pattern_a: pattern_a.into(),
                pattern_b: pattern_b.into(),
            }),
        }
    }

    pub(crate) fn pattern_a(&self) -> &AnyPattern {
        &self.inner.pattern_a
    }

    pub(crate) fn pattern_b(&self) -> &AnyPattern {
        &self.inner.pattern_b
    }
}

impl Pattern for BlendPattern {
    type Context = BlendPatternContext;

    fn for_frame(self, alpha: f32, area: Rect) -> PreparedPattern<Self::Context, Self>
    where
        Self: Sized,
    {
        let context_a = self
            .inner
            .pattern_a
            .clone()
            .for_frame(alpha, area);
        let context_b = self
            .inner
            .pattern_b
            .clone()
            .for_frame(alpha, area);

        PreparedPattern {
            context: BlendPatternContext {
                global_alpha: alpha,
                inner: Box::new(BlendPatternContextInner { context_a, context_b }),
            },
            pattern: self,
        }
    }
}

impl InstancedPattern for PreparedPattern<BlendPatternContext, BlendPattern> {
    fn map_alpha(&mut self, pos: ratatui_core::layout::Position) -> f32 {
        let inner = &mut *self.context.inner;
        let alpha_a = inner.context_a.map_alpha(pos);
        let alpha_b = inner.context_b.map_alpha(pos);

        let global_alpha = self.context.global_alpha;
        (1.0 - global_alpha) * alpha_a + global_alpha * alpha_b
    }
}
