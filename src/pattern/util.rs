/// Utility for mapping from a custom range to 0.0..=1.0
#[derive(Clone, Debug, Copy)]
pub struct ProgressionMapper {
    from: f32,
    to: f32,
}

impl ProgressionMapper {
    /// Create a new progression mapper from a custom range to 0.0..=1.0
    ///
    /// # Arguments
    /// * `from` - Start value of the source range
    /// * `to` - End value of the source range
    pub fn new(from: f32, to: f32) -> Self {
        Self { from, to }
    }

    /// Remaps an effects global `alpha` to a new range and then clamps it to `0.0..=1.0`.
    ///
    /// # Arguments
    /// * `global_alpha` - The input alpha value (typically 0.0..=1.0) to be remapped
    ///
    /// # Returns
    /// Mapped value in range 0.0..=1.0
    pub fn map(&self, global_alpha: f32) -> f32 {
        self.map_unclamped(global_alpha).clamp(0.0, 1.0)
    }

    fn map_unclamped(&self, global_alpha: f32) -> f32 {
        let range = self.to - self.from;
        if range.abs() < f32::EPSILON {
            return 1.0;
        }

        self.from + (global_alpha * range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progression_mapper_functionality() {
        // Test basic mapping
        let mapper = ProgressionMapper::new(-0.5, 1.5);
        assert_eq!(mapper.map_unclamped(0.0), -0.5); // clamped to 0.0
        assert_eq!(mapper.map_unclamped(0.5), 0.5);
        assert_eq!(mapper.map_unclamped(1.0), 1.5);

        // Test with_negative_start
        let mapper = ProgressionMapper::new(-0.3, 1.0);
        assert!((mapper.map_unclamped(0.0) - (-0.3)).abs() < 0.001);
        assert!((mapper.map_unclamped(1.0) - 1.0).abs() < 0.001);

        // Test intermediate value
        let expected = -0.3 + 0.5 * (1.0 - (-0.3)); // -0.3 + 0.5 * 1.3 = -0.3 + 0.65 = 0.35
        assert!((mapper.map_unclamped(0.5) - expected).abs() < 0.001);
    }
}
