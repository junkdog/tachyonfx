use ratatui::style::Color;

pub trait ToRgbComponents {
    fn to_rgb(&self) -> (u8, u8, u8);
}

impl ToRgbComponents for Color {
    fn to_rgb(&self) -> (u8, u8, u8) {
        match self {
            Color::Rgb(r, g, b) => (*r, *g, *b),
            Color::Reset => (0, 0, 0),
            Color::Black => (0, 0, 0),
            Color::Red => (255, 0, 0),
            Color::Green => (0, 255, 0),
            Color::Yellow => (255, 255, 0),
            Color::Blue => (0, 0, 255),
            Color::Magenta => (255, 0, 255),
            Color::Cyan => (0, 255, 255),
            Color::Gray => (128, 128, 128),
            Color::DarkGray => (64, 64, 64),
            Color::LightRed => (255, 128, 128),
            Color::LightGreen => (128, 255, 128),
            Color::LightYellow => (255, 255, 128),
            Color::LightBlue => (128, 128, 255),
            Color::LightMagenta => (255, 128, 255),
            Color::LightCyan => (128, 255, 255),
            Color::White => (255, 255, 255),
            Color::Indexed(code) => {
                let rgb = colorsys::Ansi256::new(*code).as_rgb();
                (rgb.red().round() as u8, rgb.green().round() as u8, rgb.blue().round() as u8)
            },
        }
    }
}

pub trait AsIndexedColor {
    fn as_indexed_color(&self) -> Color;
}

impl AsIndexedColor for Color {
    fn as_indexed_color(&self) -> Color {
        let (r, g, b) = self.to_rgb();

        let c = colorsys::Rgb::from([r as f64, g as f64, b as f64]);
        let ansi256 = colorsys::Ansi256::from(c);
        Color::Indexed(ansi256.code())
    }
}