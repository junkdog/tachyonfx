use crate::widget::EffectSpan;
use crate::{shuffle, SimpleRng};
use bon::builder;
use ratatui::prelude::Color;
use std::collections::BTreeSet;
use std::ops::Range;

#[derive(Clone)]
pub(crate) struct ColorRegistry {
    effect_to_color: Vec<(String, Color)>,
}

#[builder]
pub(crate) fn color_registry(
    root_span: &EffectSpan,
    hue: Range<f64>,
    saturation: f64,
    lightness: f64,
) -> ColorRegistry {
    ColorRegistry::from(root_span, hue, saturation, lightness)
}

impl ColorRegistry {
    fn from(
        root_span: &EffectSpan,
        hue: Range<f64>,
        saturation: f64,
        lightness: f64,
    ) -> Self {
        assert!(hue.start >= 0.0 && hue.end <= 360.0, "hue range must be between 0 and 360");
        assert!(saturation >= 0.0 && saturation <= 100.0, "saturation must be between 0 and 100");
        assert!(lightness >= 0.0 && lightness <= 100.0, "lightness must be between 0 and 100");

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
            .expect(format!("effect not found: {id}").as_str())
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
