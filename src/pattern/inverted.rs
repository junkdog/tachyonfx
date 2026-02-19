use alloc::boxed::Box;

#[cfg(feature = "dsl")]
use compact_str::{format_compact, CompactString};
use ratatui_core::layout::Rect;

#[cfg(feature = "dsl")]
use crate::dsl::DslFormat;
use crate::pattern::{
    any_pattern::AnyPatternContext, AnyPattern, InstancedPattern, Pattern, PreparedPattern,
};

/// A pattern that inverts the output of another pattern.
///
/// Maps every cell alpha `a` to `1.0 - a`, effectively reversing the
/// reveal direction of the wrapped pattern.
///
/// # Examples
///
/// ```
/// use tachyonfx::pattern::{InvertedPattern, RadialPattern};
///
/// // radial pattern that starts active at the edges and reveals toward center
/// let pattern = InvertedPattern::new(RadialPattern::center());
/// ```
#[derive(Clone, PartialEq, Debug)]
pub struct InvertedPattern {
    inner: Box<AnyPattern>,
}

/// Per-frame evaluation context for [`InvertedPattern`].
pub struct InvertedPatternContext {
    inner: Box<PreparedPattern<AnyPatternContext, AnyPattern>>,
}

impl InvertedPattern {
    /// Creates an inverted pattern that wraps the given pattern.
    pub fn new(pattern: impl Into<AnyPattern>) -> Self {
        Self { inner: Box::new(pattern.into()) }
    }
}

impl Pattern for InvertedPattern {
    type Context = InvertedPatternContext;

    fn for_frame(self, alpha: f32, area: Rect) -> PreparedPattern<Self::Context, Self>
    where
        Self: Sized,
    {
        let inner_prepared = self.inner.clone().for_frame(alpha, area);

        PreparedPattern {
            context: InvertedPatternContext { inner: Box::new(inner_prepared) },
            pattern: self,
        }
    }
}

impl InstancedPattern for PreparedPattern<InvertedPatternContext, InvertedPattern> {
    fn map_alpha(&mut self, pos: ratatui_core::layout::Position) -> f32 {
        1.0 - self.context.inner.map_alpha(pos)
    }
}

#[cfg(feature = "dsl")]
impl DslFormat for InvertedPattern {
    fn dsl_format(&self) -> CompactString {
        format_compact!("InvertedPattern::new({})", self.inner.dsl_format())
    }
}

#[cfg(test)]
mod tests {
    use ratatui_core::layout::{Position, Rect};

    use super::*;
    use crate::pattern::RadialPattern;

    #[test]
    fn test_inverted_flips_alpha() {
        let area = Rect::new(0, 0, 20, 1);
        let radial = RadialPattern::center();
        let inverted = InvertedPattern::new(radial);

        let mut radial_prepared = radial.for_frame(0.5, area);
        let mut inverted_prepared = inverted.for_frame(0.5, area);

        for x in 0..20 {
            let pos = Position::new(x, 0);
            let a = radial_prepared.map_alpha(pos);
            let inv = inverted_prepared.map_alpha(pos);
            assert!(
                (a + inv - 1.0).abs() < 1e-5,
                "a + inv should equal 1.0 at x={x}: a={a}, inv={inv}"
            );
        }
    }

    #[test]
    fn test_inverted_boundary_alphas() {
        let area = Rect::new(0, 0, 10, 1);
        let pattern = InvertedPattern::new(RadialPattern::center());

        // alpha=0 on radial => all 0.0 => inverted => all 1.0
        let mut p = pattern.clone().for_frame(0.0, area);
        for x in 0..10 {
            let a = p.map_alpha(Position::new(x, 0));
            assert!(a == 1.0, "alpha=0 inverted: expected 1.0 at x={x}, got {a}");
        }

        // alpha=1 on radial => all 1.0 => inverted => all 0.0
        let mut p = pattern.for_frame(1.0, area);
        for x in 0..10 {
            let a = p.map_alpha(Position::new(x, 0));
            assert!(a == 0.0, "alpha=1 inverted: expected 0.0 at x={x}, got {a}");
        }
    }
}
