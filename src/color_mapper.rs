use ratatui::style::Color;
use crate::interpolation::Interpolatable;

#[derive(Default)]
pub struct ColorMapper {
    original: (Color, f32),
    transformed: Color
}

/// note: expects interpolated `alpha`
impl ColorMapper {
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

    pub fn mapping(
        &mut self,
        from_color: Color,
        to_color: &Color,
        alpha: f32,
    ) -> Color {
        self.map(from_color, alpha, |c| c.lerp(to_color, alpha))
    }
}