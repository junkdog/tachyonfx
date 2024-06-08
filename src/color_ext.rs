use ratatui::style::Color;

pub trait AsIndexedColor {
    fn as_indexed_color(&self) -> Color;
}

impl AsIndexedColor for Color {
    fn as_indexed_color(&self) -> Color {
        match self {
            Color::Rgb(ri, gi, bi) => {
                let c = colorsys::Rgb::from([*ri as f64, *gi as f64, *bi as f64]);
                let ansi256 = colorsys::Ansi256::from(c);
                Color::Indexed(ansi256.code())
            }
            _ => *self
        }
    }
}