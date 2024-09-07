use bon::builder;
use std::fmt::Debug;
use std::ops::Range;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use crate::{CellFilter, CellIterator, Duration, EffectTimer};
use crate::simple_rng::{RangeSampler, SimpleRng};
use crate::shader::Shader;


/// Type of glitch transformation to apply to a cell.
#[derive(Clone, Debug)]
pub enum GlitchType { // fixme: make non-public again
    ChangeCase,
    ChangeCharByValue(i8),
}

/// A glitch effect that can be applied to a cell.
#[derive(Clone, Debug)]
#[builder]
pub struct GlitchCell {
    cell_idx: usize,
    glitch_remaining_ms: u32,
    presleep_remaining_ms: u32,
    glitch: GlitchType,
}

/// applies a glitch effect to random parts of the screen.
#[derive(Clone)]
#[builder]
pub struct Glitch {
    cell_glitch_ratio: f32,
    action_start_delay_ms: Range<u32>,
    action_ms: Range<u32>,
    #[builder(default)]
    rng: SimpleRng,
    #[builder(default)]
    selection: CellFilter,

    #[builder(skip)]
    glitch_cells: Vec<GlitchCell>,
    area: Option<Rect>,
}

impl Glitch {
    fn ensure_population(
        &mut self,
        screen: &Rect,
    ) {
        let total_cells = (screen.width as f32 * screen.height as f32 * self.cell_glitch_ratio)
            .round() as u32;

        let current_population = self.glitch_cells.len() as u32;
        if current_population < total_cells {
            for _ in 0..(total_cells - current_population) {
                let cell = GlitchCell::builder()
                    .cell_idx(self.rng.gen_range(0..(screen.width * screen.height) as usize))
                    .glitch(self.glitch_type())
                    .glitch_remaining_ms(self.rng.gen_range(self.action_ms.clone()))
                    .presleep_remaining_ms(self.rng.gen_range(self.action_start_delay_ms.clone()))
                    .build();
                self.glitch_cells.push(cell);
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
            1 => GlitchType::ChangeCharByValue(-10 + self.rng.gen_range(0..20) as i8),
            _ => unreachable!(),
        }
    }
}

impl Shader for Glitch {
    fn name(&self) -> &'static str {
        "glitch"
    }

    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect,
    ) -> Option<Duration> {
        // ensure glitch population meets the cell_glitch_ratio
        self.ensure_population(&area);

        // subtract durations
        let last_frame_ms = duration.as_millis();
        self.glitch_cells.iter_mut().for_each(|cell| Self::update_cell(cell, last_frame_ms));

        // remove invalid cells (e.g., from resizing)
        self.glitch_cells.retain(|cell| cell.cell_idx < buf.content.len());

        let selector = self.selection.selector(area);

        // apply glitches to buffer
        self.glitch_cells.iter().filter(|c| c.presleep_remaining_ms == 0).for_each(|cell| {
            let x = cell.cell_idx % area.width as usize;
            let y = cell.cell_idx / area.width as usize;
            let pos = Position::new(area.x + x as u16, area.y + y as u16);
            let c  = buf.cell_mut(Position::new(area.x + x as u16, area.y + y as u16)).unwrap();

            if !selector.is_valid(pos, c) {
                return;
            }

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
                            .saturating_sub(v.unsigned_abs())
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

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {}

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

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        self.selection = strategy;
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> { None }

    fn cell_selection(&self) -> Option<CellFilter> {
        Some(self.selection.clone())
    }

    fn reset(&mut self) {
        self.glitch_cells.clear();
    }
}
