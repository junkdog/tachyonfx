use ratatui_core::layout::Position;

/// Trait for patterns that can compute per-cell alpha values based on position.
///
/// This trait allows patterns to transform the global animation alpha into
/// position-specific alpha values, creating spatial effects like fades, sweeps,
/// and transitions. Each implementation defines how the pattern reveals or hides
/// cells based on their coordinates.
pub trait InstancedPattern {
    /// Computes the alpha value for a specific cell position.
    ///
    /// # Arguments
    /// * `pos` - The terminal cell position to compute alpha for
    ///
    /// # Returns
    /// Alpha value in range 0.0-1.0, where 0.0 is fully transparent and 1.0 is fully
    /// opaque
    fn map_alpha(&mut self, pos: Position) -> f32;
}
