use std::time::Duration;

use ratatui::style::Color;

pub use glitch::Glitch;
use ping_pong::PingPong;
pub use shader_fn::*;
pub use sweep_in::Direction;

use crate::CellIterator;
use crate::effect::{Effect, IntoEffect};
use crate::effect_timer::EffectTimer;
use crate::fx::ansi256::Ansi256;
use crate::fx::consume_tick::ConsumeTick;
use crate::fx::containers::{ParallelEffect, SequentialEffect};
use crate::fx::dissolve::Dissolve;
use crate::fx::fade::FadeColors;
use crate::fx::hsl_shift::HslShift;
use crate::fx::never_complete::NeverComplete;
use crate::fx::repeat::Repeat;
use crate::fx::resize::ResizeArea;
use crate::fx::sleep::Sleep;
use crate::fx::sweep_in::SweepIn;
use crate::fx::temporary::{IntoTemporaryEffect, TemporaryEffect};

mod ansi256;
mod consume_tick;
mod containers;
mod dissolve;
mod fade;
mod glitch;
mod never_complete;
mod ping_pong;
mod repeat;
mod resize;
mod sleep;
mod sweep_in;
mod temporary;
mod translate;
mod hsl_shift;
mod shader_fn;

/// Creates a custom effect using a user-defined function.
///
/// This function allows you to define custom effects by providing a closure that will be called
/// with the current state, `ShaderFnContext`, and a cell iterator. You can use this closure
/// to apply custom transformations or animations to the terminal cells. The function also takes
/// an initial state that can be used to maintain state across invocations.
///
/// # Arguments
/// * `state` - An initial state that will be passed to the closure on each invocation.
/// * `timer` - An `EffectTimer` instance to control the duration and timing of the effect.
/// * `f` - A closure that defines the custom effect. The closure takes three parameters:
///   * `state`: A mutable reference to the state provided during the creation of the effect.
///   * `context`: A `ShaderFnContext` instance containing timing and area information.
///   * `cell_iter`: An iterator over the terminal cells.
///
/// # Returns
/// * An `Effect` instance that can be used with other effects or applied directly to terminal cells.
///
/// # Examples
///
/// ```no_run
/// use ratatui::style::Color;
/// use tachyonfx::*;
///
/// let timer = EffectTimer::from_ms(1000, Interpolation::CubicInOut);
/// let initial_state = (); // no state to keep track of
///
/// fx::effect_fn(timer, |_state, context, cell_iter| {
///    let mut fg_mapper = ColorMapper::default();
///    let alpha = context.alpha();
///
///    for (_pos, cell) in cell_iter {
///        // context.timer.progress() is already interpolated, so we can linearly lerp to the target color
///        let color = fg_mapper.map(cell.fg, alpha, |c| c.lerp(&Color::Indexed(35), alpha));
///        cell.set_fg(color);
///    }
/// }).with_cell_selection(CellFilter::FgColor(Color::DarkGray));
/// ```
///
/// In this example, the custom effect function interpolates the foreground color of each
/// cell to a new color over the specified duration. The effect is only applied to cells with
/// a foreground color of `Color::DarkGray`.
pub fn effect_fn<F, S, T>(state: S, timer: T, f: F) -> Effect
where
    S: Clone + 'static,
    T: Into<EffectTimer>,
    F: FnMut(&mut S, ShaderFnContext, CellIterator) + 'static,
{
    ShaderFn::new(state, f, timer).into_effect()
}

/// changes the hue, saturation, and lightness of the foreground and background colors.
pub fn hsl_shift<T: Into<EffectTimer>>(
    hsl_fg_change: Option<[f32; 3]>,
    hsl_bg_change: Option<[f32; 3]>,
    lifetime: T,
) -> Effect {
    HslShift::builder()
        .hsl_mod_fg(hsl_fg_change)
        .hsl_mod_bg(hsl_bg_change)
        .lifetime(lifetime.into())
        .into()
}

/// Shifts the foreground color by the specified hue, saturation, and lightness
/// over the specified duration.
pub fn hsl_shift_fg<T: Into<EffectTimer>>(
    hsl_fg_change: [f32; 3],
    lifetime: T,
) -> Effect {
    hsl_shift(Some(hsl_fg_change), None, lifetime)
}

/// Returns an effect that downsamples to 256 color mode.
pub fn term256_colors() -> Effect {
    Ansi256::default().into_effect()
}

/// Repeat the effect indefinitely or for a specified number of times or duration.
pub fn repeat(effect: Effect, mode: repeat::RepeatMode) -> Effect {
    Repeat::new(effect, mode).into_effect()
}

/// plays the effect forwards and then backwards.
pub fn ping_pong(effect: Effect) -> Effect {
    PingPong::new(effect).into_effect()
}

/// Repeat the effect indefinitely.
pub fn repeating(effect: Effect) -> Effect {
    repeat(effect, repeat::RepeatMode::Forever)
}

/// Sweeps out to the specified color.
pub fn sweep_out<T: Into<EffectTimer>, C: Into<Color>>(
    direction: Direction,
    gradient_length: u16,
    faded_color: C,
    lifetime: T,
) -> Effect {
    let flipped = match direction {
        Direction::LeftToRight => Direction::RightToLeft,
        Direction::RightToLeft => Direction::LeftToRight,
        Direction::UpToDown => Direction::DownToUp,
        Direction::DownToUp => Direction::UpToDown,
    };
    sweep_in(flipped, gradient_length, faded_color, lifetime).reversed()
}

/// Sweeps in a from the specified color.
pub fn sweep_in<T: Into<EffectTimer>, C: Into<Color>>(
    direction: Direction,
    gradient_length: u16,
    faded_color: C,
    lifetime: T,
) -> Effect {
    SweepIn::new(direction, gradient_length, faded_color.into(), lifetime.into())
        .into_effect()
}

pub fn translate<T: Into<EffectTimer>>(
    fx: Option<Effect>,
    translate_by: (i16, i16),
    lifetime: T,
) -> Effect {
    translate::Translate::new(fx, translate_by, lifetime.into()).into_effect()
}

/// An effect that resizes the area of the wrapped effect to the specified
/// dimensions. The effect will be rendered within the resized area.
pub fn resize_area<T: Into<EffectTimer>>(
    fx: Option<Effect>,
    initial_w: u16,
    initial_h: u16,
    lifetime: T,
) -> Effect {
    ResizeArea::new(fx, initial_w, initial_h, lifetime.into()).into_effect()
}

/// Runs the effects in sequence, one after the other. Reports completion
/// once the last effect has completed.
pub fn sequence(effects: Vec<Effect>) -> Effect {
    SequentialEffect::new(effects).into_effect()
}

/// Runs the effects in parallel, all at the same time. Reports completion
/// once all effects have completed.
pub fn parallel(effects: Vec<Effect>) -> Effect {
    ParallelEffect::new(effects).into_effect()
}

/// Dissolves the current text into the new text over the specified duration. The
/// `cycle_len` parameter specifies the number of cell states are tracked before
/// it cycles and repeats.
pub fn dissolve<T: Into<EffectTimer>>(cycle_len: usize, lifetime: T) -> Effect {
    Dissolve::new(lifetime.into(), cycle_len)
        .into_effect()
}

/// The reverse of [dissolve()].
pub fn coalesce<T: Into<EffectTimer>>(cycle_len: usize, lifetime: T) -> Effect {
    let lifetime = lifetime.into().reversed();
    Dissolve::new(lifetime, cycle_len)
        .into_effect()
}


/// Fades the foreground color to the specified color over the specified duration.
pub fn fade_to_fg<T: Into<EffectTimer>, C: Into<Color>>(
    fg: C,
    lifetime: T,
) -> Effect {
    fade(Some(fg), None, lifetime.into(), false)
}

/// Fades the foreground color from the specified color over the specified duration.
pub fn fade_from_fg<T: Into<EffectTimer>, C: Into<Color>>(
    fg: C,
    lifetime: T,
) -> Effect {
    fade(Some(fg), None, lifetime.into(), true)
}

/// Fades to the specified the background and foreground colors over the specified duration.
pub fn fade_to<T: Into<EffectTimer>, C: Into<Color>>(
    fg: C,
    bg: C,
    lifetime: T,
) -> Effect {
    fade(Some(fg), Some(bg), lifetime.into(), false)
}

/// Fades from the specified the background and foreground colors over the specified duration.
pub fn fade_from<T: Into<EffectTimer>, C: Into<Color>>(
    fg: C,
    bg: C,
    lifetime: T,
) -> Effect {
    fade(Some(fg), Some(bg), lifetime.into(), true)
}


/// Pauses for the specified duration.
pub fn sleep<T: Into<EffectTimer>>(duration: T) -> Effect {
    Sleep::new(duration).into_effect()
}

/// Consumes a single tick.
pub fn consume_tick() -> Effect {
    ConsumeTick::default().into_effect()
}

/// An effect that forces the wrapped effect to never report completion,
/// effectively making it run indefinitely. Once the effect reaches the end,
/// it will continue to process the effect without advancing the duration.
pub fn never_complete(effect: Effect) -> Effect {
    NeverComplete::new(effect).into_effect()
}

/// Wraps an effect and enforces a duration on it. Once the duration has
/// elapsed, the effect will be marked as complete.
pub fn with_duration(duration: Duration, effect: Effect) -> Effect {
    effect.with_duration(duration)
}

/// Creates an effect that runs indefinitely but has an enforced duration,
/// after which the effect will be marked as complete.
pub fn timed_never_complete(duration: Duration, effect: Effect) -> Effect {
    TemporaryEffect::new(never_complete(effect), duration).into_effect()
}


fn fade<C: Into<Color>>(
    fg: Option<C>,
    bg: Option<C>,
    lifetime: EffectTimer,
    reverse: bool,
) -> Effect {
    FadeColors::builder()
        .fg(fg.map(|c| c.into()))
        .bg(bg.map(|c| c.into()))
        .lifetime(if reverse { lifetime.reversed() } else { lifetime })
        .into()
}