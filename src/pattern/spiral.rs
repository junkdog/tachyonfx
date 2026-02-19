#[cfg(feature = "dsl")]
use compact_str::{format_compact, CompactString, ToCompactString};
use ratatui_core::layout::{Position, Rect};

#[cfg(feature = "dsl")]
use crate::dsl::{dsl_format::fmt_f32, DslFormat};
use crate::pattern::{InstancedPattern, Pattern, PreparedPattern};

/// Spiral arm reveal pattern using diamond pseudo-angle and Manhattan distance.
///
/// Produces spiral-arm shaped reveals without any trigonometry or `sqrt`.
/// Uses `diamond_angle` for angular measurement (monotonic with true angle,
/// 1 division + 2-3 comparisons) and Manhattan distance for radial distance.
#[derive(Clone, Debug, Copy, PartialEq)]
pub struct SpiralPattern {
    center_x: f32,
    center_y: f32,
    arms: u16,
    transition_width: f32,
}

impl SpiralPattern {
    /// Creates a spiral pattern centered at the middle of the area with default
    /// transition width and 1 arm
    pub fn center() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            arms: 1,
            transition_width: 2.0,
        }
    }

    /// Creates a spiral pattern with custom center point (0.0-1.0 normalized
    /// coordinates), default transition width, and 1 arm
    pub fn new(center_x: f32, center_y: f32) -> Self {
        Self {
            center_x: center_x.clamp(0.0, 1.0),
            center_y: center_y.clamp(0.0, 1.0),
            arms: 1,
            transition_width: 2.0,
        }
    }

    /// Creates a spiral pattern with custom center and transition width
    pub fn with_transition(center: (f32, f32), transition_width: f32) -> Self {
        let (center_x, center_y) = center;
        Self {
            center_x: center_x.clamp(0.0, 1.0),
            center_y: center_y.clamp(0.0, 1.0),
            arms: 1,
            transition_width: transition_width.max(0.1),
        }
    }

    /// Sets the transition width for gradient smoothing
    pub fn with_transition_width(mut self, width: f32) -> Self {
        self.transition_width = width.max(0.1);
        self
    }

    /// Sets a custom center point for the spiral pattern
    pub fn with_center(mut self, center: (f32, f32)) -> Self {
        let (center_x, center_y) = center;
        self.center_x = center_x.clamp(0.0, 1.0);
        self.center_y = center_y.clamp(0.0, 1.0);
        self
    }

    /// Sets the number of spiral arms
    pub fn with_arms(mut self, arms: u16) -> Self {
        self.arms = arms.max(1);
        self
    }
}

/// Precomputed per-frame state for [`SpiralPattern`].
pub struct SpiralContext {
    center_x: f32,
    center_y: f32,
    max_distance: f32,
    arms: f32,
    scaled_alpha: f32,
    inv_transition_width: f32,
    transition_width: f32,
}

impl Pattern for SpiralPattern {
    type Context = SpiralContext;

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
        }
        .max(1.0); // avoid division by zero

        let arms = (self.arms.max(1)) as f32;
        // Scale so alpha=0 → all inactive, alpha=1 → all active
        // spiral ∈ [0,1), so we need:
        //   alpha=0 → scaled < 0 (inactive)
        //   alpha=1 → scaled - spiral ≥ tw for all spiral in [0,1)
        let scaled_alpha = alpha * (1.0 + 2.0 * transition_width) - transition_width;

        PreparedPattern {
            pattern: self,
            context: SpiralContext {
                center_x,
                center_y,
                max_distance,
                arms,
                scaled_alpha,
                inv_transition_width: 1.0 / transition_width,
                transition_width,
            },
        }
    }
}

impl InstancedPattern for PreparedPattern<SpiralContext, SpiralPattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        let ctx = &self.context;

        let dx = pos.x as f32 - ctx.center_x;
        let dy = (pos.y as f32 - ctx.center_y) * 2.0; // aspect-ratio correction

        let angle = diamond_angle(dx, dy) / 4.0; // normalize to 0..1
        let dist = (abs_f32(dx) + abs_f32(dy)) / ctx.max_distance; // 0..1

        // spiral arm position: fractional part of (angle * arms + dist)
        let spiral = fract(angle * ctx.arms + dist);

        // threshold comparison with gradient transition
        let diff = ctx.scaled_alpha - spiral;
        if diff >= ctx.transition_width {
            1.0
        } else if diff > 0.0 {
            (diff * ctx.inv_transition_width).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// Diamond pseudo-angle: returns a value in 0..4 that increases
/// monotonically with the true angle, using only comparisons and
/// one division. No trig, no sqrt.
///
/// Quadrants:
/// - 0..1: dx >= 0, dy >= 0  (first quadrant)
/// - 1..2: dx <  0, dy >= 0  (second quadrant)
/// - 2..3: dx <  0, dy <  0  (third quadrant)
/// - 3..4: dx >= 0, dy <  0  (fourth quadrant)
#[inline(always)]
fn diamond_angle(dx: f32, dy: f32) -> f32 {
    let adx = abs_f32(dx);
    let ady = abs_f32(dy);
    let sum = adx + ady;
    if sum < 1e-10 {
        return 0.0; // at center
    }
    let p = dx / sum; // -1..1
    if dy >= 0.0 {
        1.0 - p // 0..2
    } else {
        3.0 + p // 2..4
    }
}

/// Fractional part of a float, always non-negative.
#[inline(always)]
fn fract(x: f32) -> f32 {
    x - floor(x)
}

#[inline(always)]
fn floor(x: f32) -> f32 {
    let i = x as i32;
    if (i as f32) > x {
        (i - 1) as f32
    } else {
        i as f32
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
impl DslFormat for SpiralPattern {
    fn dsl_format(&self) -> CompactString {
        let base = if (self.center_x - 0.5).abs() < f32::EPSILON
            && (self.center_y - 0.5).abs() < f32::EPSILON
            && (self.transition_width - 2.0).abs() < f32::EPSILON
        {
            "SpiralPattern::center()".to_compact_string()
        } else if (self.transition_width - 2.0).abs() < f32::EPSILON {
            format_compact!(
                "SpiralPattern::new({}, {})",
                fmt_f32(self.center_x),
                fmt_f32(self.center_y)
            )
        } else {
            format_compact!(
                "SpiralPattern::with_transition(({}, {}), {})",
                fmt_f32(self.center_x),
                fmt_f32(self.center_y),
                fmt_f32(self.transition_width)
            )
        };

        if self.arms != 1 {
            format_compact!("{}.with_arms({})", base, self.arms)
        } else {
            base
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use ratatui_core::layout::{Position, Rect};

    use super::*;

    #[test]
    fn test_spiral_boundary_alphas() {
        let area = Rect::new(0, 0, 10, 5);
        let pattern = SpiralPattern::center().with_transition_width(2.0);

        // alpha=0 => all cells should be 0.0
        let mut p = pattern.for_frame(0.0, area);
        for x in 0..10 {
            for y in 0..5 {
                let a = p.map_alpha(Position::new(x, y));
                assert!(a == 0.0, "alpha=0: expected 0.0 at ({x},{y}), got {a}");
            }
        }

        // alpha=1 => all cells should be 1.0
        let mut p = pattern.for_frame(1.0, area);
        for x in 0..10 {
            for y in 0..5 {
                let a = p.map_alpha(Position::new(x, y));
                assert!(a > 0.99, "alpha=1: expected ~1.0 at ({x},{y}), got {a}");
            }
        }
    }

    #[test]
    fn test_spiral_arm_count() {
        // With more arms, the spiral should have more distinct "bands"
        let area = Rect::new(0, 0, 20, 10);
        let pattern_1 = SpiralPattern::center().with_arms(1);
        let pattern_3 = SpiralPattern::center().with_arms(3);

        let mut p1 = pattern_1.for_frame(0.5, area);
        let mut p3 = pattern_3.for_frame(0.5, area);

        // Count transitions (alpha changes direction) along a horizontal line
        let count_transitions = |p: &mut PreparedPattern<SpiralContext, SpiralPattern>| {
            let mut transitions = 0;
            let mut prev_alpha = p.map_alpha(Position::new(0, 5));
            for x in 1..20 {
                let alpha = p.map_alpha(Position::new(x, 5));
                if (alpha - prev_alpha).abs() > 0.1 {
                    transitions += 1;
                }
                prev_alpha = alpha;
            }
            transitions
        };

        let t1 = count_transitions(&mut p1);
        let t3 = count_transitions(&mut p3);

        // 3 arms should produce more transitions than 1 arm
        assert!(
            t3 >= t1,
            "3 arms should have >= transitions than 1 arm: 1-arm={t1}, 3-arm={t3}"
        );
    }

    #[test]
    fn test_spiral_animation_progression() {
        let area = Rect::new(0, 0, 10, 5);
        let pattern = SpiralPattern::center().with_transition_width(2.0);
        let test_pos = Position::new(8, 3);

        let mut alphas = Vec::new();
        for i in 0..=10 {
            let global_alpha = i as f32 / 10.0;
            let mut prepared = pattern.for_frame(global_alpha, area);
            let alpha = prepared.map_alpha(test_pos);
            alphas.push(alpha);
        }

        // Overall: more cells should be active at the end than at the start
        let sum_early: f32 = alphas[0..3].iter().sum();
        let sum_late: f32 = alphas[8..=10].iter().sum();
        assert!(
            sum_late > sum_early,
            "Later frames should have more active cells: early_sum={sum_early:.3}, late_sum={sum_late:.3}",
        );
    }

    #[test]
    fn test_diamond_angle_monotonic() {
        // Test that diamond_angle increases monotonically around the circle
        let n = 100;
        let mut prev_angle = diamond_angle(1.0, 0.0); // start at 0 degrees
        for i in 1..n {
            let theta = (i as f32 / n as f32) * core::f32::consts::TAU;
            let dx = micromath::F32Ext::cos(theta);
            let dy = micromath::F32Ext::sin(theta);
            let angle = diamond_angle(dx, dy);
            // Allow for the wrap-around at 4.0 -> 0.0
            if angle < prev_angle && (prev_angle - angle) < 3.5 {
                panic!("diamond_angle not monotonic at i={i}: prev={prev_angle}, cur={angle}");
            }
            prev_angle = angle;
        }
    }
}
