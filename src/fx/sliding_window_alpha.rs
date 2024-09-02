use std::ops::Range;
use ratatui::layout::{Position, Rect};
use crate::fx::Direction;

pub struct SlidingWindowAlpha {
    alpha_fn: fn(Position, Range<f32>) -> f32,
    gradient: Range<f32>,
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
                horizontal_gradient(area, progress, gradient_len),
            Direction::UpToDown | Direction::DownToUp =>
                vertical_gradient(area, progress, gradient_len),
        };

        Self { alpha_fn, gradient }
    }

    pub fn alpha(&self, position: Position) -> f32 {
        (self.alpha_fn)(position, self.gradient.clone())
    }
}

fn horizontal_gradient(area: Rect, alpha: f32, gradient_len: u16) -> Range<f32> {
    let gradient_len = gradient_len as f32;
    let x_start = (area.x as f32 - gradient_len) + ((area.width as f32 + gradient_len) * alpha);
    let x_end = x_start + gradient_len;

    x_start..x_end
}

fn vertical_gradient(area: Rect, progress: f32, gradient_len: u16) -> Range<f32> {
    let gradient_len = gradient_len as f32;
    let y_start = (area.y as f32 - gradient_len) + ((area.height as f32 + gradient_len) * progress);
    let y_end = y_start + gradient_len;

    y_start..y_end
}

fn slide_up(
    position: Position,
    gradient: Range<f32>,
) -> f32 {
    match position.y as f32 {
        y if gradient.contains(&y) => 1.0 - (y - gradient.start) / (gradient.end - gradient.start),
        y if y < gradient.start    => 1.0,
        _                          => 0.0,
    }
}

fn slide_down(
    position: Position,
    gradient: Range<f32>,
) -> f32 {
    match position.y as f32 {
        y if gradient.contains(&y) => (y - gradient.start) / (gradient.end - gradient.start),
        y if y >= gradient.end     => 1.0,
        _                          => 0.0,
    }
}

fn slide_right(
    position: Position,
    gradient: Range<f32>,
) -> f32 {
    match position.x as f32 {
        x if gradient.contains(&x) => (x - gradient.start) / (gradient.end - gradient.start),
        x if x >= gradient.end     => 1.0,
        _                          => 0.0,
    }
}

fn slide_left(
    position: Position,
    gradient: Range<f32>,
) -> f32 {
    match position.x as f32 {
        x if gradient.contains(&x) => 1.0 - (x - gradient.start) / (gradient.end - gradient.start),
        x if x < gradient.start    => 1.0,
        _                          => 0.0,
    }
}
