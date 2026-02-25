use ratatui_core::{
    layout::Offset,
    style::{Color, Style},
};

use crate::{color_space::hsl_to_rgb, color_to_hsl, math, ColorSpace};

/// Easing functions for interpolation
mod easing {
    use core::f32::consts::{E, PI, TAU};

    use crate::math;

    pub(super) fn back_in(t: f32) -> f32 {
        let c1 = 1.70158;
        let c3 = c1 + 1.0;
        c3 * t * t * t - c1 * t * t
    }

    pub(super) fn back_out(t: f32) -> f32 {
        let c1 = 1.70158;
        let c3 = c1 + 1.0;
        1.0 + c3 * math::powi(t - 1.0, 3) + c1 * math::powi(t - 1.0, 2)
    }

    pub(super) fn back_in_out(t: f32) -> f32 {
        let c1 = 1.70158;
        let c2 = c1 * 1.525;

        if t < 0.5 {
            (math::powi(2.0 * t, 2) * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
        } else {
            (math::powi(2.0 * t - 2.0, 2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
        }
    }

    pub(super) fn bounce_out(t: f32) -> f32 {
        let n1 = 7.5625;
        let d1 = 2.75;

        if t < 1.0 / d1 {
            n1 * t * t
        } else if t < 2.0 / d1 {
            let t_adj = t - 1.5 / d1;
            n1 * t_adj * t_adj + 0.75
        } else if t < 2.5 / d1 {
            let t_adj = t - 2.25 / d1;
            n1 * t_adj * t_adj + 0.9375
        } else {
            let t_adj = t - 2.625 / d1;
            n1 * t_adj * t_adj + 0.984375
        }
    }

    pub(super) fn bounce_in(t: f32) -> f32 {
        1.0 - bounce_out(1.0 - t)
    }

    pub(super) fn bounce_in_out(t: f32) -> f32 {
        if t < 0.5 {
            (1.0 - bounce_out(1.0 - 2.0 * t)) / 2.0
        } else {
            (1.0 + bounce_out(2.0 * t - 1.0)) / 2.0
        }
    }

    pub(super) fn circ_in(t: f32) -> f32 {
        1.0 - math::sqrt(1.0 - t * t)
    }

    pub(super) fn circ_out(t: f32) -> f32 {
        math::sqrt(1.0 - (t - 1.0) * (t - 1.0))
    }

    pub(super) fn circ_in_out(t: f32) -> f32 {
        if t < 0.5 {
            (1.0 - circ_out(1.0 - 2.0 * t)) / 2.0
        } else {
            (circ_out(2.0 * t - 1.0) + 1.0) / 2.0
        }
    }

    pub(super) fn cubic_in(t: f32) -> f32 {
        t * t * t
    }

    pub(super) fn cubic_out(t: f32) -> f32 {
        1.0 - math::powi(1.0 - t, 3)
    }

    pub(super) fn cubic_in_out(t: f32) -> f32 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - math::powi(-2.0 * t + 2.0, 3) / 2.0
        }
    }

    pub(super) fn elastic_in(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else {
            let c4 = TAU / 3.0;
            -math::powf(2.0, 10.0 * (t - 1.0)) * math::sin((t - 1.0) * c4 - PI / 2.0)
        }
    }

    pub(super) fn elastic_out(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else {
            let c4 = TAU / 3.0;
            math::powf(2.0, -10.0 * t) * math::sin(t * c4 - PI / 2.0) + 1.0
        }
    }

    pub(super) fn elastic_in_out(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else if t < 0.5 {
            -(elastic_out(1.0 - 2.0 * t) - 1.0) / 2.0
        } else {
            (elastic_out(2.0 * t - 1.0) + 1.0) / 2.0
        }
    }

    pub(super) fn expo_in(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else {
            math::powf(2.0, 10.0 * (t - 1.0))
        }
    }

    pub(super) fn expo_out(t: f32) -> f32 {
        if t == 1.0 {
            1.0
        } else {
            1.0 - math::powf(2.0, -10.0 * t)
        }
    }

    pub(super) fn expo_in_out(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else if t < 0.5 {
            expo_in(2.0 * t) / 2.0
        } else {
            (2.0 - expo_in(2.0 * (1.0 - t))) / 2.0
        }
    }

    pub(super) fn quad_in(t: f32) -> f32 {
        t * t
    }

    pub(super) fn quad_out(t: f32) -> f32 {
        1.0 - (1.0 - t) * (1.0 - t)
    }

    pub(super) fn quad_in_out(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            1.0 - math::powi(-2.0 * t + 2.0, 2) / 2.0
        }
    }

    pub(super) fn quart_in(t: f32) -> f32 {
        t * t * t * t
    }

    pub(super) fn quart_out(t: f32) -> f32 {
        1.0 - math::powi(1.0 - t, 4)
    }

    pub(super) fn quart_in_out(t: f32) -> f32 {
        if t < 0.5 {
            8.0 * t * t * t * t
        } else {
            1.0 - math::powi(-2.0 * t + 2.0, 4) / 2.0
        }
    }

    pub(super) fn quint_in(t: f32) -> f32 {
        t * t * t * t * t
    }

    pub(super) fn quint_out(t: f32) -> f32 {
        1.0 - math::powi(1.0 - t, 5)
    }

    pub(super) fn quint_in_out(t: f32) -> f32 {
        if t < 0.5 {
            16.0 * t * t * t * t * t
        } else {
            1.0 - math::powi(-2.0 * t + 2.0, 5) / 2.0
        }
    }

    pub(super) fn reverse(t: f32) -> f32 {
        1.0 - t
    }

    pub(super) fn smooth_step(t: f32) -> f32 {
        t * t * (3.0 - 2.0 * t)
    }

    pub(super) fn spring(t: f32) -> f32 {
        let damping = 6.0;
        let frequency = 4.5 * TAU;
        1.0 - math::powf(E, -damping * t) * math::cos(frequency * t)
    }

    pub(super) fn sine_in(t: f32) -> f32 {
        1.0 - math::wave_sin(0.25 + t * 0.25)
    }

    pub(super) fn sine_out(t: f32) -> f32 {
        math::wave_sin(t * 0.25)
    }

    pub(super) fn sine_in_out(t: f32) -> f32 {
        -math::cos(t * PI) / 2.0 + 0.5
    }
}

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

    SmoothStep,
    Spring,

    SineIn,
    SineOut,
    SineInOut,
}

impl Interpolation {
    pub fn alpha(&self, a: f32) -> f32 {
        match self {
            Interpolation::BackIn => easing::back_in(a),
            Interpolation::BackOut => easing::back_out(a),
            Interpolation::BackInOut => easing::back_in_out(a),

            Interpolation::BounceIn => easing::bounce_in(a),
            Interpolation::BounceOut => easing::bounce_out(a),
            Interpolation::BounceInOut => easing::bounce_in_out(a),

            Interpolation::CircIn => easing::circ_in(a),
            Interpolation::CircOut => easing::circ_out(a),
            Interpolation::CircInOut => easing::circ_in_out(a),

            Interpolation::CubicIn => easing::cubic_in(a),
            Interpolation::CubicOut => easing::cubic_out(a),
            Interpolation::CubicInOut => easing::cubic_in_out(a),

            Interpolation::ElasticIn => easing::elastic_in(a),
            Interpolation::ElasticOut => easing::elastic_out(a),
            Interpolation::ElasticInOut => easing::elastic_in_out(a),

            Interpolation::ExpoIn => easing::expo_in(a),
            Interpolation::ExpoOut => easing::expo_out(a),
            Interpolation::ExpoInOut => easing::expo_in_out(a),

            Interpolation::Linear => a,

            Interpolation::QuadIn => easing::quad_in(a),
            Interpolation::QuadOut => easing::quad_out(a),
            Interpolation::QuadInOut => easing::quad_in_out(a),

            Interpolation::QuartIn => easing::quart_in(a),
            Interpolation::QuartOut => easing::quart_out(a),
            Interpolation::QuartInOut => easing::quart_in_out(a),

            Interpolation::QuintIn => easing::quint_in(a),
            Interpolation::QuintOut => easing::quint_out(a),
            Interpolation::QuintInOut => easing::quint_in_out(a),

            Interpolation::Reverse => easing::reverse(a),

            Interpolation::SmoothStep => easing::smooth_step(a),
            Interpolation::Spring => easing::spring(a),

            Interpolation::SineIn => easing::sine_in(a),
            Interpolation::SineOut => easing::sine_out(a),
            Interpolation::SineInOut => easing::sine_in_out(a),
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

            SmoothStep => SmoothStep,
            Spring => Spring,

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
        math::round((*self as f32).lerp(&(*target as f32), alpha)) as u16
    }
}

impl Interpolatable for i16 {
    fn lerp(&self, target: &i16, alpha: f32) -> i16 {
        math::round((*self as f32).lerp(&(*target as f32), alpha)) as i16
    }
}

impl Interpolatable for f32 {
    fn lerp(&self, target: &f32, alpha: f32) -> f32 {
        self + (target - self) * alpha
    }
}

impl Interpolatable for i32 {
    fn lerp(&self, target: &i32, alpha: f32) -> i32 {
        self + math::round((target - self) as f32 * alpha) as i32
    }
}

impl Interpolatable for Style {
    fn lerp(&self, target: &Style, alpha: f32) -> Style {
        let fg = self.fg.lerp(&target.fg, alpha);
        let bg = self.bg.lerp(&target.bg, alpha);

        let mut s = *self;
        if let Some(fg) = fg {
            s = s.fg(fg);
        }
        if let Some(bg) = bg {
            s = s.bg(bg);
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

#[cfg(all(test, feature = "std"))]
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
            0.004999995,
            0.02000004,
            0.045000017,
            0.08000004,
            0.125,
            0.18,
            0.245,
            0.39000005,
            0.55999994,
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
            8.131516e-20,
            0.44000006,
            0.60999995,
            0.755,
            0.82,
            0.875,
            0.92,
            0.955,
            0.97999996,
            0.995,
            1.0,
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
            0.0, 0.01000002, 0.03999999, 0.09, 0.19500002, 0.5, 0.80500007, 0.90999997, 0.96000004,
            0.99, 1.0,
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
            -0.0007031244,
            -0.00050347176,
            0.0010069435,
            0.005624995,
            0.017361118,
            0.04472223,
            0.10500001,
            0.23222226,
            0.49111104,
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
            0.0, 0.50888884, 0.7677778, 0.895, 0.9552778, 0.9826389, 0.994375, 0.99899304,
            1.0005034, 1.0007031, 1.0,
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
            -0.00025171041,
            0.0028125048,
            0.0223611,
            0.11611113,
            0.5,
            0.88388896,
            0.9776389,
            0.9971875,
            1.0002518,
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
            0.06250001,
            0.125,
            0.25000003,
            0.4999999,
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
            0.031250004,
            0.12500001,
            0.5,
            0.87500006,
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
        let expected = vec![
            0.0,
            1.0000001e-5,
            0.00032000002,
            0.0024300003,
            0.010240001,
            0.03125,
            0.07776001,
            0.16806999,
            0.32768002,
            0.5904899,
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
    fn test_smooth_step() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::SmoothStep.alpha(a))
            .collect();
        let expected = vec![
            0.0, 0.028, 0.104, 0.21600002, 0.35200003, 0.5, 0.648, 0.784, 0.896, 0.97199994, 1.0,
        ];
        assert_eq!(steps, expected);
    }

    #[test]
    fn test_spring() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::Spring.alpha(a))
            .collect();

        // verify boundary conditions
        assert!((steps[0] - 0.0).abs() < 0.01, "spring(0) should be ~0");
        assert!((steps[10] - 1.0).abs() < 0.01, "spring(1) should be ~1");

        // verify overshoot (spring should exceed 1.0 at some point)
        assert!(
            steps.iter().any(|&v| v > 1.0),
            "spring should overshoot past 1.0"
        );
    }

    #[test]
    fn test_sine_in() {
        let steps: Vec<f32> = generate_alpha_steps()
            .iter()
            .map(|&a| Interpolation::SineIn.alpha(a))
            .collect();
        let expected = vec![
            0.0,
            0.00999999,
            0.04000002,
            0.089999974,
            0.15999997,
            0.25,
            0.36,
            0.49000007,
            0.6399999,
            0.80999994,
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
            0.0, 0.19000004, 0.36000007, 0.50999993, 0.64, 0.75, 0.84000003, 0.91, 0.96, 0.99, 1.0,
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
            0.0, 0.02000001, 0.07999998, 0.18, 0.31999996, 0.5, 0.68000007, 0.81999993, 0.91999996,
            0.98, 1.0,
        ];
        assert_eq!(steps, expected);
    }
}
