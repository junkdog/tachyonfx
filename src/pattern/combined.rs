use alloc::boxed::Box;

#[cfg(feature = "dsl")]
use compact_str::{format_compact, CompactString};
use ratatui_core::layout::Rect;

#[cfg(feature = "dsl")]
use crate::dsl::DslFormat;
use crate::pattern::{
    any_pattern::AnyPatternContext, AnyPattern, InstancedPattern, Pattern, PreparedPattern,
};

/// Operation used to combine the alpha values of two sub-patterns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PatternOp {
    /// Element-wise product: `a * b`
    Multiply,
    /// Element-wise maximum: `max(a, b)`
    Max,
    /// Element-wise minimum: `min(a, b)`
    Min,
    /// Arithmetic mean: `(a + b) / 2`
    Average,
}

/// A pattern that combines two sub-patterns using a binary operation.
///
/// At each cell position the final alpha is computed by applying
/// the selected [`PatternOp`] to the alpha values produced by
/// `pattern_a` and `pattern_b`.
///
/// # Examples
///
/// ```
/// use tachyonfx::pattern::{CombinedPattern, RadialPattern, DiagonalPattern};
///
/// // only reveal cells where both patterns are active
/// let pattern = CombinedPattern::multiply(
///     RadialPattern::center(),
///     DiagonalPattern::top_left_to_bottom_right(),
/// );
/// ```
#[derive(Clone, PartialEq, Debug)]
pub struct CombinedPattern {
    inner: Box<CombinedPatternInner>,
}

#[derive(Clone, PartialEq, Debug)]
struct CombinedPatternInner {
    op: PatternOp,
    pattern_a: AnyPattern,
    pattern_b: AnyPattern,
}

/// Per-frame evaluation context for [`CombinedPattern`].
pub struct CombinedPatternContext {
    op: PatternOp,
    inner: Box<CombinedPatternContextInner>,
}

struct CombinedPatternContextInner {
    context_a: PreparedPattern<AnyPatternContext, AnyPattern>,
    context_b: PreparedPattern<AnyPatternContext, AnyPattern>,
}

impl CombinedPattern {
    /// Creates a combined pattern with the given operation.
    pub fn new(
        op: PatternOp,
        pattern_a: impl Into<AnyPattern>,
        pattern_b: impl Into<AnyPattern>,
    ) -> Self {
        Self {
            inner: Box::new(CombinedPatternInner {
                op,
                pattern_a: pattern_a.into(),
                pattern_b: pattern_b.into(),
            }),
        }
    }

    /// Multiplies the alpha values of two patterns: `a * b`.
    pub fn multiply(pattern_a: impl Into<AnyPattern>, pattern_b: impl Into<AnyPattern>) -> Self {
        Self::new(PatternOp::Multiply, pattern_a, pattern_b)
    }

    /// Takes the maximum alpha of two patterns: `max(a, b)`.
    pub fn max(pattern_a: impl Into<AnyPattern>, pattern_b: impl Into<AnyPattern>) -> Self {
        Self::new(PatternOp::Max, pattern_a, pattern_b)
    }

    /// Takes the minimum alpha of two patterns: `min(a, b)`.
    pub fn min(pattern_a: impl Into<AnyPattern>, pattern_b: impl Into<AnyPattern>) -> Self {
        Self::new(PatternOp::Min, pattern_a, pattern_b)
    }

    /// Averages the alpha values of two patterns: `(a + b) / 2`.
    pub fn average(pattern_a: impl Into<AnyPattern>, pattern_b: impl Into<AnyPattern>) -> Self {
        Self::new(PatternOp::Average, pattern_a, pattern_b)
    }
}

impl Pattern for CombinedPattern {
    type Context = CombinedPatternContext;

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
            context: CombinedPatternContext {
                op: self.inner.op,
                inner: Box::new(CombinedPatternContextInner { context_a, context_b }),
            },
            pattern: self,
        }
    }
}

impl InstancedPattern for PreparedPattern<CombinedPatternContext, CombinedPattern> {
    fn map_alpha(&mut self, pos: ratatui_core::layout::Position) -> f32 {
        let inner = &mut *self.context.inner;
        let a = inner.context_a.map_alpha(pos);
        let b = inner.context_b.map_alpha(pos);

        match self.context.op {
            PatternOp::Multiply => a * b,
            PatternOp::Max => {
                if a > b {
                    a
                } else {
                    b
                }
            },
            PatternOp::Min => {
                if a < b {
                    a
                } else {
                    b
                }
            },
            PatternOp::Average => (a + b) * 0.5,
        }
    }
}

#[cfg(feature = "dsl")]
impl DslFormat for PatternOp {
    fn dsl_format(&self) -> CompactString {
        match self {
            PatternOp::Multiply => "PatternOp::Multiply",
            PatternOp::Max => "PatternOp::Max",
            PatternOp::Min => "PatternOp::Min",
            PatternOp::Average => "PatternOp::Average",
        }
        .into()
    }
}

#[cfg(feature = "dsl")]
impl DslFormat for CombinedPattern {
    fn dsl_format(&self) -> CompactString {
        let ctor = match self.inner.op {
            PatternOp::Multiply => "multiply",
            PatternOp::Max => "max",
            PatternOp::Min => "min",
            PatternOp::Average => "average",
        };
        format_compact!(
            "CombinedPattern::{}({}, {})",
            ctor,
            self.inner.pattern_a.dsl_format(),
            self.inner.pattern_b.dsl_format(),
        )
    }
}

#[cfg(test)]
mod tests {
    use ratatui_core::layout::{Position, Rect};

    use super::*;
    use crate::pattern::{DiagonalPattern, RadialPattern};

    #[test]
    fn test_multiply_both_active() {
        let area = Rect::new(0, 0, 10, 1);
        let pattern = CombinedPattern::multiply(RadialPattern::center(), RadialPattern::center());

        let mut p = pattern.for_frame(1.0, area);
        // both radials at alpha=1 give 1.0, so multiply gives 1.0
        for x in 0..10 {
            let a = p.map_alpha(Position::new(x, 0));
            assert!((a - 1.0).abs() < 1e-5, "multiply(1,1) at x={x}: got {a}");
        }
    }

    #[test]
    fn test_multiply_one_zero() {
        let area = Rect::new(0, 0, 10, 1);
        let pattern = CombinedPattern::multiply(RadialPattern::center(), RadialPattern::center());

        // At alpha=0, both radials yield 0.0
        let mut p = pattern.for_frame(0.0, area);
        for x in 0..10 {
            let a = p.map_alpha(Position::new(x, 0));
            assert!(a == 0.0, "multiply at alpha=0, x={x}: got {a}");
        }
    }

    #[test]
    fn test_max_takes_larger() {
        let area = Rect::new(0, 0, 20, 1);
        let radial = RadialPattern::center(); // peaks at center
        let diagonal = DiagonalPattern::top_left_to_bottom_right();

        let mut r = radial.for_frame(0.5, area);
        let mut d = diagonal.for_frame(0.5, area);
        let mut combined = CombinedPattern::max(radial, diagonal).for_frame(0.5, area);

        for x in 0..20 {
            let pos = Position::new(x, 0);
            let ra = r.map_alpha(pos);
            let da = d.map_alpha(pos);
            let ca = combined.map_alpha(pos);
            let expected = if ra > da { ra } else { da };
            assert!(
                (ca - expected).abs() < 1e-5,
                "max at x={x}: expected {expected}, got {ca} (radial={ra}, diagonal={da})"
            );
        }
    }

    #[test]
    fn test_min_takes_smaller() {
        let area = Rect::new(0, 0, 20, 1);
        let radial = RadialPattern::center();
        let diagonal = DiagonalPattern::top_left_to_bottom_right();

        let mut r = radial.for_frame(0.5, area);
        let mut d = diagonal.for_frame(0.5, area);
        let mut combined = CombinedPattern::min(radial, diagonal).for_frame(0.5, area);

        for x in 0..20 {
            let pos = Position::new(x, 0);
            let ra = r.map_alpha(pos);
            let da = d.map_alpha(pos);
            let ca = combined.map_alpha(pos);
            let expected = if ra < da { ra } else { da };
            assert!(
                (ca - expected).abs() < 1e-5,
                "min at x={x}: expected {expected}, got {ca}"
            );
        }
    }

    #[test]
    fn test_average_is_mean() {
        let area = Rect::new(0, 0, 20, 1);
        let radial = RadialPattern::center();
        let diagonal = DiagonalPattern::top_left_to_bottom_right();

        let mut r = radial.for_frame(0.5, area);
        let mut d = diagonal.for_frame(0.5, area);
        let mut combined = CombinedPattern::average(radial, diagonal).for_frame(0.5, area);

        for x in 0..20 {
            let pos = Position::new(x, 0);
            let ra = r.map_alpha(pos);
            let da = d.map_alpha(pos);
            let ca = combined.map_alpha(pos);
            let expected = (ra + da) * 0.5;
            assert!(
                (ca - expected).abs() < 1e-5,
                "average at x={x}: expected {expected}, got {ca}"
            );
        }
    }
}
