use crate::CellFilter;
use crate::widget::EffectSpan;

#[derive(Clone)]
pub(crate) struct CellFilterRegistry {
    filters: Vec<String>,
}

impl CellFilterRegistry {
    pub(crate) fn from(root_span: &EffectSpan) -> Self {
        let mut this = Self {
            filters: vec![CellFilter::All.to_string()],
        };

        root_span.iter()
            .map(|span| &span.cell_filter)
            .for_each(|filter| this.register(filter));

        this
    }

    pub(crate) fn id_of(&self, filter: &CellFilter) -> String {
        let sought = filter.to_string();
        let filter_idx = self.filters.iter()
            .position(|f| f == &sought)
            .unwrap();

        format_id(filter_idx)
    }

    fn register(&mut self, filter: &CellFilter) {
        let s = filter.to_string();
        if !self.filters.contains(&s) {
            self.filters.push(s);
        }
    }


    pub(crate) fn entries(&self) -> Vec<(String, String)> {
        self.filters.iter().enumerate()
            .map(|(idx, filter)| (format_id(idx), filter.clone()))
            .collect()
    }
}

fn format_id(idx: usize) -> String {
    if idx == 0 {
        "    *".to_string()
    } else {
        format!("cf-{:02}", idx)
    }
}