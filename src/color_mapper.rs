use ratatui::style::Color;

/// A utility struct for mapping and transforming colors based on
/// a given alpha value. The `ColorMapper` caches the original color
/// and alpha value to avoid redundant transformations.
#[derive(Default)]
pub struct ColorMapper {
    original: (Color, f32),
    transformed: Color
}

impl ColorMapper {
    /// Maps the given color to a transformed color using the provided transformation function.
    /// The transformation is only applied if the input color or alpha value has changed since
    /// the last call.
    ///
    /// # Arguments
    /// * `from_color` - The original color to be transformed.
    /// * `alpha` - The alpha value used for the transformation.
    /// * `transform` - A closure that defines the transformation to be applied to the color.
    ///
    /// # Returns
    /// * The transformed color.
    ///
    /// # Example
    /// ```
    /// use ratatui::style::Color;
    /// use tachyonfx::{ColorMapper, Interpolatable};
    ///
    /// let start = Color::Green;
    /// let target = Color::Red;
    /// let a = 0.5;
    ///
    /// let mut fg_mapper = ColorMapper::default();
    /// let interpolated_color: Color = fg_mapper.map(start, a, |c| c.lerp(&target, a));
    /// ```
    pub fn map(
        &mut self,
        from_color: Color,
        alpha: f32,
        transform: impl Fn(Color) -> Color
    ) -> Color {
        if self.original != (from_color, alpha) {
            self.original = (from_color, alpha);
            self.transformed = transform(from_color);
        }

        self.transformed
    }
}