use core::f32::consts::TAU;

use crate::math::{self, wave_cos, wave_sin};

const INV_TAU: f32 = 1.0 / TAU;

/// Fractional part of `v`, wrapped to `[0, 1)` for negative values.
#[inline(always)]
fn fract_positive(v: f32) -> f32 {
    let f = micromath::F32Ext::fract(v);
    if f < 0.0 {
        f + 1.0
    } else {
        f
    }
}

/// Waveform function selector.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WaveFn {
    Sin,
    Cos,
    /// Linear ramp up/down; produces angular/faceted patterns.
    Triangle,
    /// Linear ramp with sharp reset; produces directional sweep effects.
    Sawtooth,
}

impl WaveFn {
    pub fn name(self) -> &'static str {
        match self {
            WaveFn::Sin => "sin",
            WaveFn::Cos => "cos",
            WaveFn::Triangle => "triangle",
            WaveFn::Sawtooth => "sawtooth",
        }
    }

    fn eval(self, v: f32) -> f32 {
        let t = v * INV_TAU; // radians to normalized cycles
        match self {
            WaveFn::Sin => wave_sin(t),
            WaveFn::Cos => wave_cos(t),
            WaveFn::Triangle => {
                // arithmetic triangle wave: linear ramp via modular arithmetic
                let t = fract_positive(t) * 2.0; // [0, 2)
                if t < 1.0 {
                    2.0 * t - 1.0
                } else {
                    3.0 - 2.0 * t
                }
            },
            WaveFn::Sawtooth => fract_positive(t) * 2.0 - 1.0,
        }
    }
}

/// Whether a modulator affects the phase or amplitude of its parent oscillator.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ModTarget {
    /// Offsets the phase input of the oscillator (FM synthesis).
    Phase,
    /// Scales the output of the oscillator around 1.0 (AM synthesis).
    Amplitude,
}

/// Modulation source that affects either the phase or amplitude of its parent oscillator.
///
/// The signal is evaluated as `func(kx*x + ky*y + kt*t + phase) * intensity`,
/// where `x`/`y` are cell coordinates relative to the effect area and `t` is the
/// effect's animation progress (0.0 to 1.0).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Modulator {
    func: WaveFn,
    kx: f32,
    ky: f32,
    kt: f32,
    phase: f32,
    intensity: f32,
    target: ModTarget,
}

#[allow(dead_code)]
impl Modulator {
    fn new(func: WaveFn, kx: f32, ky: f32, kt: f32) -> Self {
        Self {
            func,
            kx,
            ky,
            kt,
            phase: 0.0,
            intensity: 1.0,
            target: ModTarget::Phase,
        }
    }

    /// Creates a sine modulator.
    ///
    /// - `kx`: spatial frequency along x (columns); higher = more oscillations per column
    /// - `ky`: spatial frequency along y (rows); higher = more oscillations per row
    /// - `kt`: temporal frequency; higher = faster animation over the effect's lifetime
    pub fn sin(kx: f32, ky: f32, kt: f32) -> Self {
        Self::new(WaveFn::Sin, kx, ky, kt)
    }

    /// Creates a cosine modulator. See [`Modulator::sin`] for parameter docs.
    pub fn cos(kx: f32, ky: f32, kt: f32) -> Self {
        Self::new(WaveFn::Cos, kx, ky, kt)
    }

    /// Creates a triangle-wave modulator. See [`Modulator::sin`] for parameter docs.
    pub fn triangle(kx: f32, ky: f32, kt: f32) -> Self {
        Self::new(WaveFn::Triangle, kx, ky, kt)
    }

    /// Creates a sawtooth-wave modulator. See [`Modulator::sin`] for parameter docs.
    pub fn sawtooth(kx: f32, ky: f32, kt: f32) -> Self {
        Self::new(WaveFn::Sawtooth, kx, ky, kt)
    }

    pub fn phase(self, phase: f32) -> Self {
        Self { phase, ..self }
    }

    pub fn intensity(self, intensity: f32) -> Self {
        Self { intensity, ..self }
    }

    pub fn on_phase(self) -> Self {
        Self { target: ModTarget::Phase, ..self }
    }

    pub fn on_amplitude(self) -> Self {
        Self { target: ModTarget::Amplitude, ..self }
    }

    pub fn func_name(&self) -> &'static str {
        self.func.name()
    }

    pub fn kx(&self) -> f32 {
        self.kx
    }
    pub fn ky(&self) -> f32 {
        self.ky
    }
    pub fn kt(&self) -> f32 {
        self.kt
    }
    pub fn phase_offset(&self) -> f32 {
        self.phase
    }
    pub fn intensity_value(&self) -> f32 {
        self.intensity
    }
    pub fn target(&self) -> ModTarget {
        self.target
    }

    fn signal(self, x: f32, y: f32, t: f32) -> f32 {
        self.intensity
            * self
                .func
                .eval(self.kx * x + self.ky * y + self.kt * t + self.phase)
    }
}

/// A single trig oscillator with optional modulation.
///
/// The signal is evaluated as `func(kx*x + ky*y + kt*t + phase)`,
/// where `x`/`y` are cell coordinates relative to the effect area and `t` is the
/// effect's animation progress (0.0 to 1.0).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Oscillator {
    func: WaveFn,
    kx: f32,
    ky: f32,
    kt: f32,
    phase: f32,
    modulator: Option<Modulator>,
}

#[allow(dead_code)]
impl Oscillator {
    fn new(func: WaveFn, kx: f32, ky: f32, kt: f32) -> Self {
        Self { func, kx, ky, kt, phase: 0.0, modulator: None }
    }

    /// Creates a sine oscillator.
    ///
    /// - `kx`: spatial frequency along x (columns); higher = more oscillations per column
    /// - `ky`: spatial frequency along y (rows); higher = more oscillations per row
    /// - `kt`: temporal frequency; higher = faster animation over the effect's lifetime
    pub fn sin(kx: f32, ky: f32, kt: f32) -> Self {
        Self::new(WaveFn::Sin, kx, ky, kt)
    }

    /// Creates a cosine oscillator. See [`Oscillator::sin`] for parameter docs.
    pub fn cos(kx: f32, ky: f32, kt: f32) -> Self {
        Self::new(WaveFn::Cos, kx, ky, kt)
    }

    /// Creates a triangle-wave oscillator. See [`Oscillator::sin`] for parameter docs.
    pub fn triangle(kx: f32, ky: f32, kt: f32) -> Self {
        Self::new(WaveFn::Triangle, kx, ky, kt)
    }

    /// Creates a sawtooth-wave oscillator. See [`Oscillator::sin`] for parameter docs.
    pub fn sawtooth(kx: f32, ky: f32, kt: f32) -> Self {
        Self::new(WaveFn::Sawtooth, kx, ky, kt)
    }

    pub fn phase(self, phase: f32) -> Self {
        Self { phase, ..self }
    }

    pub fn modulated_by(self, modulator: Modulator) -> Self {
        Self { modulator: Some(modulator), ..self }
    }

    pub fn func_name(&self) -> &'static str {
        self.func.name()
    }

    pub fn kx(&self) -> f32 {
        self.kx
    }
    pub fn ky(&self) -> f32 {
        self.ky
    }
    pub fn kt(&self) -> f32 {
        self.kt
    }
    pub fn phase_offset(&self) -> f32 {
        self.phase
    }
    pub fn modulator(&self) -> Option<&Modulator> {
        self.modulator.as_ref()
    }

    fn eval(self, x: f32, y: f32, t: f32) -> f32 {
        let (phase_mod, amp_mod) = self.modulator.map_or((0.0, 1.0), |m| {
            let s = m.signal(x, y, t);
            match m.target {
                ModTarget::Phase => (s, 1.0),
                ModTarget::Amplitude => (0.0, 1.0 + s),
            }
        });

        self.func
            .eval(self.kx * x + self.ky * y + self.kt * t + self.phase + phase_mod)
            * amp_mod
    }
}

/// How two oscillators are combined.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Combinator {
    Multiply,
    Average,
    Max,
}

/// Optional post-processing of the combined signal.
#[derive(Copy, Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum PostTransform {
    None,
    Power(i32),
    /// Mirror the negative half of the signal; visually doubles frequency.
    Abs,
}

/// One layer in the wave interference pattern.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct WaveLayer {
    a: Oscillator,
    b: Option<(Combinator, Oscillator)>,
    amplitude: f32,
    post_transform: PostTransform,
}

#[allow(dead_code)]
impl WaveLayer {
    pub fn new(a: Oscillator) -> Self {
        Self {
            a,
            b: None,
            amplitude: 1.0,
            post_transform: PostTransform::None,
        }
    }

    pub fn multiply(self, b: Oscillator) -> Self {
        Self { b: Some((Combinator::Multiply, b)), ..self }
    }

    pub fn average(self, b: Oscillator) -> Self {
        Self { b: Some((Combinator::Average, b)), ..self }
    }

    pub fn max(self, b: Oscillator) -> Self {
        Self { b: Some((Combinator::Max, b)), ..self }
    }

    pub fn amplitude(self, amplitude: f32) -> Self {
        Self { amplitude, ..self }
    }

    pub fn power(self, n: i32) -> Self {
        Self { post_transform: PostTransform::Power(n), ..self }
    }

    pub fn abs(self) -> Self {
        Self { post_transform: PostTransform::Abs, ..self }
    }

    pub fn amplitude_value(&self) -> f32 {
        self.amplitude
    }

    pub fn oscillator_a(&self) -> &Oscillator {
        &self.a
    }

    pub fn oscillator_b(&self) -> Option<(&Combinator, &Oscillator)> {
        self.b.as_ref().map(|(c, o)| (c, o))
    }

    pub fn post_transform(&self) -> PostTransform {
        self.post_transform
    }

    pub fn evaluate(&self, x: f32, y: f32, t: f32) -> f32 {
        let va = self.a.eval(x, y, t);

        let raw = match self.b {
            Some((Combinator::Multiply, ref osc)) => va * osc.eval(x, y, t),
            Some((Combinator::Average, ref osc)) => (va + osc.eval(x, y, t)) * 0.5,
            Some((Combinator::Max, ref osc)) => va.max(osc.eval(x, y, t)),
            None => va,
        };

        let transformed = match self.post_transform {
            PostTransform::None => raw,
            PostTransform::Power(n) => math::powi(raw, n),
            PostTransform::Abs => raw.abs(),
        };

        transformed * self.amplitude
    }
}

#[cfg(test)]
mod tests {
    use core::f32::consts::{FRAC_PI_2, PI};

    use super::*;

    const EPS: f32 = 0.05; // parabolic approximations aren't exact

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    // --- WaveFn ---

    #[test]
    fn sin_key_points() {
        assert!(approx(WaveFn::Sin.eval(0.0), 0.0));
        assert!(approx(WaveFn::Sin.eval(FRAC_PI_2), 1.0));
        assert!(approx(WaveFn::Sin.eval(PI), 0.0));
        assert!(approx(WaveFn::Sin.eval(3.0 * FRAC_PI_2), -1.0));
    }

    #[test]
    fn cos_key_points() {
        assert!(approx(WaveFn::Cos.eval(0.0), 1.0));
        assert!(approx(WaveFn::Cos.eval(FRAC_PI_2), 0.0));
        assert!(approx(WaveFn::Cos.eval(PI), -1.0));
    }

    #[test]
    fn triangle_key_points() {
        assert!(approx(WaveFn::Triangle.eval(0.0), -1.0));
        assert!(approx(WaveFn::Triangle.eval(FRAC_PI_2), 0.0));
        assert!(approx(WaveFn::Triangle.eval(PI), 1.0));
        assert!(approx(WaveFn::Triangle.eval(3.0 * FRAC_PI_2), 0.0));
        assert!(approx(WaveFn::Triangle.eval(TAU), -1.0));
    }

    #[test]
    fn sawtooth_key_points() {
        assert!(approx(WaveFn::Sawtooth.eval(0.0), -1.0));
        assert!(approx(WaveFn::Sawtooth.eval(PI), 0.0));
        // just before TAU wraps back to -1
        assert!(WaveFn::Sawtooth.eval(TAU - 0.01) > 0.9);
    }

    #[test]
    fn wavefn_negative_inputs_in_range() {
        let inputs = [-FRAC_PI_2, -PI, -3.0 * FRAC_PI_2, -TAU, -7.5];
        for wf in [WaveFn::Sin, WaveFn::Cos, WaveFn::Triangle, WaveFn::Sawtooth] {
            for &v in &inputs {
                let result = wf.eval(v);
                assert!(
                    (-1.0..=1.0).contains(&result),
                    "{:?}.eval({v}) = {result}, out of [-1, 1]",
                    wf
                );
            }
        }
    }

    #[test]
    fn wavefn_negative_matches_positive_period() {
        // f(-v) should equal f(TAU - v) for periodic functions
        for wf in [WaveFn::Sin, WaveFn::Cos, WaveFn::Triangle, WaveFn::Sawtooth] {
            for &v in &[0.5, 1.0, 2.0, FRAC_PI_2, PI] {
                let neg = wf.eval(-v);
                let wrapped = wf.eval(TAU - v);
                assert!(
                    approx(neg, wrapped),
                    "{:?}: eval({}) = {} but eval(TAU - {}) = {}",
                    wf,
                    -v,
                    neg,
                    v,
                    wrapped
                );
            }
        }
    }

    #[test]
    fn wavefn_periodic() {
        for wf in [WaveFn::Sin, WaveFn::Cos, WaveFn::Triangle, WaveFn::Sawtooth] {
            let v = 1.23;
            assert!(
                approx(wf.eval(v), wf.eval(v + TAU)),
                "{:?} not periodic",
                wf
            );
        }
    }

    // --- Modulator ---

    #[test]
    fn modulator_intensity_scales_signal() {
        let m = Modulator::sin(1.0, 0.0, 0.0).intensity(0.5);
        let full = Modulator::sin(1.0, 0.0, 0.0).signal(FRAC_PI_2, 0.0, 0.0);
        let half = m.signal(FRAC_PI_2, 0.0, 0.0);
        assert!(approx(half, full * 0.5));
    }

    #[test]
    fn modulator_target_defaults_to_phase() {
        let m = Modulator::sin(1.0, 0.0, 0.0);
        assert!(matches!(m.target, ModTarget::Phase));
    }

    // --- Oscillator ---

    #[test]
    fn oscillator_without_modulator() {
        let osc = Oscillator::sin(1.0, 0.0, 0.0);
        assert!(approx(osc.eval(FRAC_PI_2, 0.0, 0.0), 1.0));
    }

    #[test]
    fn oscillator_amplitude_modulation() {
        let carrier = Oscillator::cos(0.0, 0.0, 0.0); // cos(0) = 1.0 always
        let modulated = carrier.modulated_by(
            Modulator::cos(0.0, 0.0, 0.0)
                .on_amplitude()
                .intensity(0.5),
        );
        // amp_mod = 1.0 + 0.5 * cos(0) = 1.5, carrier = cos(0) = 1.0
        assert!(approx(modulated.eval(0.0, 0.0, 0.0), 1.5));
    }

    // --- WaveLayer ---

    #[test]
    fn layer_single_oscillator() {
        let layer = WaveLayer::new(Oscillator::sin(1.0, 0.0, 0.0));
        assert!(approx(layer.evaluate(FRAC_PI_2, 0.0, 0.0), 1.0));
    }

    #[test]
    fn layer_amplitude_scales_output() {
        let layer = WaveLayer::new(Oscillator::sin(1.0, 0.0, 0.0)).amplitude(0.5);
        assert!(approx(layer.evaluate(FRAC_PI_2, 0.0, 0.0), 0.5));
    }

    #[test]
    fn layer_multiply_combinator() {
        // sin(pi/2) * sin(pi/2) = 1.0 * 1.0
        let layer =
            WaveLayer::new(Oscillator::sin(1.0, 0.0, 0.0)).multiply(Oscillator::sin(1.0, 0.0, 0.0));
        assert!(approx(layer.evaluate(FRAC_PI_2, 0.0, 0.0), 1.0));

        // sin(0) * sin(pi/2) = 0.0
        assert!(approx(layer.evaluate(0.0, 0.0, 0.0), 0.0));
    }

    #[test]
    fn layer_average_combinator() {
        // (sin(pi/2) + sin(0)) / 2 = 0.5
        let layer = WaveLayer::new(Oscillator::sin(1.0, 0.0, 0.0))
            .average(Oscillator::sin(1.0, 0.0, 0.0).phase(FRAC_PI_2));
        // at x=0: (sin(0) + sin(pi/2)) / 2 = 0.5
        assert!(approx(layer.evaluate(0.0, 0.0, 0.0), 0.5));
    }

    #[test]
    fn layer_abs_post_transform() {
        let layer = WaveLayer::new(Oscillator::sin(1.0, 0.0, 0.0)).abs();
        // sin(3*pi/2) = -1.0, abs => 1.0
        assert!(approx(layer.evaluate(3.0 * FRAC_PI_2, 0.0, 0.0), 1.0));
    }

    #[test]
    fn layer_power_post_transform() {
        let layer = WaveLayer::new(Oscillator::sin(1.0, 0.0, 0.0)).power(2);
        // sin(pi/2)^2 = 1.0
        assert!(approx(layer.evaluate(FRAC_PI_2, 0.0, 0.0), 1.0));
        // sin(pi)^2 ~= 0.0
        assert!(approx(layer.evaluate(PI, 0.0, 0.0), 0.0));
    }
}
