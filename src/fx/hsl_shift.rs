use derive_builder::Builder;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::{CellIterator, ColorMapper, Effect, HslConvertable, Interpolatable, IntoEffect};
use crate::effect::CellFilter;
use crate::effect_timer::EffectTimer;
use crate::shader::Shader;

#[derive(Clone, Default, Builder)]
#[builder(pattern = "owned")]
pub struct HslShift {
    timer: EffectTimer,
    #[builder(default)]
    hsl_mod_fg: Option<[f32; 3]>,
    #[builder(default)]
    hsl_mod_bg: Option<[f32; 3]>,
    #[builder(default)]
    area: Option<Rect>,
    #[builder(default)]
    cell_filter: CellFilter,
}

impl HslShift {
    pub fn builder() -> HslShiftBuilder {
        HslShiftBuilder::default()
    }
}

impl Shader for HslShift {
    fn execute(&mut self, alpha: f32, _area: Rect, cell_iter: CellIterator) {
        let mut fg_mapper = ColorMapper::default();
        let mut bg_mapper = ColorMapper::default();

        let hsl_lerp = |c: Color, hsl: [f32; 3]| -> Color {
            let (h, s, l) = c.to_hsl();

            let (h, s, l) = (
                (h + 0.0.lerp(&hsl[0], alpha)) % 360.0,
                (s + 0.0.lerp(&hsl[1], alpha)).clamp(0.0, 100.0),
                (l + 0.0.lerp(&hsl[2], alpha)).clamp(0.0, 100.0),
            );

            HslConvertable::from_hsl(h, s, l)
        };

        for (_, cell) in cell_iter {
            if let Some(hsl_mod) = self.hsl_mod_fg {
                let fg = fg_mapper.map(cell.fg, alpha, |c| hsl_lerp(c, hsl_mod));
                cell.set_fg(fg);
            }
            if let Some(hsl_mod) = self.hsl_mod_bg {
                let bg = bg_mapper.map(cell.bg, alpha, |c| hsl_lerp(c, hsl_mod));
                cell.set_bg(bg);
            }

        }
    }

    fn done(&self) -> bool {
        self.timer.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> { self.area }
    fn set_area(&mut self, area: Rect) {
        self.area = Some(area);
    }

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        self.cell_filter = strategy;
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        Some(self.cell_filter.clone())
    }
}

impl From<HslShiftBuilder> for Effect {
    fn from(builder: HslShiftBuilder) -> Self {
        builder.build().unwrap().into_effect()
    }
}