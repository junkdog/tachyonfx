use ratatui::layout::Rect;
use crate::{RangeSampler, SimpleRng};

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Direction {
    LeftToRight,
    RightToLeft,
    UpToDown,
    DownToUp,
}

impl Direction {
    pub(crate) fn flipped(&self) -> Self {
        match self {
            Self::LeftToRight => Self::RightToLeft,
            Self::RightToLeft => Self::LeftToRight,
            Self::UpToDown    => Self::DownToUp,
            Self::DownToUp    => Self::UpToDown,
        }
    }

    pub(crate) fn flips_timer(&self) -> bool {
        self == &Direction::RightToLeft || self == &Direction::DownToUp
    }
}

/// Generates random variances for directional effects.
pub(crate) struct DirectionalVariance {
    rng: SimpleRng,
    direction: Direction,
    max: i16,
}

impl DirectionalVariance {
    /// Creates a new `DirectionalVariance` instance.
    ///
    /// This method initializes a `DirectionalVariance` with a seed based on the given area's dimensions,
    /// the specified direction for the sliding effect, and the maximum variance allowed.
    ///
    /// # Arguments
    ///
    /// * `area` - The `Rect` representing the area of the effect. Used to seed the RNG.
    /// * `direction` - The `Direction` of the sliding effect.
    /// * `max` - The maximum variance that can be generated.
    ///
    /// # Returns
    ///
    /// A new `DirectionalVariance` instance.
    pub(super) fn from(
        area: Rect,
        direction: Direction,
        max: u16
    ) -> Self {
        Self {
            rng: SimpleRng::new((area.width as u32) << 16 | area.height as u32),
            direction,
            max: max as i16,
        }
    }

    /// Generates the next variance value.
    ///
    /// This method produces a tuple representing an (x, y) offset based on the
    /// configured direction and maximum variance. The generated variance is always
    /// within the range [-max, max] for the relevant axis, and 0 for the other axis.
    ///
    /// # Returns
    ///
    /// A tuple `(i16, i16)` representing the (x, y) variance to be applied.
    /// If the maximum variance is set to 0, it always returns (0, 0).
    pub(crate) fn next(&mut self) -> (i16, i16) {
        if self.max == 0 {
            return (0, 0);
        }

        let variance = self.rng.gen_range(0..self.max);
        match self.direction {
            Direction::LeftToRight => (variance, 0),
            Direction::RightToLeft => (-variance, 0),
            Direction::UpToDown    => (0, variance),
            Direction::DownToUp    => (0, -variance),
        }
    }
}