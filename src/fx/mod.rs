use ratatui::buffer::Buffer;
use ratatui::layout::{Offset, Size};
use ratatui::style::Color;

pub use glitch::Glitch;
use ping_pong::PingPong;
use prolong::{Prolong, ProlongPosition};
pub use shader_fn::*;
use slide::SlideCell;
pub use direction::*;
use crate::{CellIterator, Duration, RefCount, ThreadSafetyMarker};
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
use crate::fx::translate_buffer::TranslateBuffer;

mod ansi256;
mod consume_tick;
pub(crate) mod containers;
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
mod translate_buffer;
mod hsl_shift;
mod shader_fn;
mod slide;
mod sliding_window_alpha;
mod offscreen_buffer;
mod prolong;
mod direction;

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
/// use tachyonfx::{fx, HslConvertable};
///
/// fx::never_complete(fx::effect_fn(Instant::now(), 0, |state, _ctx, cell_iter| {
///     let cycle: f32 = (state.elapsed().as_millis() % 3600) as f32;
///
///     cell_iter
///         .filter(|(_, cell)| cell.symbol() != " ")
///         .enumerate()
///         .for_each(|(i, (_pos, cell))| {
///             let hue = (2.0 * i as f32 + cycle * 0.2) % 360.0;
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
    S: Clone + Send + 'static,
    T: Into<EffectTimer>,
    F: FnMut(&mut S, ShaderFnContext, CellIterator) + ThreadSafetyMarker + 'static,
{
    ShaderFn::builder()
        .name("shader_fn")
        .state(state)
        .code(ShaderFnSignature::new_iter(f))
        .timer(timer)
        .build()
        .into_effect()
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
    S: Clone + Send + 'static,
    T: Into<EffectTimer>,
    F: FnMut(&mut S, ShaderFnContext, &mut Buffer) + ThreadSafetyMarker + 'static,
{
    ShaderFn::builder()
        .name("shader_fn_buf")
        .state(state)
        .code(ShaderFnSignature::new_buffer(f))
        .timer(timer)
        .build()
        .into_effect()
}

/// changes the hue, saturation, and lightness of the foreground and background colors.
pub fn hsl_shift<T: Into<EffectTimer>>(
    hsl_fg_change: Option<[f32; 3]>,
    hsl_bg_change: Option<[f32; 3]>,
    timer: T,
) -> Effect {
    if hsl_fg_change.is_none() && hsl_bg_change.is_none() {
        panic!("At least one of the foreground or background color must be changed");
    }

    HslShift::builder()
        .maybe_hsl_mod_fg(hsl_fg_change)
        .maybe_hsl_mod_bg(hsl_bg_change)
        .timer(timer.into())
        .build()
        .into_effect()
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

/// Creates an effect that sweeps out from a specified color with optional randomness.
///
/// Refer to [`sweep_in`](fn.sweep_in.html) for more information.
pub fn sweep_out<T: Into<EffectTimer>, C: Into<Color>>(
    direction: Direction,
    gradient_length: u16,
    randomness: u16,
    faded_color: C,
    timer: T,
) -> Effect {
    sweep_in(direction.flipped(), gradient_length, randomness, faded_color, timer)
        .reversed()
}

/// Creates an effect that sweeps in from a specified color with optional randomness.
///
/// This function generates a sweeping effect that transitions from a specified color
/// to the original content. The sweep can be applied in any of the four cardinal directions
/// and includes options for gradient length and randomness to create more dynamic effects.
///
/// # Arguments
///
/// * `direction` - The direction of the sweep effect. Can be one of:
///   - `Direction::LeftToRight`
///   - `Direction::RightToLeft`
///   - `Direction::UpToDown`
///   - `Direction::DownToUp`
///
/// * `gradient_length` - The length of the gradient transition in cells. This determines
///   how smooth the transition is between the faded color and the original content.
///
/// * `randomness` - The maximum random offset applied to each column or row of the effect.
///   Higher values create a more irregular, "noisy" transition. Set to 0 for a uniform sweep.
///
/// * `faded_color` - The color from which the content sweeps in.
///
/// * `timer` - Controls the duration and timing of the effect.
///
/// # Returns
///
/// Returns a sweep `Effect`.
///
/// # Examples
///
/// Basic usage:
/// ```
/// use tachyonfx::{fx, EffectTimer, Interpolation};
/// use tachyonfx::fx::Direction;
/// use ratatui::style::Color;
///
/// let sweep_effect = fx::sweep_in(
///     Direction::LeftToRight,
///     10,
///     0,
///     Color::Blue,
///     EffectTimer::from_ms(1000, Interpolation::Linear)
/// );
/// ```
///
/// With randomness:
/// ```
/// use tachyonfx::{fx, EffectTimer, Interpolation};
/// use tachyonfx::fx::Direction;
/// use ratatui::style::Color;
///
/// let sweep_effect = fx::sweep_in(
///     Direction::UpToDown,
///     15,
///     5,
///     Color::Cyan,
///     EffectTimer::from_ms(2000, Interpolation::QuadOut)
/// );
/// ```
///
/// # See Also
///
/// * [`sweep_out`](fn.sweep_out.html) - For the reverse effect.
pub fn sweep_in<T: Into<EffectTimer>, C: Into<Color>>(
    direction: Direction,
    gradient_length: u16,
    randomness: u16,
    faded_color: C,
    timer: T,
) -> Effect {
    SweepIn::new(direction, gradient_length, randomness, faded_color.into(), timer.into())
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
/// let slide_effect = fx::slide_in(Direction::LeftToRight, 10, 0, Color::Black, timer);
/// ```
///
/// This example creates a sliding effect that moves cells in from the left to the right
/// with a gradient length of 10 and a black background color over two seconds.
pub fn slide_in<T: Into<EffectTimer>, C: Into<Color>>(
    direction: Direction,
    gradient_length: u16,
    randomness: u16,
    color_behind_cells: C,
    timer: T,
) -> Effect {
    slide_out(direction.flipped(), gradient_length, randomness, color_behind_cells, timer)
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
/// let slide_effect = fx::slide_out(Direction::UpToDown, 10, 0, Color::Black, timer);
/// ```
///
/// This example creates a sliding effect that moves cells out from the top to the bottom
/// with a gradient length of 10 and a black background color over two seconds.
pub fn slide_out<T: Into<EffectTimer>, C: Into<Color>>(
    direction: Direction,
    gradient_length: u16,
    randomness: u16,
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
        .randomness_extent(randomness)
        .direction(direction)
        .build()
        .into_effect()
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

/// Creates an effect that translates the contents of an auxiliary buffer onto the main buffer.
///
/// This function creates a `TranslateBuffer` shader, which efficiently translates pre-rendered
/// content without re-rendering it on every frame. It's particularly useful for large or complex
/// content that doesn't change frequently.
///
/// # Arguments
///
/// * `translate_by` - An `Offset` specifying the final translation amount.
/// * `timer` - Specifies the duration and interpolation of the translation effect. Can be any type
///   that implements `Into<EffectTimer>`.
/// * `aux_buffer` - A shared reference to the auxiliary buffer containing the pre-rendered content
///   to be translated.
///
/// # Returns
///
/// Returns an `Effect` that can be used with other effects or applied directly to a buffer.
pub fn translate_buf<T: Into<EffectTimer>>(
    translate_by: Offset,
    aux_buffer: RefCount<Buffer>,
    timer: T,
) -> Effect {
    TranslateBuffer::new(aux_buffer, translate_by, timer.into()).into_effect()
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
/// use tachyonfx::{fx, ref_count, Duration, Effect, EffectTimer, Interpolation, Shader};
///
/// let duration = Duration::from_millis(16);
/// let mut main_buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
///
/// let area = Rect::new(0, 0, 80, 24);
/// let offscreen_buffer = ref_count(Buffer::empty(area));
///
/// let fade_effect = fx::fade_to_fg(Color::Red, EffectTimer::from_ms(1000, Interpolation::Linear));
/// let mut offscreen_effect = fx::offscreen_buffer(fade_effect, offscreen_buffer.clone());
///
/// // Later, in your rendering loop
/// offscreen_effect.process(duration, &mut main_buffer, area);
/// // Composite the offscreen buffer onto the main buffer as needed
/// ```
///
/// This example creates an offscreen buffer and applies a fade effect to it. The effect can be
/// processed independently of the main render buffer, allowing for more complex or
/// performance-intensive effects to be computed separately.
pub fn offscreen_buffer(fx: Effect, render_target: RefCount<Buffer>) -> Effect {
    offscreen_buffer::OffscreenBuffer::new(fx, render_target).into_effect()
}

/// Runs the effects in sequence, one after the other. Reports completion
/// once the last effect has completed.
pub fn sequence(effects: &[Effect]) -> Effect {
    SequentialEffect::new(effects.into()).into_effect()
}

/// Runs the effects in parallel, all at the same time. Reports completion
/// once all effects have completed.
pub fn parallel(effects: &[Effect]) -> Effect {
    ParallelEffect::new(effects.into()).into_effect()
}

/// Dissolves the current text into the new text over the specified duration. The
/// `cycle_len` parameter specifies the number of cell states are tracked before
/// it cycles and repeats.
pub fn dissolve<T: Into<EffectTimer>>(timer: T) -> Effect {
    Dissolve::new(timer.into())
        .into_effect()
}

/// The reverse of [dissolve()].
pub fn coalesce<T: Into<EffectTimer>>(timer: T) -> Effect {
    Dissolve::new(timer.into().reversed())
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

/// Creates an effect that pauses for the specified duration.
///
/// This function creates an effect that does nothing for the given duration,
/// effectively creating a pause or delay in a sequence of effects.
///
/// # Arguments
///
/// * `duration` - The duration of the sleep effect. This can be any type that
///   can be converted into an `EffectTimer`.
///
/// # Returns
///
/// An `Effect` that, when processed, will pause for the specified duration.
pub fn sleep<T: Into<EffectTimer>>(duration: T) -> Effect {
    Sleep::new(duration).into_effect()
}

/// Creates an effect that delays the execution of another effect.
///
/// This function creates a sequence of two effects: a sleep effect followed by
/// the provided effect. This effectively delays the start of the provided effect
/// by the specified duration.
///
/// # Arguments
///
/// * `duration` - The duration of the delay. This can be any type that can be
///   converted into an `EffectTimer`.
/// * `effect` - The effect to be delayed.
///
/// # Returns
///
/// An `Effect` that, when processed, will first pause for the specified duration
/// and then apply the provided effect.
///
/// # Examples
///
/// ```
/// use tachyonfx::{fx, Duration, Effect, EffectTimer};
/// use ratatui::style::Color;
///
/// let fade_effect = fx::fade_to_fg(Color::Red, Duration::from_secs(1));
/// let delayed_fade: Effect = fx::delay(Duration::from_secs(2), fade_effect);
/// ```
pub fn delay<T: Into<EffectTimer>>(duration: T, effect: Effect) -> Effect {
    sequence(&[sleep(duration), effect])
}

/// Creates an effect that prolongs the start of another effect.
///
/// This function wraps the given effect with additional duration at its beginning.
/// The original effect will not progress until the additional duration has elapsed.
/// During this time, the wrapped effect will be processed with zero duration.
///
/// # Arguments
///
/// * `duration` - The additional duration to add before the effect starts. This can be
///                any type that can be converted into an `EffectTimer`.
/// * `effect` - The original effect to be prolonged.
///
/// # Returns
///
/// A new `Effect` that includes the additional duration at the start.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use ratatui::style::Color;
/// use tachyonfx::{Effect, fx, EffectTimer, Interpolation};
///
/// fx::prolong_start(500, // 500ms
///     fx::fade_from_fg(Color::Red, EffectTimer::from_ms(1000, Interpolation::Linear))
/// );
/// ```
/// This example creates an effect that waits for 500ms before starting a fade effect from red to
/// the original color over 1000ms. The total duration of this combined effect will be 1500ms.
pub fn prolong_start<T: Into<EffectTimer>>(duration: T, effect: Effect) -> Effect {
    Prolong::new(ProlongPosition::Start, duration.into(), effect).into_effect()
}

/// Creates an effect that prolongs the end of another effect.
///
/// This function wraps the given effect with additional duration at its end.
/// The original effect will complete its normal progression, then the additional
/// duration will keep the effect in its final state for the specified time.
///
/// # Arguments
///
/// * `duration` - The additional duration to add after the effect completes. This can be
///                any type that can be converted into an `EffectTimer`.
/// * `effect` - The original effect to be prolonged.
///
/// # Returns
///
/// A new `Effect` that includes the additional duration at the end.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use ratatui::style::Color;
/// use tachyonfx::{Effect, fx, EffectTimer, Interpolation};
///
/// fx::prolong_end(500, // 500ms
///     fx::fade_to_fg(Color::Red, EffectTimer::from_ms(1000, Interpolation::Linear))
/// );
/// ```
///
/// This example creates an effect that fades the foreground color to red over 1000ms,
/// then holds the red color for an additional 500ms. The total duration of this combined
/// effect will be 1500ms.
pub fn prolong_end<T: Into<EffectTimer>>(duration: T, effect: Effect) -> Effect {
    Prolong::new(ProlongPosition::End, duration.into(), effect).into_effect()
}

/// Creates an effect that consumes a single tick of processing time.
///
/// This function creates an effect that does nothing but mark itself as complete
/// after a single processing tick. It can be useful for creating very short pauses
/// or for synchronizing effects in complex sequences.
///
/// # Returns
///
/// An `Effect` that completes after a single processing tick.
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
    if fg.is_none() && bg.is_none() {
        panic!("At least one of fg or bg must be provided");
    }

    FadeColors::builder()
        .maybe_fg(fg.map(Into::into))
        .maybe_bg(bg.map(Into::into))
        .timer(if reverse { timer.reversed() } else { timer })
        .build()
        .into_effect()
}


#[cfg(feature = "sendable")]
macro_rules! invoke_fn {
    // Arc<Mutex<F>> for sendable
    ($f:expr, $($args:expr),* $(,)?) => {
        $f.lock().unwrap()($($args),*)
    };
}

#[cfg(not(feature = "sendable"))]
macro_rules! invoke_fn {
    // Rc<Arc<F>> for non-sendable
    ($f:expr, $($args:expr),* $(,)?) => {
        $f.borrow_mut()($($args),*)
    };
}

pub (crate) use invoke_fn;

#[cfg(test)]
mod tests {
    use ratatui::prelude::Color;
    use crate::fx::offscreen_buffer::OffscreenBuffer;
    use crate::fx::translate::Translate;
    use super::*;
    use crate::Shader;

    const DIRECTIONS: [Direction; 4] = [
        Direction::DownToUp,
        Direction::UpToDown,
        Direction::LeftToRight,
        Direction::RightToLeft,
    ];

    #[test]
    fn test_name_fade() {
        assert_eq!(
            fade_to(Color::Red, Color::Green, 1000).name(),
            "fade_to"
        );

        assert_eq!(
            fade_from_fg(Color::Red, 1000).name(),
            "fade_from"
        );

        assert_eq!(
            fade_to(Color::Red, Color::Green, 1000).reversed().name(),
            "fade_from"
        );

        assert_eq!(
            fade_from_fg(Color::Red, 1000).reversed().name(),
            "fade_to"
        );
    }

    #[test]
    fn test_name_sweep() {
        let c = Color::Red;

        DIRECTIONS.iter().for_each(|dir| {
            assert_eq!(sweep_out(*dir, 1, 0, c, 1000).name(), "sweep_out",
                "testing for direction={:?}", dir
            );
        });

        DIRECTIONS.iter().for_each(|dir| {
            assert_eq!(sweep_out(*dir, 1, 0, c, 1000).reversed().name(), "sweep_in",
                "testing reversed() for direction={:?}", dir
            );
        });

        DIRECTIONS.iter().for_each(|dir| {
            assert_eq!(sweep_in(*dir, 1, 0, c, 1000).name(), "sweep_in",
                "testing for direction={:?}", dir
            );
        });

        DIRECTIONS.iter().for_each(|dir| {
            assert_eq!(sweep_in(*dir, 1, 0, c, 1000).reversed().name(), "sweep_out",
                "testing reversed() for direction={:?}", dir
            );
        });
    }

    #[test]
    fn test_name_slide() {
        let c = Color::Red;

        let directions = [
            Direction::DownToUp,
            Direction::UpToDown,
            Direction::LeftToRight,
            Direction::RightToLeft,
        ];

        directions.iter().for_each(|dir| {
            assert_eq!(slide_out(*dir, 1, 0, c, 1000).name(), "slide_out",
                "testing for direction={:?}", dir
            );
        });

        directions.iter().for_each(|dir| {
            assert_eq!(slide_out(*dir, 1, 0, c, 1000).reversed().name(), "slide_in",
                "testing reversed() for direction={:?}", dir
            );
        });

        directions.iter().for_each(|dir| {
            assert_eq!(slide_in(*dir, 1, 0, c, 1000).name(), "slide_in",
                "testing for direction={:?}", dir
            );
        });

        directions.iter().for_each(|dir| {
            assert_eq!(slide_in(*dir, 1, 0, c, 1000).reversed().name(), "slide_out",
                "testing reversed() for direction={:?}", dir
            );
        });
    }

    #[test]
    fn assert_sizes() {
        let verify_size = |actual: usize, expected: usize| {
            assert_eq!(actual, expected);
        };

        verify_size(size_of::<EffectTimer>(),      12);
        verify_size(size_of::<Ansi256>(),          10);
        verify_size(size_of::<ConsumeTick>(),       1);
        verify_size(size_of::<Dissolve>(),         80);
        verify_size(size_of::<FadeColors>(),       80);
        verify_size(size_of::<Glitch>(),          112);
        verify_size(size_of::<HslShift>(),        104);
        verify_size(size_of::<NeverComplete>(),    16);
        verify_size(size_of::<OffscreenBuffer>(),  24);
        verify_size(size_of::<ParallelEffect>(),   24);
        verify_size(size_of::<PingPong>(),         72);
        verify_size(size_of::<Prolong>(),          32);
        verify_size(size_of::<Repeat>(),           32);
        verify_size(size_of::<ResizeArea>(),       56);
        verify_size(size_of::<SequentialEffect>(), 32);
        verify_size(size_of::<ShaderFn<()>>(),    112);
        verify_size(size_of::<Sleep>(),            12);
        verify_size(size_of::<SlideCell>(),        80);
        verify_size(size_of::<SweepIn>(),          80);
        verify_size(size_of::<TemporaryEffect>(),  32);
        verify_size(size_of::<Translate>(),        72);
        verify_size(size_of::<TranslateBuffer>(),  32);
    }
}
