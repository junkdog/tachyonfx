use crate::widget::EffectSpan;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use ratatui::prelude::Color;
use std::collections::BTreeSet;
use ratatui::layout::Rect;

#[derive(Clone)]
pub(crate) struct AreaRegistry {
    rects: Vec<Rect>
}

impl AreaRegistry {
    pub(crate) fn from(root_span: &EffectSpan) -> Self {
        let effect_spans: Vec<&EffectSpan> = root_span.iter().collect();
        let mut rects: Vec<Rect> = effect_spans.iter()
            .map(|span| span.area.clone())
            .filter(Option::is_some)
            .map(Option::unwrap)
            .collect();

        let pack = |a: &Rect| -> u64 {
            (a.x as u64) << 48
                | (a.y as u64) << 32
                | (a.width as u64) << 16
                | (a.height as u64)
        };

        rects.sort_by(|a, b| pack(a).cmp(&pack(b)));
        rects.dedup();

        Self {
            rects
        }
    }

    pub(crate) fn id_of(&self, area: Option<Rect>) -> String {
        match area {
            None => "   ".to_string(),
            Some(a) => {
                let id = self.rects.iter().position(|r| r == &a).unwrap() + 1;
                format!("r#{:}", id)
            }
        }
    }

    pub(crate) fn entries(&self) -> Vec<(String, String)> {
        self.rects.iter()
            .map(|area| (self.id_of(Some(*area)), area.to_string()))
            .collect()
    }
}
