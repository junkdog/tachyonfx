use alloc::vec;

use ratatui_core::layout::{Position, Rect};

use crate::{
    features::Shared,
    math,
    pattern::{InstancedPattern, Pattern, PreparedPattern},
    wave::WaveLayer,
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
}

impl PartialEq for WavePattern {
    fn eq(&self, other: &Self) -> bool {
        self.contrast == other.contrast && *self.layers == *other.layers
    }
}

#[allow(dead_code)]
impl WavePattern {
    /// Creates a wave pattern from a single layer.
    pub fn new(layer: WaveLayer) -> Self {
        Self { layers: Shared::from(vec![layer]), contrast: 1 }
    }

    /// Adds a layer to the pattern.
    pub fn with_layer(self, layer: WaveLayer) -> Self {
        let mut layers = self.layers.to_vec();
        layers.push(layer);
        Self {
            layers: Shared::from(layers),
            contrast: self.contrast,
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

    /// Evaluates all layers and returns the combined signal in 0..1.
    fn sample(&self, x: f32, y: f32, t: f32) -> f32 {
        let mut sum = 0.0f32;
        for layer in self.layers.iter() {
            sum += layer.evaluate(x, y, t);
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

/// Per-frame context for `WavePattern`: global alpha, area, and time derived from alpha.
pub struct WavePatternContext {
    alpha: f32,
    area: Rect,
}

impl Pattern for WavePattern {
    type Context = WavePatternContext;

    fn for_frame(self, alpha: f32, area: Rect) -> PreparedPattern<Self::Context, Self>
    where
        Self: Sized,
    {
        PreparedPattern {
            pattern: self,
            context: WavePatternContext { alpha, area },
        }
    }
}

impl InstancedPattern for PreparedPattern<WavePatternContext, WavePattern> {
    fn map_alpha(&mut self, pos: Position) -> f32 {
        let WavePatternContext { alpha, area } = self.context;

        // normalise position to [0..width/height]
        let x = (pos.x as f32) - (area.x as f32);
        let y = (pos.y as f32) - (area.y as f32);

        // use global alpha as the time parameter so the pattern
        // animates in lockstep with the effect's progress
        let t = alpha;

        let wave_alpha = self.pattern.sample(x, y, t);

        // blend: cells whose wave value exceeds the threshold are active
        // the threshold sweeps from 1→0 as global alpha goes 0→1
        let threshold = 1.0 - alpha;
        if wave_alpha >= threshold {
            1.0
        } else {
            // soft transition: linearly ramp over the last 0.15 of threshold distance
            let transition = 0.15f32;
            let distance_below = threshold - wave_alpha;
            if distance_below < transition {
                1.0 - (distance_below / transition)
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
