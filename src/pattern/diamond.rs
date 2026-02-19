#[cfg(feature = "dsl")]
use compact_str::{format_compact, CompactString, ToCompactString};
use ratatui_core::layout::{Position, Rect};

#[cfg(feature = "dsl")]
use crate::dsl::{dsl_format::fmt_f32, DslFormat};
use crate::pattern::{InstancedPattern, Pattern, PreparedPattern};

/// Diamond-shaped reveal pattern using Manhattan distance.
///
/// Like [`RadialPattern`] but uses Manhattan distance (`|dx| + 2·|dy|`)
/// instead of Euclidean distance, producing diamond-shaped reveals.
/// Cheaper per-cell cost (no `sqrt`).
#[derive(Clone, Debug, Copy, PartialEq)]
pub struct DiamondPattern {
    center_x: f32,
    center_y: f32,
    transition_width: f32,
}

impl DiamondPattern {
    /// Creates a diamond pattern centered at the middle of the area with default
    /// transition width
    pub fn center() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            transition_width: 2.0,
        }
    }

    /// Creates a diamond pattern with custom center point (0.0-1.0 normalized
    /// coordinates) and default transition width
    pub fn new(center_x: f32, center_y: f32) -> Self {
        Self {
            center_x: center_x.clamp(0.0, 1.0),
            center_y: center_y.clamp(0.0, 1.0),
            transition_width: 2.0,
        }
    }

    /// Creates a diamond pattern with custom center and transition width
    pub fn with_transition(center: (f32, f32), transition_width: f32) -> Self {
        let (center_x, center_y) = center;
        Self {
            center_x: center_x.clamp(0.0, 1.0),
            center_y: center_y.clamp(0.0, 1.0),
            transition_width: transition_width.max(0.1),
        }
    }

    /// Sets the transition width for gradient smoothing
    pub fn with_transition_width(mut self, width: f32) -> Self {
        self.transition_width = width.max(0.1);
        self
    }

    /// Sets a custom center point for the diamond pattern
    pub fn with_center(mut self, center: (f32, f32)) -> Self {
        let (center_x, center_y) = center;
        self.center_x = center_x.clamp(0.0, 1.0);
        self.center_y = center_y.clamp(0.0, 1.0);
        self
    }
}

/// Precomputed per-frame state for [`DiamondPattern`].
pub struct DiamondContext {
    center_x: f32,
    center_y: f32,
    threshold: f32,
    threshold_end: f32,
    inv_transition_width: f32,
}

impl Pattern for DiamondPattern {
    type Context = DiamondContext;

    fn for_frame(self, alpha: f32, area: Rect) -> PreparedPattern<Self::Context, Self>
    where
        Self: Sized,
    {
        let center_x = area.x as f32 + (self.center_x * area.width as f32);
        let center_y = area.y as f32 + (self.center_y * area.height as f32);
        let transition_width = self.transition_width.max(0.1);

        let max_distance = {
            let corners = [
                (area.x as f32, area.y as f32),
                (area.right() as f32, area.y as f32),
                (area.x as f32, area.bottom() as f32),
                (area.right() as f32, area.bottom() as f32),
            ];
            corners
                .iter()
                .map(|(x, y)| {
                    let dx = x - center_x;
                    let dy = y - center_y;
                    abs_f32(dx) + 2.0 * abs_f32(dy)
                })
                .fold(0.0f32, f32::max)
        };

        let threshold = (alpha * (max_distance + 2.0 * transition_width)) - transition_width;

        PreparedPattern {
            pattern: self,
            context: DiamondContext {
                center_x,
                center_y,
                threshold,
                threshold_end: threshold + transition_width,
                inv_transition_width: 1.0 / transition_width,
            },
        }
    }
}

impl InstancedPattern for PreparedPattern<DiamondContext, DiamondPattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        let ctx = &self.context;

        let dx = pos.x as f32 - ctx.center_x;
        let dy = pos.y as f32 - ctx.center_y;
        // Manhattan distance with aspect-ratio correction (terminal cells ~2:1)
        let distance = abs_f32(dx) + 2.0 * abs_f32(dy);

        if distance <= ctx.threshold {
            1.0
        } else if distance <= ctx.threshold_end {
            let distance_into_transition = distance - ctx.threshold;
            1.0 - (distance_into_transition * ctx.inv_transition_width).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

#[inline(always)]
fn abs_f32(x: f32) -> f32 {
    if x < 0.0 {
        -x
    } else {
        x
    }
}

#[cfg(feature = "dsl")]
impl DslFormat for DiamondPattern {
    fn dsl_format(&self) -> CompactString {
        if (self.center_x - 0.5).abs() < f32::EPSILON
            && (self.center_y - 0.5).abs() < f32::EPSILON
            && (self.transition_width - 2.0).abs() < f32::EPSILON
        {
            "DiamondPattern::center()".to_compact_string()
        } else if (self.transition_width - 2.0).abs() < f32::EPSILON {
            format_compact!(
                "DiamondPattern::new({}, {})",
                fmt_f32(self.center_x),
                fmt_f32(self.center_y)
            )
        } else {
            format_compact!(
                "DiamondPattern::with_transition(({}, {}), {})",
                fmt_f32(self.center_x),
                fmt_f32(self.center_y),
                fmt_f32(self.transition_width)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use ratatui_core::layout::{Position, Rect};

    use super::*;

    #[test]
    fn test_diamond_boundary_alphas() {
        let area = Rect::new(0, 0, 10, 1);
        let pattern = DiamondPattern::center().with_transition_width(2.0);

        // alpha=0 => all cells should be 0.0
        let mut p = pattern.for_frame(0.0, area);
        for x in 0..10 {
            let a = p.map_alpha(Position::new(x, 0));
            assert!(a == 0.0, "alpha=0: expected 0.0 at x={x}, got {a}");
        }

        // alpha=1 => all cells should be 1.0
        let mut p = pattern.for_frame(1.0, area);
        for x in 0..10 {
            let a = p.map_alpha(Position::new(x, 0));
            assert!(a == 1.0, "alpha=1: expected 1.0 at x={x}, got {a}");
        }
    }

    #[test]
    fn test_diamond_symmetry() {
        let area = Rect::new(0, 0, 20, 1);
        let pattern = DiamondPattern::center().with_transition_width(2.0);
        let mut p = pattern.for_frame(0.5, area);

        // center at x=10; check x=8 vs x=12 (both 2 cells away)
        let left = p.map_alpha(Position::new(8, 0));
        let right = p.map_alpha(Position::new(12, 0));
        assert!(
            (left - right).abs() < 1e-5,
            "Symmetric positions should have equal alpha: left={left}, right={right}"
        );
    }

    #[test]
    fn test_diamond_monotonic_from_center() {
        let area = Rect::new(0, 0, 40, 1);
        let pattern = DiamondPattern::center().with_transition_width(3.0);
        let mut p = pattern.for_frame(0.5, area);

        let center_x = 20u16;
        let mut prev_alpha = p.map_alpha(Position::new(center_x, 0));
        for d in 1..20u16 {
            let a = p.map_alpha(Position::new(center_x + d, 0));
            assert!(
                a <= prev_alpha + 1e-5,
                "Alpha should decrease with distance: at d={d}, got {a} > prev {prev_alpha}"
            );
            prev_alpha = a;
        }
    }

    #[test]
    fn test_diamond_animation_progression() {
        let area = Rect::new(0, 0, 10, 1);
        let pattern = DiamondPattern::center().with_transition_width(2.0);
        let test_pos = Position::new(8, 0);

        let mut alphas = Vec::new();
        for i in 0..=10 {
            let global_alpha = i as f32 / 10.0;
            let mut prepared = pattern.for_frame(global_alpha, area);
            let alpha = prepared.map_alpha(test_pos);
            alphas.push(alpha);
        }

        let early_alpha = alphas[2];
        let late_alpha = alphas[8];
        assert!(
            late_alpha > early_alpha,
            "Animation should progress: early alpha={early_alpha:.3}, late alpha={late_alpha:.3}",
        );

        let start_alpha = alphas[0];
        assert!(
            start_alpha < 0.1,
            "At animation start, distant position should be inactive, got alpha={start_alpha:.3}",
        );

        let final_alpha = alphas[10];
        assert!(
            final_alpha > 0.8,
            "At animation end, position should be mostly active, got alpha={final_alpha:.3}",
        );
    }
}
