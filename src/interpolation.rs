use ratatui::style::{Color, Style};
use simple_easing::{back_in, back_in_out, back_out, bounce_in, bounce_out, circ_in, circ_in_out, circ_out, cubic_in, elastic_in, elastic_in_out, elastic_out, expo_in, expo_in_out, expo_out, quad_in, quad_in_out, quad_out, quart_in, quart_in_out, quart_out, quint_in, quint_in_out, quint_out, reverse, sine_in, sine_in_out, sine_out};

#[derive(Clone, Debug, Default)]
pub enum Interpolation {
    BackIn,
    BackOut,
    BackInOut,

    BounceIn,
    BounceOut,
    BounceInOut,

    CircIn,
    CircOut,
    CircInOut,

    CubicIn,
    CubicOut,
    CubicInOut,

    ElasticIn,
    ElasticOut,
    ElasticInOut,

    ExpoIn,
    ExpoOut,
    ExpoInOut,

    #[default]
    Linear,

    QuadIn,
    QuadOut,
    QuadInOut,

    QuartIn,
    QuartOut,
    QuartInOut,

    QuintIn,
    QuintOut,
    QuintInOut,

    Reverse,

    SineIn,
    SineOut,
    SineInOut,
}

impl Interpolation {

    pub fn alpha(&self, a: f32) -> f32 {
        match self {
            Interpolation::BackIn       => back_in(a),
            Interpolation::BackOut      => back_out(a),
            Interpolation::BackInOut    => back_in_out(a),

            Interpolation::BounceIn     => bounce_in(a),
            Interpolation::BounceOut    => bounce_out(a),
            Interpolation::BounceInOut  => back_in_out(a),

            Interpolation::CircIn       => circ_in(a),
            Interpolation::CircOut      => circ_out(a),
            Interpolation::CircInOut    => circ_in_out(a),

            Interpolation::CubicIn      => cubic_in(a),
            Interpolation::CubicOut     => circ_out(a),
            Interpolation::CubicInOut   => circ_in_out(a),

            Interpolation::ElasticIn    => elastic_in(a),
            Interpolation::ElasticOut   => elastic_out(a),
            Interpolation::ElasticInOut => elastic_in_out(a),

            Interpolation::ExpoIn       => expo_in(a),
            Interpolation::ExpoOut      => expo_out(a),
            Interpolation::ExpoInOut    => expo_in_out(a),

            Interpolation::Linear       => a,

            Interpolation::QuadIn       => quad_in(a),
            Interpolation::QuadOut      => quad_out(a),
            Interpolation::QuadInOut    => quad_in_out(a),

            Interpolation::QuartIn      => quart_in(a),
            Interpolation::QuartOut     => quart_out(a),
            Interpolation::QuartInOut   => quart_in_out(a),

            Interpolation::QuintIn      => quint_in(a),
            Interpolation::QuintOut     => quint_out(a),
            Interpolation::QuintInOut   => quint_in_out(a),

            Interpolation::Reverse      => reverse(a),

            Interpolation::SineIn       => sine_in(a),
            Interpolation::SineOut      => sine_out(a),
            Interpolation::SineInOut    => sine_in_out(a),
        }
    }
}

pub trait Interpolatable<T> {
    fn lerp(&self, target: &T, alpha: f32) -> T;
    
    fn tween(&self, target: &T, alpha: f32, interpolation: Interpolation) -> T {
        self.lerp(target, interpolation.alpha(alpha))
    }
}

impl Interpolatable<u16> for u16 {
    fn lerp(&self, target: &u16, alpha: f32) -> u16 {
        (*self as f32).lerp(
            &(*target as f32),
            alpha
        ).round() as u16
    }
}

impl Interpolatable<i16> for i16 {
    fn lerp(&self, target: &i16, alpha: f32) -> i16 {
        (*self as f32).lerp(
            &(*target as f32),
            alpha
        ).round() as i16
    }
}

impl Interpolatable<f32> for f32 {
    fn lerp(&self, target: &f32, alpha: f32) -> f32 {
        self + (target - self) * alpha
    }
}

impl Interpolatable<Style> for Style {
    fn lerp(&self, target: &Style, alpha: f32) -> Style {
        let fg = self.fg.lerp(&target.fg, alpha);
        let bg = self.bg.lerp(&target.bg, alpha);

        let mut s = *self;
        if let Some(fg) = fg { s = s.fg(fg) }
        if let Some(bg) = bg { s = s.bg(bg) }

        s
    }
}

impl Interpolatable<Color> for Color {
    fn lerp(&self, target: &Color, alpha: f32) -> Color {
        if alpha == 0.0 {
            return *self;
        } else if alpha == 1.0 {
            return *target;
        }
        
        let (h, s, v) = self.to_hsv();
        let (h2, s2, v2) = target.to_hsv();
        Color::from_hsv(
            h.lerp(&h2, alpha),
            s.lerp(&s2, alpha),
            v.lerp(&v2, alpha),
        )
    }
}

impl Interpolatable<Option<Color>> for Option<Color> {
    fn lerp(&self, target: &Option<Color>, alpha: f32) -> Option<Color> {
        match (self, target) {
            (Some(c1), Some(c2)) => Some(c1.lerp(c2, alpha)),
            (Some(c1), None)     => Some(*c1),
            (None,     Some(c2)) => Some(*c2),
            (None,     None)     => None,
        }
    }
}

trait HsvConvertable {
    fn from_hsv(h: f32, s: f32, v: f32) -> Self;
    fn to_hsv(&self) -> (f32, f32, f32);
}

impl HsvConvertable for Color {
    fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        let hsl = colorsys::Hsl::new(h as f64, s as f64, v as f64, None);
        let color: colorsys::Rgb = hsl.as_ref().into();
        
        let red = color.red().round();
        let green = color.green().round();
        let blue = color.blue().round();
        
        Color::Rgb(red as u8, green as u8, blue as u8)
    }

    fn to_hsv(&self) -> (f32, f32, f32) {
        match self {
            Color::Rgb(r, g, b) => {
                let rgb = colorsys::Rgb::from([*r, *g, *b]);
                let hsl: colorsys::Hsl = rgb.as_ref().into();
                (hsl.hue() as f32, hsl.saturation() as f32, hsl.lightness() as f32)
            }
            _ => (0.0, 0.0, 0.0)
        }
    }
}
