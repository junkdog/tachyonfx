use derive_builder::Builder;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::{CellFilter, CellIterator, Effect, EffectTimer, IntoEffect, Shader};
use crate::fx::Direction;
use crate::fx::moving_window::{horizontal_gradient, vertical_gradient, window_alpha_fn};

/// A shader that applies a directional sliding effect to terminal cells.
#[derive(Builder, Clone)]
#[builder(pattern = "owned")]
pub struct SlideCell {
    /// The color behind the sliding cell.
    color_behind_cell: Color,
    /// The direction of the sliding effect.
    direction: Direction,
    /// The length of the gradient used for the sliding effect.
    gradient_length: u16,
    /// The timer controlling the duration and progress of the effect.
    timer: EffectTimer,
    /// The area within which the effect is applied.
    #[builder(default)]
    area: Option<Rect>,
    /// The cell selection strategy used to filter cells.
    #[builder(default)]
    cell_filter: CellFilter,
}

impl SlideCell {
    pub fn builder() -> SlideCellBuilder {
        SlideCellBuilder::default()
    }

    fn slided_cell(&self, alpha: f32) -> char {
        let alpha = alpha.clamp(0.0, 1.0);
        let char_idx = (LAST_IDX as f32 * alpha).round() as usize;

        match self.direction {
            Direction::LeftToRight | Direction::RightToLeft => SHRINK_H[char_idx],
            Direction::UpToDown    | Direction::DownToUp    => SHRINK_V[char_idx],
        }
    }
}

impl Shader for SlideCell {
    fn name(&self) -> &'static str {
        if self.timer.is_reversed() { "slide_out" } else { "slide_in" }
    }

    fn execute(&mut self, alpha: f32, area: Rect, cells: CellIterator) {
        let direction = self.direction;

        let gradient = match direction {
            Direction::LeftToRight | Direction::RightToLeft =>
                horizontal_gradient(area, alpha, self.gradient_length),
            Direction::UpToDown | Direction::DownToUp =>
                vertical_gradient(area, alpha, self.gradient_length),
        };
        let window_alpha = window_alpha_fn(direction, gradient);

        cells.for_each(|(pos, cell)| {
            match window_alpha(pos) {
                0.0 => {},
                1.0 => {
                    cell.set_char(' ');
                    cell.fg = cell.bg;
                    cell.bg = self.color_behind_cell;
                }
                a => {
                    cell.set_char(self.slided_cell(a));
                    cell.fg = cell.bg;
                    cell.bg = self.color_behind_cell;
                }
            }
        });
    }

    fn done(&self) -> bool { self.timer.done() }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.area
    }

    fn set_area(&mut self, area: Rect) {
        self.area = Some(area);
    }

    fn set_cell_selection(&mut self, strategy: CellFilter) {
        self.cell_filter = strategy;
    }

    fn timer_mut(&mut self) -> Option<&mut EffectTimer> {
        Some(&mut self.timer)
    }

    fn timer(&self) -> Option<EffectTimer> {
        Some(self.timer.clone())
    }

    fn cell_selection(&self) -> Option<CellFilter> {
        Some(self.cell_filter.clone())
    }
}

impl From<SlideCellBuilder> for Effect {
    fn from(value: SlideCellBuilder) -> Self {
        value.build().unwrap().into_effect()
    }
}

const SHRINK_V: &'static [char; 9] = &['█', '▇', '▆', '▅', '▄', '▃', '▂', '▁', ' '];
const SHRINK_H: &'static [char; 9] = &['█', '▉', '▊', '▋', '▌', '▍', '▎', '▏', ' '];
const LAST_IDX: usize = SHRINK_H.len() - 1;
