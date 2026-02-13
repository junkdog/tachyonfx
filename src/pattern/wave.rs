use alloc::vec;

use ratatui_core::layout::{Position, Rect};

use crate::{
    features::Shared,
    math,
    pattern::{InstancedPattern, Pattern, PreparedPattern},
    wave::{SignalSampler, WaveLayer},
};

/// A pattern driven by wave interference.
///
/// Uses one or more [`WaveLayer`]s to produce a spatial alpha field.
/// The combined signal (in −1..1) is rescaled to 0..1 for use as an
/// alpha value, and then raised to a configurable contrast exponent
/// before being mixed with the global animation progress.
#[derive(Clone, Debug)]
pub struct WavePattern {
    layers: Shared<[WaveLayer]>,
    /// Exponent applied after normalisation; >1 increases contrast.
    contrast: i32,
    /// Width of the soft transition between active/inactive cells (0..1 normalised
    /// space).
    transition_width: f32,
}

impl PartialEq for WavePattern {
    fn eq(&self, other: &Self) -> bool {
        self.contrast == other.contrast
            && self.transition_width == other.transition_width
            && *self.layers == *other.layers
    }
}

#[allow(dead_code)]
impl WavePattern {
    /// Creates a wave pattern from a single layer.
    pub fn new(layer: WaveLayer) -> Self {
        Self {
            layers: Shared::from(vec![layer]),
            contrast: 1,
            transition_width: 0.15,
        }
    }

    /// Adds a layer to the pattern.
    pub fn with_layer(self, layer: WaveLayer) -> Self {
        let mut layers = self.layers.to_vec();
        layers.push(layer);
        Self {
            layers: Shared::from(layers),
            contrast: self.contrast,
            transition_width: self.transition_width,
        }
    }

    /// Sets the contrast exponent (default 1).
    /// Values >1 push the pattern toward black/white extremes.
    pub fn with_contrast(mut self, contrast: i32) -> Self {
        self.contrast = contrast;
        self
    }

    pub(crate) fn layers(&self) -> &[WaveLayer] {
        &self.layers
    }

    pub(crate) fn contrast(&self) -> i32 {
        self.contrast
    }

    /// Sets the transition width for the soft edge between active/inactive cells.
    /// The value is in normalised [0,1] space (default 0.15). Clamped to >= 0.01.
    pub fn with_transition_width(mut self, width: f32) -> Self {
        self.transition_width = width.max(0.01);
        self
    }

    pub(crate) fn transition_width(&self) -> f32 {
        self.transition_width
    }
}

impl SignalSampler for WavePattern {
    fn sample(&self, x: f32, y: f32, t: f32) -> f32 {
        let mut sum = 0.0f32;
        for layer in self.layers.iter() {
            sum += layer.sample(x, y, t);
        }

        // normalise from [-layer_count..layer_count] to [0..1]
        let n = self.layers.len() as f32;
        let normalised = (sum / n + 1.0) * 0.5;

        if self.contrast != 1 {
            math::powi(normalised.clamp(0.0, 1.0), self.contrast)
        } else {
            normalised.clamp(0.0, 1.0)
        }
    }
}

/// Per-frame evaluation context for [`WavePattern`].
///
/// Holds precomputed values derived from the current animation progress
/// and effect area, allowing efficient per-cell alpha computation.
pub struct WavePatternContext {
    /// Animation progress, used as the time parameter for wave sampling.
    alpha: f32,
    /// `1.0 - alpha`; the activation threshold that sweeps from 1→0.
    threshold: f32,
    /// `1.0 / transition_width`
    inv_transition_width: f32,
    /// `1.0 / layer_count`
    inv_n: f32,
    /// Precomputed area origin x as f32.
    area_x: f32,
    /// Precomputed area origin y as f32.
    area_y: f32,
}

impl Pattern for WavePattern {
    type Context = WavePatternContext;

    fn for_frame(self, alpha: f32, area: Rect) -> PreparedPattern<Self::Context, Self>
    where
        Self: Sized,
    {
        let inv_n = 1.0 / self.layers.len() as f32;
        let inv_tw = 1.0 / self.transition_width.max(0.01);
        PreparedPattern {
            pattern: self,
            context: WavePatternContext {
                alpha,
                threshold: 1.0 - alpha,
                inv_transition_width: inv_tw,
                inv_n,
                area_x: area.x as f32,
                area_y: area.y as f32,
            },
        }
    }
}

impl InstancedPattern for PreparedPattern<WavePatternContext, WavePattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        let ctx = &self.context;

        let x = pos.x as f32 - ctx.area_x;
        let y = pos.y as f32 - ctx.area_y;

        // Inlines WavePattern::sample() to use precomputed inv_n;
        // keep in sync with SignalSampler impl for WavePattern.
        let mut sum = 0.0f32;
        for layer in self.pattern.layers.iter() {
            sum += layer.sample(x, y, ctx.alpha);
        }
        let normalised = (sum * ctx.inv_n + 1.0) * 0.5;
        let wave_alpha = if self.pattern.contrast != 1 {
            math::powi(normalised.clamp(0.0, 1.0), self.pattern.contrast)
        } else {
            normalised.clamp(0.0, 1.0)
        };

        if wave_alpha >= ctx.threshold {
            1.0
        } else {
            let distance_below = ctx.threshold - wave_alpha;
            let t = 1.0 - distance_below * ctx.inv_transition_width;
            if t > 0.0 {
                t
            } else {
                0.0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui_core::layout::{Position, Rect};

    use super::*;
    use crate::wave::Oscillator;

    const AREA: Rect = Rect { x: 0, y: 0, width: 20, height: 10 };

    #[test]
    fn alpha_zero_gives_all_inactive() {
        let pattern = WavePattern::new(WaveLayer::new(Oscillator::sin(1.0, 1.0, 0.0)));
        let mut prepared = pattern.for_frame(0.0, AREA);

        // at alpha 0 the threshold is 1.0; wave values are in [0,1] so
        // almost nothing should be fully active
        let alpha = prepared.map_alpha(Position::new(5, 5));
        assert!(alpha < 0.5, "expected low alpha at progress 0, got {alpha}");
    }

    #[test]
    fn alpha_one_gives_all_active() {
        let pattern = WavePattern::new(WaveLayer::new(Oscillator::sin(1.0, 1.0, 0.0)));
        let mut prepared = pattern.for_frame(1.0, AREA);

        // at alpha 1 the threshold is 0.0; all normalised wave values ≥ 0
        for y in 0..AREA.height {
            for x in 0..AREA.width {
                let alpha = prepared.map_alpha(Position::new(x, y));
                assert!(
                    alpha > 0.99,
                    "expected full alpha at progress 1.0, got {alpha} at ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn alpha_values_in_range() {
        let pattern = WavePattern::new(WaveLayer::new(Oscillator::sin(1.0, 0.5, 0.0)))
            .with_layer(WaveLayer::new(Oscillator::cos(0.5, 1.0, 0.0)));

        let mut prepared = pattern.for_frame(0.5, AREA);

        for y in 0..AREA.height {
            for x in 0..AREA.width {
                let alpha = prepared.map_alpha(Position::new(x, y));
                assert!(
                    (0.0..=1.0).contains(&alpha),
                    "alpha out of range: {alpha} at ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn custom_transition_width_affects_alpha() {
        let base_layer = WaveLayer::new(Oscillator::sin(1.0, 1.0, 0.0));

        let narrow = WavePattern::new(base_layer).with_transition_width(0.01);
        let wide = WavePattern::new(base_layer).with_transition_width(0.5);

        // At mid-progress, a wider transition produces more partially-active
        // cells (alpha between 0 and 1) than a narrow one.
        let alpha = 0.5;
        let mut partial_narrow = 0u32;
        let mut partial_wide = 0u32;

        let mut prepared_narrow = narrow.for_frame(alpha, AREA);
        let mut prepared_wide = wide.for_frame(alpha, AREA);

        for y in 0..AREA.height {
            for x in 0..AREA.width {
                let pos = Position::new(x, y);
                let a_narrow = prepared_narrow.map_alpha(pos);
                let a_wide = prepared_wide.map_alpha(pos);

                if a_narrow > 0.0 && a_narrow < 1.0 {
                    partial_narrow += 1;
                }
                if a_wide > 0.0 && a_wide < 1.0 {
                    partial_wide += 1;
                }
            }
        }

        assert!(
            partial_wide >= partial_narrow,
            "wider transition should produce at least as many partial cells: wide={partial_wide}, narrow={partial_narrow}"
        );
    }

    #[test]
    fn contrast_changes_distribution() {
        let base_layer = WaveLayer::new(Oscillator::sin(1.0, 1.0, 0.0));

        let normal = WavePattern::new(base_layer);
        let high_contrast = WavePattern::new(base_layer).with_contrast(3);

        // sample the raw wave values (at alpha=1.0 so threshold=0, all cells active)
        // and verify that contrast shifts the distribution
        let mut sum_normal = 0.0f32;
        let mut sum_contrast = 0.0f32;

        for y in 0..AREA.height {
            for x in 0..AREA.width {
                sum_normal += normal.sample(x as f32, y as f32, 0.0);
                sum_contrast += high_contrast.sample(x as f32, y as f32, 0.0);
            }
        }

        // power > 1 on values in [0,1] pushes them toward 0,
        // so the sum should decrease with higher contrast
        assert!(
            sum_contrast < sum_normal,
            "high contrast (power 3) should reduce average wave value: normal={sum_normal}, contrast={sum_contrast}"
        );
    }
}
