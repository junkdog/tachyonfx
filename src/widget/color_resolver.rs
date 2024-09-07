use crate::widget::EffectSpan;
use crate::{RangeSampler, SimpleRng};
use bon::builder;
use ratatui::prelude::Color;
use std::collections::BTreeSet;
use std::ops::Range;

#[derive(Clone)]
pub(crate) struct ColorResolver {
    effect_to_color: Vec<(String, Color)>,
}

#[builder]
pub(crate) fn color_registry(
    root_span: &EffectSpan,
    hue: Range<f64>,
    saturation: f64,
    lightness: f64,
) -> ColorResolver {
    ColorResolver::from(root_span, hue, saturation, lightness)
}

impl ColorResolver {
    fn from(
        root_span: &EffectSpan,
        hue: Range<f64>,
        saturation: f64,
        lightness: f64,
    ) -> Self {
        assert!(hue.start >= 0.0 && hue.end <= 360.0, "hue range must be between 0 and 360");
        assert!((0.0..=100.0).contains(&saturation), "saturation must be between 0 and 100");
        assert!((0.0..=100.0).contains(&lightness), "lightness must be between 0 and 100");

        let effect_spans: Vec<&EffectSpan> = root_span.iter().collect();
        let effect_identifiers: BTreeSet<String> = effect_spans.iter()
            .map(|span| span.label.clone())
            .map(|label| id_of(&label).to_string())
            .collect();

        let hue_range = hue.end - hue.start;

        let len = effect_identifiers.len();
        let mut colors: Vec<Color> = (0..len)
            .map(|idx| hue.start + hue_range * idx as f64 / len as f64)
            .map(|hue| Color::from_hsl(hue, saturation as _, lightness as _))
            .collect();

        let mut lcg = SimpleRng::default();
        shuffle(&mut colors, &mut lcg);

        let effect_to_color = effect_identifiers.iter()
            .enumerate()
            .map(|(idx, effect)| (effect.clone(), colors[idx]))
            .collect();

        Self {
            effect_to_color,
        }
    }

    pub(crate) fn color_of(&self, effect: &str) -> Color {
        let id = id_of(effect);

        self.effect_to_color.iter()
            .find(|(label, _)| label == id)
            .map(|(_, color)| *color)
            .unwrap_or_else(|| panic!("effect not found: {id}"))
    }
}

fn id_of(effect: &str) -> &str {
    effect
        .strip_suffix("_out")
        .or(effect.strip_suffix("_in"))
        .or(effect.strip_suffix("_to"))
        .or(effect.strip_suffix("_from"))
        .unwrap_or(effect)
}

fn shuffle<T>(vec: &mut [T], rng: &mut SimpleRng) {
    let len = vec.len();
    for i in 0..len {
        let j = rng.gen_range(i..len);
        vec.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shuffle() {
        let mut lcg = SimpleRng::new(12345);
        let mut vec = vec![1, 2, 3, 4, 5];
        let original = vec.clone();

        shuffle(&mut vec, &mut lcg);

        assert_ne!(vec, original);
        assert_eq!(vec.len(), original.len());
        assert_eq!(vec.iter().sum::<i32>(), original.iter().sum::<i32>());
    }
}