use ratatui::layout::Rect;

/// A trait that provides a method to calculate a centered, shrunk rectangle
/// within the bounds of the original rectangle.
pub trait CenteredShrink {

    /// Calculates a new rectangle that is centered within the original rectangle
    /// with the specified width and height.
    ///
    /// # Arguments
    /// * `width` - The width of the new centered rectangle.
    /// * `height` - The height of the new centered rectangle.
    ///
    /// # Returns
    /// * A new `Rect` that is centered within the original rectangle with the specified dimensions.
    ///
    /// # Example
    /// ```
    /// use ratatui::layout::Rect;
    /// use tachyonfx::CenteredShrink;
    ///
    /// let original_rect = Rect::new(0, 0, 100, 100);
    /// let centered_rect = original_rect.inner_centered(50, 50);
    ///
    /// assert_eq!(centered_rect, Rect::new(25, 25, 50, 50));
    /// ```
    fn inner_centered(&self, width: u16, height: u16) -> Rect;
}

impl CenteredShrink for Rect {
    fn inner_centered(&self, width: u16, height: u16) -> Rect {
        let x = self.x + (self.width.saturating_sub(width) / 2);
        let y = self.y + (self.height.saturating_sub(height) / 2);
        Rect::new(x, y, width.min(self.width), height.min(self.height))
    }
}
