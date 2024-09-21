use std::ops::Range;
use ratatui::layout::{Position, Rect};
use crate::fx::Direction;

pub struct SlidingWindowAlpha {
    alpha_fn: fn(Position, Range<f32>, f32) -> f32,
    gradient: Range<f32>,
    alpha_per_cell: f32,
}

#[bon::bon]
impl SlidingWindowAlpha {
    #[builder(finish_fn = build)]
    pub fn builder(
        direction: Direction,
        area: Rect,
        progress: f32,
        gradient_len: u16,
    ) -> Self {
        let alpha_fn = match direction {
            Direction::UpToDown    => slide_up,
            Direction::DownToUp    => slide_down,
            Direction::LeftToRight => slide_left,
            Direction::RightToLeft => slide_right,
        };

        let gradient = match direction {
            Direction::LeftToRight | Direction::RightToLeft =>
                gradient(progress, area.x, area.width, gradient_len),
            Direction::UpToDown | Direction::DownToUp =>
                gradient(progress, area.y, area.height, gradient_len),
        };

        let alpha_per_cell = 1.0 / (gradient.end - gradient.start);
        Self { alpha_fn, gradient, alpha_per_cell }
    }

    pub fn alpha(&self, position: Position) -> f32 {
        (self.alpha_fn)(position, self.gradient.clone(), self.alpha_per_cell)
    }
}

fn gradient(progress: f32, coordinate: u16, area_len: u16, gradient_len: u16) -> Range<f32> {
    let gradient_len = gradient_len as f32;
    let start = (coordinate as f32 - gradient_len) + ((area_len as f32 + gradient_len) * progress);
    let end = start + gradient_len;

    start..end
}

fn slide_down(
    position: Position,
    gradient: Range<f32>,
    alpha_per_cell: f32,
) -> f32 {
    match position.y as f32 {
        y if y < gradient.start => 0.0,
        y if y > gradient.end   => 1.0,
        y                       => alpha_per_cell * (y - gradient.start),
    }
}

fn slide_up(
    position: Position,
    gradient: Range<f32>,
    alpha_per_cell: f32,
) -> f32 {
    1.0 - slide_down(position, gradient, alpha_per_cell)
}

fn slide_right(
    position: Position,
    gradient: Range<f32>,
    alpha_per_cell: f32,
) -> f32 {
    match position.x as f32 {
        x if x < gradient.start => 0.0,
        x if x > gradient.end   => 1.0,
        x                       => alpha_per_cell * (x - gradient.start),
    }
}

fn slide_left(
    position: Position,
    gradient: Range<f32>,
    alpha_per_cell: f32,
) -> f32 {
    1.0 - slide_right(position, gradient, alpha_per_cell)
}