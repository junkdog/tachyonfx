use crate::widget::EffectSpan;
use ratatui::prelude::Color;
use std::collections::BTreeSet;
use crate::simple_rng::{shuffle, SimpleRng};

#[derive(Clone)]
pub(crate) struct ColorRegistry {
    effect_to_color: Vec<(String, Color)>
}

impl ColorRegistry {
    pub(crate) fn from(root_span: &EffectSpan) -> Self {
        let effect_spans: Vec<&EffectSpan> = root_span.iter().collect();
        let effect_identifiers: BTreeSet<String> = effect_spans.iter()
            .map(|span| span.label.clone())
            .map(|label| id_of(&label).to_string())
            .collect();

        // hsl: 0..360, 59, 52
        let len = effect_identifiers.len();
        let mut colors: Vec<Color> = (0..len).map(|idx| 360.0 * idx as f64 / len as f64)
            .map(|hue| Color::from_hsl(hue, 52.0, 62.0))
            .collect();

        let mut lcg = SimpleRng::default();
        shuffle(&mut colors, &mut lcg);

        let effect_to_color = effect_identifiers.iter()
            .enumerate()
            .map(|(idx, effect)| (effect.clone(), colors[idx]))
            .collect();

        Self {
            effect_to_color
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
