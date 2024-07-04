use std::ops::Range;
use ratatui::layout::{Position, Rect};
use crate::fx::Direction;

pub(crate) fn horizontal_gradient(area: Rect, alpha: f32, gradient_len: u16) -> Range<f32> {
    let gradient_len = gradient_len as f32;
    let x_start = (area.x as f32 - gradient_len) + ((area.width as f32 + gradient_len) * alpha);
    let x_end = x_start + gradient_len;

    x_start..x_end
}

pub(crate) fn vertical_gradient(area: Rect, progress: f32, gradient_len: u16) -> Range<f32> {
    let gradient_len = gradient_len as f32;
    let y_start = (area.y as f32 - gradient_len) + ((area.height as f32 + gradient_len) * progress);
    let y_end = y_start + gradient_len;

    y_start..y_end
}

pub(crate) fn window_alpha_fn(
    direction: Direction,
    gradient: Range<f32>,
) -> Box::<dyn Fn(Position) -> f32> {
    let gradient_len = gradient.end - gradient.start;
    match direction {
        Direction::LeftToRight => Box::new(move |p: Position| -> f32 {
            match p.x as f32 {
                x if gradient.contains(&x) => 1.0 - (x - gradient.start) / gradient_len,
                x if x < gradient.start    => 1.0,
                _                          => 0.0,
            }
        }),
        Direction::RightToLeft => Box::new(move |p: Position| -> f32 {
            match p.x as f32 {
                x if gradient.contains(&x) => (x - gradient.start) / gradient_len,
                x if x >= gradient.end     => 1.0,
                _                          => 0.0,
            }
        }),
        Direction::UpToDown => Box::new(move |p: Position| -> f32 {
            match p.y as f32 {
                y if gradient.contains(&y) => 1.0 - (y - gradient.start) / gradient_len,
                y if y < gradient.start    => 1.0,
                _                          => 0.0,
            }
        }),
        Direction::DownToUp => Box::new(move |p: Position| -> f32 {
            match p.y as f32 {
                y if gradient.contains(&y) => (y - gradient.start) / gradient_len,
                y if y >= gradient.start   => 1.0,
                _                          => 0.0,
            }
        }),
    }
}