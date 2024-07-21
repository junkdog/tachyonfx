use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use ratatui::buffer::Buffer;
use ratatui::layout::Size;
use ratatui::style::Color;

pub use glitch::Glitch;
use ping_pong::PingPong;
pub use shader_fn::*;
use slide::SlideCell;
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
mod slide;
mod moving_window;
mod offscreen_buffer;

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
/// let no_state = (); // no state to keep track of
///
/// fx::effect_fn(no_state, timer, |_state, context, cell_iter| {
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
///
/// ```no_run
/// use std::time::Instant;
/// use ratatui::style::Color;
/// use tachyonfx::fx;
///
/// fx::never_complete(fx::effect_fn(Instant::now(), 0, |state, _ctx, cell_iter| {
///     let cycle: f64 = (state.elapsed().as_millis() % 3600) as f64;
///
///     cell_iter
///         .filter(|(_, cell)| cell.symbol() != " ")
///         .enumerate()
///         .for_each(|(i, (_pos, cell))| {
///             let hue = (2.0 * i as f64 + cycle * 0.2) % 360.0;
///             let color = Color::from_hsl(hue, 100.0, 50.0);
///             cell.set_fg(color);
///         });
/// }));
/// ```
///
/// This example creates an effect that runs indefinitely and cycles the color of each
/// foreground cell based on the elapsed time. Each cell's color is slightly offset by
/// the cell's position.
///
pub fn effect_fn<F, S, T>(state: S, timer: T, f: F) -> Effect
where
    S: Clone + 'static,
    T: Into<EffectTimer>,
    F: FnMut(&mut S, ShaderFnContext, CellIterator) + 'static,
{
    ShaderFn::with_iterator(state, f, timer).into_effect()
}

/// Creates a custom effect using a user-defined function that operates on a buffer.
///
/// This function allows you to define custom effects by providing a closure that will be called
/// with the current state, `ShaderFnContext`, and a mutable buffer. You can use this closure
/// to apply custom transformations or animations to the terminal buffer. The function also takes
/// an initial state that can be used to maintain state across invocations.
///
/// # Arguments
/// * `state` - An initial state that will be passed to the closure on each invocation.
/// * `timer` - An `EffectTimer` instance to control the duration and timing of the effect.
/// * `f` - A closure that defines the custom effect. The closure takes three parameters:
///   * `state`: A mutable reference to the state provided during the creation of the effect.
///   * `context`: A `ShaderFnContext` instance containing timing and area information.
///   * `buffer`: A mutable reference to the terminal buffer.
///
/// # Returns
/// * An `Effect` instance that can be used with other effects or applied directly to terminal cells.
pub fn effect_fn_buf<F, S, T>(state: S, timer: T, f: F) -> Effect
where
    S: Clone + 'static,
    T: Into<EffectTimer>,
    F: FnMut(&mut S, ShaderFnContext, &mut Buffer) + 'static,
{
    ShaderFn::with_buffer(state, f, timer).into_effect()
}

/// changes the hue, saturation, and lightness of the foreground and background colors.
pub fn hsl_shift<T: Into<EffectTimer>>(
    hsl_fg_change: Option<[f32; 3]>,
    hsl_bg_change: Option<[f32; 3]>,
    timer: T,
) -> Effect {
    HslShift::builder()
        .hsl_mod_fg(hsl_fg_change)
        .hsl_mod_bg(hsl_bg_change)
        .timer(timer.into())
        .into()
}

/// Shifts the foreground color by the specified hue, saturation, and lightness
/// over the specified duration.
pub fn hsl_shift_fg<T: Into<EffectTimer>>(
    hsl_fg_change: [f32; 3],
    timer: T,
) -> Effect {
    hsl_shift(Some(hsl_fg_change), None, timer)
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
    timer: T,
) -> Effect {
    sweep_in(direction.flipped(), gradient_length, faded_color, timer)
        .reversed()
}

/// Sweeps in a from the specified color.
pub fn sweep_in<T: Into<EffectTimer>, C: Into<Color>>(
    direction: Direction,
    gradient_length: u16,
    faded_color: C,
    timer: T,
) -> Effect {
    SweepIn::new(direction, gradient_length, faded_color.into(), timer.into())
        .into_effect()
}

/// Creates an effect that slides terminal cells in from a specified direction with a gradient.
///
/// This function creates a sliding effect that moves terminal cells in from a specified direction.
/// The effect can include a gradient length and a color behind the cells. The effect duration and
/// timing are controlled by the provided timer.
///
/// # Arguments
/// * `direction` - The direction from which the cells slide in.
/// * `gradient_length` - The length of the gradient used for the sliding effect.
/// * `color_behind_cells` - The color behind the sliding cells.
/// * `timer` - An `EffectTimer` instance to control the duration and timing of the effect.
///
/// # Returns
/// * An `Effect` instance that applies the sliding-in effect.
///
/// # Usage Notes
/// This effect should be applied before rendering any affected `ratatui` widgets. Other effects,
/// such as `fx::dissolve` or `fx::fade_to_fg`, are applied after rendering. You can manually retrieve
/// the currently recalculated draw area using the `area()` function of the effect.
///
/// # Examples
///
/// ```no_run
/// use ratatui::style::Color;
/// use tachyonfx::*;
/// use tachyonfx::fx::Direction;
///
/// let timer = EffectTimer::from_ms(2000, Interpolation::Linear);
/// let slide_effect = fx::slide_in(Direction::LeftToRight, 10, Color::Black, timer);
/// ```
///
/// This example creates a sliding effect that moves cells in from the left to the right
/// with a gradient length of 10 and a black background color over two seconds.
pub fn slide_in<T: Into<EffectTimer>, C: Into<Color>>(
    direction: Direction,
    gradient_length: u16,
    color_behind_cells: C,
    timer: T,
) -> Effect {
    slide_out(direction.flipped(), gradient_length, color_behind_cells, timer)
        .reversed()
}

/// Creates an effect that slides terminal cells out to a specified direction with a gradient.
///
/// This function creates a sliding effect that moves terminal cells out to a specified direction.
/// The effect can include a gradient length and a color behind the cells. The effect duration and
/// timing are controlled by the provided timer.
///
/// # Arguments
/// * `direction` - The direction in which the cells slide out.
/// * `gradient_length` - The length of the gradient used for the sliding effect.
/// * `color_behind_cells` - The color behind the sliding cells.
/// * `timer` - An `EffectTimer` instance to control the duration and timing of the effect.
///
/// # Returns
/// * An `Effect` instance that applies the sliding-out effect.
///
/// # Examples
///
/// ```no_run
/// use ratatui::style::Color;
/// use tachyonfx::*;
/// use tachyonfx::fx::Direction;
///
/// let timer = EffectTimer::from_ms(2000, Interpolation::CubicInOut);
/// let slide_effect = fx::slide_out(Direction::UpToDown, 10, Color::Black, timer);
/// ```
///
/// This example creates a sliding effect that moves cells out from the top to the bottom
/// with a gradient length of 10 and a black background color over two seconds.
pub fn slide_out<T: Into<EffectTimer>, C: Into<Color>>(
    direction: Direction,
    gradient_length: u16,
    color_behind_cells: C,
    timer: T,
) -> Effect {
    let timer: EffectTimer = timer.into();
    let timer = match direction {
        Direction::LeftToRight => timer,
        Direction::RightToLeft => timer.reversed(),
        Direction::UpToDown    => timer,
        Direction::DownToUp    => timer.reversed(),
    };

    SlideCell::builder()
        .timer(timer)
        .color_behind_cell(color_behind_cells.into())
        .gradient_length(gradient_length)
        .direction(direction)
        .into()
}

/// Translates an effect by a specified amount over a specified duration.
///
/// This function creates a translation effect that moves an existing effect by a given
/// amount of rows and columns over the specified duration. If no effect is provided, only
/// the translation is applied.
///
/// # Arguments
/// * `fx` - An optional `Effect`, receives the .
/// * `translate_by` - A tuple specifying the number of rows and columns to translate the effect by.
/// * `timer` - An `EffectTimer` instance to control the duration and timing of the translation.
///
/// # Returns
/// * An `Effect` instance that applies the translation to the given effect or as a standalone effect.
///
/// # Usage Notes
/// This effect should be applied before rendering any affected `ratatui` widgets. Other effects,
/// such as `fx::dissolve` or `fx::slide_in`, are applied after rendering. You can manually retrieve
/// the currently recalculated draw area using the `area()` function of the effect.
///
/// # Examples
///
/// ```no_run
/// use ratatui::style::Color;
/// use tachyonfx::*;
///
/// let timer = EffectTimer::from_ms(1000, Interpolation::Linear);
/// let effect = fx::fade_to_fg(Color::Red, timer);
/// fx::translate(Some(effect), (5, 10), timer);
/// ```
///
/// This example creates a translation effect that moves a fade-to-red effect by 5 rows
/// and 10 columns over one second.
pub fn translate<T: Into<EffectTimer>>(
    fx: Option<Effect>,
    translate_by: (i16, i16),
    timer: T,
) -> Effect {
    translate::Translate::new(fx, translate_by, timer.into()).into_effect()
}

/// Resizes the area of the wrapped effect to the specified dimensions over a specified duration.
///
/// This function creates a resizing effect that changes the dimensions of an existing effect's
/// rendering area over the specified duration. If no effect is provided, only the resizing is applied.
///
/// # Arguments
/// * `fx` - An optional `Effect`, receives the resized area.
/// * `initial_size` - A `Size` instance specifying the initial dimensions of the effect area.
/// * `timer` - An `EffectTimer` instance to control the duration and timing of the resizing.
///
/// # Returns
/// * An `Effect` instance that applies the resizing to the given effect or as a standalone effect.
///
/// # Usage Notes
/// This effect should be applied before rendering any affected `ratatui` widgets. Most other effects,
/// such as `fx::dissolve` or `fx::slide_in`, are applied after rendering. You can manually retrieve
/// the currently recalculated draw area using the `area()` function of the effect.
///
/// # Examples
///
/// ```no_run
/// use ratatui::layout::Size;
/// use ratatui::style::Color;
/// use tachyonfx::*;
///
/// let timer = EffectTimer::from_ms(2, Interpolation::CubicInOut);
/// let effect = fx::fade_to_fg(Color::Blue, timer);
/// fx::resize_area(Some(effect), Size::new(20, 10), timer);
/// ```
///
/// This example creates a resizing effect that changes the dimensions of a fade-to-blue effect's
/// rendering area to 20 by 10 over two seconds.
pub fn resize_area<T: Into<EffectTimer>>(
    fx: Option<Effect>,
    initial_size: Size,
    timer: T,
) -> Effect {
    ResizeArea::new(fx, initial_size, timer.into()).into_effect()
}

/// Creates an effect that renders to an offscreen buffer.
///
/// This function wraps an existing effect and redirects its rendering to a separate buffer,
/// allowing for complex effects to be computed without affecting the main render buffer.
/// The offscreen buffer can then be composited onto the main buffer as needed.
///
/// # Arguments
/// * `fx` - The effect to be rendered offscreen.
/// * `render_target` - A shared, mutable reference to the offscreen `Buffer`.
///
/// # Returns
/// * An `Effect` that renders to the specified offscreen buffer.
///
/// # Examples
///
///
/// ```no_run
/// use std::cell::RefCell;
/// use std::rc::Rc;
/// use ratatui::prelude::{Buffer, Color, Rect};
/// use tachyonfx::{fx, Effect, EffectTimer, Interpolation};
///
/// let area = Rect::new(0, 0, 80, 24);
/// let offscreen_buffer = Rc::new(RefCell::new(Buffer::empty(area)));
///
/// let fade_effect = fx::fade_to_fg(Color::Red, EffectTimer::from_ms(1000, Interpolation::Linear));
/// let offscreen_effect = fx::offscreen_buffer(fade_effect, offscreen_buffer.clone());
///
/// // Later, in your rendering loop:
/// offscreen_effect.process(duration, &mut main_buffer, area);
/// // Composite the offscreen buffer onto the main buffer as needed
/// ```
///
/// This example creates an offscreen buffer and applies a fade effect to it. The effect can be
/// processed independently of the main render buffer, allowing for more complex or
/// performance-intensive effects to be computed separately.
pub fn offscreen_buffer(fx: Effect, render_target: Rc<RefCell<Buffer>>) -> Effect {
    offscreen_buffer::OffscreenBuffer::new(fx, render_target).into_effect()
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
pub fn dissolve<T: Into<EffectTimer>>(cycle_len: usize, timer: T) -> Effect {
    Dissolve::new(timer.into(), cycle_len)
        .into_effect()
}

/// The reverse of [dissolve()].
pub fn coalesce<T: Into<EffectTimer>>(cycle_len: usize, timer: T) -> Effect {
    Dissolve::new(timer.into().reversed(), cycle_len)
        .into_effect()
}


/// Fades the foreground color to the specified color over the specified duration.
pub fn fade_to_fg<T: Into<EffectTimer>, C: Into<Color>>(
    fg: C,
    timer: T,
) -> Effect {
    fade(Some(fg), None, timer.into(), false)
}

/// Fades the foreground color from the specified color over the specified duration.
pub fn fade_from_fg<T: Into<EffectTimer>, C: Into<Color>>(
    fg: C,
    timer: T,
) -> Effect {
    fade(Some(fg), None, timer.into(), true)
}

/// Fades to the specified the background and foreground colors over the specified duration.
pub fn fade_to<T: Into<EffectTimer>, C: Into<Color>>(
    fg: C,
    bg: C,
    timer: T,
) -> Effect {
    fade(Some(fg), Some(bg), timer.into(), false)
}

/// Fades from the specified the background and foreground colors over the specified duration.
pub fn fade_from<T: Into<EffectTimer>, C: Into<Color>>(
    fg: C,
    bg: C,
    timer: T,
) -> Effect {
    fade(Some(fg), Some(bg), timer.into(), true)
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
    timer: EffectTimer,
    reverse: bool,
) -> Effect {
    FadeColors::builder()
        .fg(fg.map(|c| c.into()))
        .bg(bg.map(|c| c.into()))
        .timer(if reverse { timer.reversed() } else { timer })
        .into()
}
