use ratatui::layout::{Position, Rect};

use crate::Motion;

#[derive(Clone, Copy, Debug)]
pub struct SlidingWindowAlpha {
    alpha_fn: fn(Position, Gradient, f32) -> f32,
    gradient: Gradient,
    alpha_per_cell: f32,
}

#[derive(Clone, Copy, Debug)]
struct Gradient {
    start: f32,
    end: f32,
}

#[bon::bon]
impl SlidingWindowAlpha {
    #[builder(finish_fn = build)]
    pub fn builder(direction: Motion, area: Rect, progress: f32, gradient_len: u16) -> Self {
        let alpha_fn = match direction {
            Motion::UpToDown => move_up_to_down,
            Motion::DownToUp => move_down_to_up,
            Motion::LeftToRight => move_left_to_right,
            Motion::RightToLeft => move_right_to_left,
        };

        let gradient = match direction {
            Motion::LeftToRight | Motion::RightToLeft => {
                gradient(progress, area.x, area.width, gradient_len)
            },
            Motion::UpToDown | Motion::DownToUp => {
                gradient(progress, area.y, area.height, gradient_len)
            },
        };

        let alpha_per_cell = 1.0 / (gradient.end - gradient.start);
        Self { alpha_fn, gradient, alpha_per_cell }
    }

    pub fn alpha(&self, position: Position) -> f32 {
        (self.alpha_fn)(position, self.gradient, self.alpha_per_cell)
    }
}

fn gradient(progress: f32, coordinate: u16, area_len: u16, gradient_len: u16) -> Gradient {
    let gradient_len = gradient_len as f32;
    let start = (coordinate as f32 - gradient_len) + ((area_len as f32 + gradient_len) * progress);
    let end = start + gradient_len;

    Gradient { start, end }
}

fn move_down_to_up(position: Position, gradient: Gradient, alpha_per_cell: f32) -> f32 {
    match position.y as f32 {
        y if y < gradient.start => 0.0,
        y if y > gradient.end => 1.0,
        y => alpha_per_cell * (y - gradient.start),
    }
}

fn move_up_to_down(position: Position, gradient: Gradient, alpha_per_cell: f32) -> f32 {
    1.0 - move_down_to_up(position, gradient, alpha_per_cell)
}

fn move_right_to_left(position: Position, gradient: Gradient, alpha_per_cell: f32) -> f32 {
    match position.x as f32 {
        x if x < gradient.start => 0.0,
        x if x > gradient.end => 1.0,
        x => alpha_per_cell * (x - gradient.start),
    }
}

fn move_left_to_right(position: Position, gradient: Gradient, alpha_per_cell: f32) -> f32 {
    1.0 - move_right_to_left(position, gradient, alpha_per_cell)
}
