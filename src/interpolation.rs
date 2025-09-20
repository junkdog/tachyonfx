use ratatui::{
    layout::Offset,
    style::{Color, Style},
};
use simple_easing::{
    back_in, back_in_out, back_out, bounce_in, bounce_in_out, bounce_out, circ_in, circ_in_out,
    circ_out, cubic_in, cubic_in_out, cubic_out, elastic_in, elastic_in_out, elastic_out, expo_in,
    expo_in_out, expo_out, quad_in, quad_in_out, quad_out, quart_in, quart_in_out, quart_out,
    quint_in, quint_in_out, quint_out, reverse, sine_in, sine_in_out, sine_out,
};

use crate::{color_space::hsl_to_rgb, color_to_hsl, ColorSpace};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
            Interpolation::BackIn => back_in(a),
            Interpolation::BackOut => back_out(a),
            Interpolation::BackInOut => back_in_out(a),

            Interpolation::BounceIn => bounce_in(a),
            Interpolation::BounceOut => bounce_out(a),
            Interpolation::BounceInOut => bounce_in_out(a),

            Interpolation::CircIn => circ_in(a),
            Interpolation::CircOut => circ_out(a),
            Interpolation::CircInOut => circ_in_out(a),

            Interpolation::CubicIn => cubic_in(a),
            Interpolation::CubicOut => cubic_out(a),
            Interpolation::CubicInOut => cubic_in_out(a),

            Interpolation::ElasticIn => elastic_in(a),
            Interpolation::ElasticOut => elastic_out(a),
            Interpolation::ElasticInOut => elastic_in_out(a),

            Interpolation::ExpoIn => expo_in(a),
            Interpolation::ExpoOut => expo_out(a),
            Interpolation::ExpoInOut => expo_in_out(a),

            Interpolation::Linear => a,

            Interpolation::QuadIn => quad_in(a),
            Interpolation::QuadOut => quad_out(a),
            Interpolation::QuadInOut => quad_in_out(a),

            Interpolation::QuartIn => quart_in(a),
            Interpolation::QuartOut => quart_out(a),
            Interpolation::QuartInOut => quart_in_out(a),

            Interpolation::QuintIn => quint_in(a),
            Interpolation::QuintOut => quint_out(a),
            Interpolation::QuintInOut => quint_in_out(a),

            Interpolation::Reverse => reverse(a),

            Interpolation::SineIn => sine_in(a),
            Interpolation::SineOut => sine_out(a),
            Interpolation::SineInOut => sine_in_out(a),
        }
    }

    pub fn flipped(&self) -> Self {
        use Interpolation::*;
        match self {
            BackIn => BackOut,
            BackOut => BackIn,
            BackInOut => BackInOut,

            BounceIn => BounceOut,
            BounceOut => BounceIn,
            BounceInOut => BounceInOut,

            CircIn => CircOut,
            CircOut => CircIn,
            CircInOut => CircInOut,

            CubicIn => CubicOut,
            CubicOut => CubicIn,
            CubicInOut => CubicInOut,

            ElasticIn => ElasticOut,
            ElasticOut => ElasticIn,
            ElasticInOut => ElasticInOut,

            ExpoIn => ExpoOut,
            ExpoOut => ExpoIn,
            ExpoInOut => ExpoInOut,

            Linear => Linear,

            QuadIn => QuadOut,
            QuadOut => QuadIn,
            QuadInOut => QuadInOut,

            QuartIn => QuartOut,
            QuartOut => QuartIn,
            QuartInOut => QuartInOut,

            QuintIn => QuintOut,
            QuintOut => QuintIn,
            QuintInOut => QuintInOut,

            Reverse => Reverse,

            SineIn => SineOut,
            SineOut => SineIn,
            SineInOut => SineInOut,
        }
    }
}

/// A trait for interpolating between two values.
pub trait Interpolatable {
    fn lerp(&self, target: &Self, alpha: f32) -> Self;

    fn tween(&self, target: &Self, alpha: f32, interpolation: Interpolation) -> Self
    where
        Self: Sized,
    {
        self.lerp(target, interpolation.alpha(alpha))
    }
}

impl<T: Interpolatable> Interpolatable for (T, T) {
    fn lerp(&self, target: &(T, T), alpha: f32) -> (T, T) {
        (self.0.lerp(&target.0, alpha), self.1.lerp(&target.1, alpha))
    }
}

impl Interpolatable for u16 {
    fn lerp(&self, target: &u16, alpha: f32) -> u16 {
        (*self as f32)
            .lerp(&(*target as f32), alpha)
            .round() as u16
    }
}

impl Interpolatable for i16 {
    fn lerp(&self, target: &i16, alpha: f32) -> i16 {
        (*self as f32)
            .lerp(&(*target as f32), alpha)
            .round() as i16
    }
}

impl Interpolatable for f32 {
    fn lerp(&self, target: &f32, alpha: f32) -> f32 {
        self + (target - self) * alpha
    }
}

impl Interpolatable for i32 {
    fn lerp(&self, target: &i32, alpha: f32) -> i32 {
        self + ((target - self) as f64 * alpha as f64).round() as i32
    }
}

impl Interpolatable for Style {
    fn lerp(&self, target: &Style, alpha: f32) -> Style {
        let fg = self.fg.lerp(&target.fg, alpha);
        let bg = self.bg.lerp(&target.bg, alpha);

        let mut s = *self;
        if let Some(fg) = fg {
            s = s.fg(fg)
        }
        if let Some(bg) = bg {
            s = s.bg(bg)
        }

        s
    }
}

impl Interpolatable for Color {
    fn lerp(&self, target: &Color, alpha: f32) -> Color {
        if alpha == 0.0 {
            return *self;
        } else if alpha == 1.0 {
            return *target;
        }

        ColorSpace::Hsl.lerp(self, target, alpha)
    }
}

impl Interpolatable for Option<Color> {
    fn lerp(&self, target: &Option<Color>, alpha: f32) -> Option<Color> {
        match (self, target) {
            (Some(c1), Some(c2)) => Some(c1.lerp(c2, alpha)),
            (Some(c1), None) => Some(*c1),
            (None, Some(c2)) => Some(*c2),
            (None, None) => None,
        }
    }
}

impl Interpolatable for Offset {
    fn lerp(&self, target: &Offset, alpha: f32) -> Offset {
        Offset {
            x: self.x.lerp(&target.x, alpha),
            y: self.y.lerp(&target.y, alpha),
        }
    }
}

#[deprecated(
    since = "0.12.0",
    note = "Replaced by ColorSpace and associated functions"
)]
pub trait HslConvertable {
    fn from_hsl_f32(h: f32, s: f32, v: f32) -> Self;
    fn to_hsl_f32(&self) -> (f32, f32, f32);
}

#[allow(deprecated)]
impl HslConvertable for Color {
    fn from_hsl_f32(h: f32, s: f32, v: f32) -> Self {
        let (r, g, b) = hsl_to_rgb(h, s, v);
        Color::Rgb(r, g, b)
    }

    fn to_hsl_f32(&self) -> (f32, f32, f32) {
        color_to_hsl(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_alpha_steps() -> Vec<f32> {
        (0..=10).map(|i| i as f32 / 10.0).collect()
    }

    #[test]
    fn test_back_in() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::BackIn.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            -0.014314221,
            -0.046450563,
            -0.08019955,
            -0.09935169,
            -0.087697506,
            -0.029027522,
            0.09286779,
            0.2941978,
            0.59117186,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_back_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::BackOut.alpha(a))
            .collect();
        let expected = vec![
            0.0, 0.40882802, 0.7058022, 0.90713227, 1.0290275, 1.0876975, 1.0993516, 1.0801995,
            1.0464505, 1.0143142, 1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_back_in_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::BackInOut.alpha(a))
            .collect();
        let expected = vec![
            -0.0,
            -0.037518553,
            -0.09255566,
            -0.078833476,
            0.08992585,
            0.5,
            0.91007423,
            1.0788335,
            1.0925556,
            1.0375186,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_bounce_in() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::BounceIn.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.011875033,
            0.060000002,
            0.06937504,
            0.22750002,
            0.234375,
            0.089999914,
            0.31937492,
            0.6975,
            0.92437494,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_bounce_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::BounceOut.alpha(a))
            .collect();
        let expected = vec![
            0.0, 0.075625, 0.3025, 0.6806251, 0.91, 0.765625, 0.7725, 0.93062496, 0.94, 0.98812497,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_bounce_in_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::BounceInOut.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.030000001,
            0.11375001,
            0.044999957,
            0.34875,
            0.5,
            0.65125006,
            0.95500004,
            0.88625,
            0.97,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_circ_in() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::CircIn.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.005012572,
            0.020204127,
            0.0460608,
            0.08348489,
            0.13397461,
            0.19999999,
            0.28585714,
            0.40000004,
            0.56411004,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_circ_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::CircOut.alpha(a))
            .collect();
        let expected = vec![
            0.0, 0.43588996, 0.59999996, 0.71414286, 0.8, 0.8660254, 0.9165152, 0.9539392,
            0.9797959, 0.9949874, 1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_circ_in_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::CircInOut.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.010102063,
            0.041742444,
            0.099999994,
            0.20000002,
            0.5,
            0.8000001,
            0.9,
            0.95825756,
            0.98989797,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_cubic_in() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::CubicIn.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.001,
            0.008,
            0.027000003,
            0.064,
            0.125,
            0.21600002,
            0.343,
            0.512,
            0.7289999,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_cubic_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::CubicOut.alpha(a))
            .collect();
        let expected =
            vec![0.0, 0.2710001, 0.48799998, 0.657, 0.784, 0.875, 0.936, 0.973, 0.992, 0.999, 1.0];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_cubic_in_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::CubicInOut.alpha(a))
            .collect();
        let expected = vec![
            0.0, 0.004, 0.032, 0.10800001, 0.256, 0.5, 0.7440001, 0.89199996, 0.968, 0.996, 1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_elastic_in() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::ElasticIn.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.001953125,
            -0.0019531213,
            -0.0039062474,
            0.015625,
            -0.015624988,
            -0.031249996,
            0.125,
            -0.12499994,
            -0.2500001,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_elastic_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::ElasticOut.alpha(a))
            .collect();
        let expected = vec![
            0.0, 1.25, 1.125, 0.875, 1.03125, 1.015625, 0.984375, 1.0039063, 1.0019531, 0.9980469,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_elastic_in_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::ElasticInOut.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.0003391572,
            -0.003906256,
            0.023938898,
            -0.117461585,
            0.5,
            1.1174616,
            0.9760611,
            1.0039063,
            0.99966085,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_expo_in() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::ExpoIn.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.001953125,
            0.00390625,
            0.0078125,
            0.015625,
            0.03125,
            0.0625,
            0.125,
            0.25,
            0.5,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_expo_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::ExpoOut.alpha(a))
            .collect();
        let expected = vec![
            0.0, 0.5, 0.75, 0.875, 0.9375, 0.96875, 0.984375, 0.9921875, 0.99609375, 0.9980469, 1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_expo_in_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::ExpoInOut.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.001953125,
            0.0078125,
            0.03125,
            0.125,
            0.5,
            0.875,
            0.96875,
            0.9921875,
            0.9980469,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_linear() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::Linear.alpha(a))
            .collect();
        let expected = vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_quad_in() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::QuadIn.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.010000001,
            0.040000003,
            0.09,
            0.16000001,
            0.25,
            0.36,
            0.48999998,
            0.64000005,
            0.80999994,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_quad_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::QuadOut.alpha(a))
            .collect();
        let expected = vec![
            0.0, 0.19000006, 0.35999995, 0.51, 0.64, 0.75, 0.84000003, 0.90999997, 0.96, 0.99, 1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_quad_in_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::QuadInOut.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.020000001,
            0.080000006,
            0.18,
            0.32000002,
            0.5,
            0.68000007,
            0.82,
            0.92,
            0.98,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_quart_in() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::QuartIn.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.000100000005,
            0.0016000001,
            0.008100001,
            0.025600001,
            0.0625,
            0.12960002,
            0.2401,
            0.40960002,
            0.6560999,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_quart_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::QuartOut.alpha(a))
            .collect();
        let expected = vec![
            0.0, 0.34390008, 0.5904, 0.75990003, 0.8704, 0.9375, 0.9744, 0.9919, 0.9984, 0.9999,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_quart_in_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::QuartInOut.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.00080000004,
            0.012800001,
            0.06480001,
            0.20480001,
            0.5,
            0.79520005,
            0.9352,
            0.9872,
            0.9992,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_quint_in() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::QuintIn.alpha(a))
            .collect();
        // Note: simple_easing library has a bug where quint_in returns the same values as
        // quart_in
        let expected = vec![
            0.0,
            0.000100000005,
            0.0016000001,
            0.008100001,
            0.025600001,
            0.0625,
            0.12960002,
            0.2401,
            0.40960002,
            0.6560999,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_quint_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::QuintOut.alpha(a))
            .collect();
        let expected = vec![
            0.0, 0.40951008, 0.67231995, 0.83193004, 0.92224, 0.96875, 0.98976, 0.99757, 0.99968,
            0.99999, 1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_quint_in_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::QuintInOut.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.00016000001,
            0.0051200003,
            0.038880005,
            0.16384001,
            0.5,
            0.83616006,
            0.96112,
            0.99488,
            0.99984,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_reverse() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::Reverse.alpha(a))
            .collect();
        let expected =
            vec![1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.39999998, 0.3, 0.19999999, 0.100000024, 0.0];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_sine_in() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::SineIn.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.012311637,
            0.04894346,
            0.10899347,
            0.190983,
            0.29289323,
            0.41221482,
            0.5460095,
            0.69098306,
            0.8435655,
            1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_sine_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::SineOut.alpha(a))
            .collect();
        let expected = vec![
            0.0, 0.15643448, 0.309017, 0.45399055, 0.58778524, 0.70710677, 0.80901706, 0.8910065,
            0.95105654, 0.98768836, 1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_sine_in_out() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::SineInOut.alpha(a))
            .collect();
        let expected = vec![
            -0.0, 0.02447173, 0.0954915, 0.20610741, 0.34549153, 0.5, 0.6545086, 0.7938926,
            0.90450853, 0.97552824, 1.0,
        ];
        assert_eq!(steps, expected);
    }
}
