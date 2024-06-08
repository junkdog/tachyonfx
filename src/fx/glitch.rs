use std::fmt::Debug;
use std::ops::Range;
use std::time::Duration;

use derive_builder::Builder;
use rand::prelude::SmallRng;
use rand::Rng;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use crate::effect::{Effect, FilterMode, IntoEffect};
use crate::shader::Shader;


/// Type of glitch transformation to apply to a cell.
#[derive(Clone, Debug)]
enum GlitchType {
    ChangeCase,
    ChangeCharByValue(i8),
}

/// A glitch effect that can be applied to a cell.
#[derive(Builder)]
#[builder(pattern = "owned")]
#[derive(Clone, Debug)]
pub struct GlitchCell {
    cell_idx: usize,
    glitch_remaining_ms: u32,
    presleep_remaining_ms: u32,
    glitch: GlitchType,
}

impl GlitchCell {
    fn builder() -> GlitchCellBuilder { GlitchCellBuilder::default() }
}

/// applies a glitch effect to random parts of the screen.
#[derive(Builder, Clone, Debug)]
#[builder(pattern = "owned")]
pub struct Glitch {
    cell_glitch_ratio: f32,
    action_start_delay_ms: Range<u32>,
    action_ms: Range<u32>,
    rng: SmallRng,

    #[builder(setter(skip))]
    glitch_cells: Vec<GlitchCell>,
    #[builder(default)]
    area: Option<Rect>,
}

impl From<GlitchBuilder> for Effect {
    fn from(value: GlitchBuilder) -> Self {
        value.build().unwrap().into_effect()
    }
}

impl Glitch {
    pub fn builder() -> GlitchBuilder { GlitchBuilder::default() }

    fn ensure_population(
        &mut self,
        screen: &Rect,
    ) {
        let total_cells = (screen.width as f32 * screen.height as f32 * self.cell_glitch_ratio)
            .round() as u32;

        let current_population = self.glitch_cells.len() as u32;
        if current_population < total_cells {
            for _ in 0..(total_cells - current_population) {
                GlitchCell::builder()
                    .cell_idx(self.rng.gen_range(0..(screen.width * screen.height) as usize))
                    .glitch(self.glitch_type())
                    .glitch_remaining_ms(self.rng.gen_range(self.action_ms.clone()))
                    .presleep_remaining_ms(self.rng.gen_range(self.action_start_delay_ms.clone()))
                    .build()
                    .map(|glitch| self.glitch_cells.push(glitch))
                    .expect("Failed to build GlitchCell");
            }
        }
    }

    fn update_cell(cell: &mut GlitchCell, last_frame_ms: u32) {
        let f = |v: u32, sub: u32| (v.saturating_sub(sub), sub.saturating_sub(v));

        let (updated, remaining) = f(cell.presleep_remaining_ms, last_frame_ms);
        cell.presleep_remaining_ms = updated;
        cell.glitch_remaining_ms = cell.glitch_remaining_ms.saturating_sub(remaining);
    }

    fn is_running(cell: &GlitchCell) -> bool {
        cell.glitch_remaining_ms > 0
    }

    fn glitch_type(&mut self) -> GlitchType {
        let idx: u32 = self.rng.gen();
        match idx % 2 {
            0 => GlitchType::ChangeCase,
            1 => GlitchType::ChangeCharByValue(self.rng.gen_range(-10..10)),
            _ => unreachable!(),
        }
    }
}

impl Shader for Glitch {
    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect,
    ) -> Option<Duration> {
        // ensure glitch population meets the cell_glitch_ratio
        self.ensure_population(&area);

        // subtract durations
        let last_frame_ms = duration.as_millis() as u32;
        self.glitch_cells.iter_mut().for_each(|cell| Self::update_cell(cell, last_frame_ms));

        // remove invalid cells (e.g., from resizing)
        self.glitch_cells.retain(|cell| cell.cell_idx < buf.content.len());

        // apply glitches to buffer
        self.glitch_cells.iter().filter(|c| c.presleep_remaining_ms == 0).for_each(|cell| {
            let x = cell.cell_idx % area.width as usize;
            let y = cell.cell_idx / area.width as usize;
            let c  = buf.get_mut(area.x + x as u16, area.y + y as u16);

            match cell.glitch {
                GlitchType::ChangeCase if c.symbol().is_ascii() => {
                    let ch = c.symbol().chars().next().unwrap();
                    c.set_char(if ch.is_ascii_uppercase() {
                        ch.to_ascii_lowercase()
                    } else {
                        ch.to_ascii_uppercase()
                    });
                }
                GlitchType::ChangeCharByValue(v) if c.symbol().len() == 1 => {
                    if c.symbol().chars().next().is_some_and(|ch| ch == ' ') {
                        return;
                    }

                    c.set_char(if v > 0 {
                        c.symbol().as_bytes()[0]
                            .saturating_add(v as u8)
                            .clamp(32, 255) as char
                    } else {
                        c.symbol().as_bytes()[0]
                            .saturating_sub(v.abs() as u8)
                            .clamp(32, 255) as char
                    });
                }
                _ => {}
            }
        });

        // remove expired glitches
        self.glitch_cells.retain(Self::is_running);

        None
    }

    fn done(&self) -> bool {
        false
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.area
    }

    fn set_area(&mut self, area: Rect) {
        self.area = Some(area)
    }

    fn cell_selection(&mut self, strategy: FilterMode) {
        todo!()
    }
}
