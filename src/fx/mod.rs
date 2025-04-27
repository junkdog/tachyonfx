//! Effects in tachyonfx operate on terminal cells after widgets have been rendered to the screen.
//! When an effect is applied, it modifies properties of the already-rendered cells - like their
//! colors, characters, or visibility. This means that the typical flow is:
//!
//! 1. Render your widget to the screen
//! 2. Apply effects to transform the rendered content
//!
//! ## Color Effects 🎨
//! Color effects are used to modify or transition between colors, either for foreground text, background, or both. These are ideal for highlighting changes, drawing attention, or creating smooth visual transitions between states.
//!
//! | Effect              | Description | Example  |
//! |---------------------|-------------|----------|
//! | [`fade_from()`] ⟳     | Fades from specified colors            |![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/fade_from.gif) |
//! | [`fade_from_fg()`] ⟳  | Fades from specified foreground color  | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/fade_from_fg.gif) |
//! | [`fade_to()`] ⟳       | Fades to specified colors              | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/fade_to.gif) |
//! | [`fade_to_fg()`] ⟳    | Fades to specified foreground color    | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/fade_to_fg.gif) |
//! | [`hsl_shift()`] 🌈    | Changes hue, saturation, and lightness | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/hsl_shift.gif) |
//! | [`hsl_shift_fg()`] 🌈 | Changes foreground HSL values          | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/hsl_shift_fg.gif) |
//!
//! ## Text/Character Effects ✍️
//! Text effects modify the actual characters or their placement in the terminal. These are perfect for transitions, reveals, and dynamic text animations.
//!
//! | Effect                 | Description | Example  |
//! |------------------------|-------------|----------|
//! | [`coalesce()`] ⬆️      | Reforms dissolved foreground | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/coalesce.gif) |
//! | [`coalesce_from()`] ⬆️ | Reforms dissolved foreground | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/coalesce_from.gif) |
//! | [`explode()`] 💥       | Explodes content outward     | N/A |
//! | [`dissolve()`] ⬇️      | Dissolves foreground content | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/dissolve.gif) |
//! | [`dissolve_to()`] ⬇️   | Dissolves foreground content | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/dissolve_to.gif) |
//! | [`slide_in()`] ↔️      | Slides content with gradient | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/slide_in.gif) |
//! | [`slide_out()`] ↔️     | Slides content with gradient | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/slide_out.gif) |
//! | [`sweep_in()`] ↔️      | Sweeps content with color    | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/sweep_in.gif) |
//! | [`sweep_out()`] ↔️     | Sweeps content with color    | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/sweep_out.gif) |
//!
//! ## Timing and Control Effects ⏱️
//! Control effects modify how other effects behave over time. They're essential for creating complex animations and controlling the flow of multiple effects.
//!
//! | Effect              | Description | Example  |
//! |---------------------|-------------|----------|
//! | [`consume_tick()`] ⌛ | Consumes a single tick            | N/A |
//! | [`delay()`] ⏳ | Delays effect by specified duration      | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/delay.gif)|
//! | [`freeze_at()`] ⏳ | Freezes another effect at a specific alpha (transition) value      | N/A |
//! | [`never_complete()`] ♾️ | Makes effect run indefinitely   | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/never_complete.gif) |
//! | [`ping_pong()`] 🔄 | Plays effect forward then backward   | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/ping_pong.gif)|
//! | [`prolong_start()`] ⏳ | Extends effect duration          | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/prolong_start.gif)|
//! | [`prolong_end()`] ⏳ | Extends effect duration            | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/prolong_end.gif)|
//! | [`remap_alpha()`] 🔁 | Remaps an effect's alpha progression to operate within a smaller range | N/A |
//! | [`repeat()`] 🔁 | Repeats effect by count or duration     | N/A |
//! | [`repeating()`] 🔁 | Repeats an effect indefinitely       | N/A |
//! | [`sleep()`] 💤 | Pauses for specified duration            | N/A |
//! | [`with_duration()`] ⏱️ | Applies duration limit to effect | N/A |
//!
//!
//! ## Geometry Effects 📐
//! Geometry effects modify the position or size of content. These are useful for creating dynamic layouts and transitions.
//!
//! | Effect                 | Description | Example  |
//! |------------------------|-------------|----------|
//! | [`translate()`] ➡️     | Moves effect area     | N/A |
//! | [`translate_buf()`] ➡️ | Moves buffer contents | N/A |
//! | [`resize_area()`] ⬌   | Resizes effect area   | N/A |
//!
//! ## Combination Effects 🔗
//! Combination effects allow multiple effects to be composed together. These are crucial for creating complex animations.
//!
//! | Effect              | Description | Example  |
//! |---------------------|-------------|----------|
//! | [`parallel()`] ⫽ | Runs effects simultaneously | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/parallel.gif) |
//! | [`sequence()`] ⟶ | Runs effects sequentially   | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/sequence.gif) |
//!
//! ## Other Effects 🛠️
//! Advanced effects for custom behaviors or quick one-off effects.
//!
//! | Effect & Description | Preview | Example |
//! |---------------------|---------|----------|
//! | [`effect_fn()`] 🔧        | Custom effects with cell iterator | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/effect_fn.gif) |
//! | [`effect_fn_buf()`] 🔧    | Custom effects with buffer        | ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/effect_fn_buf.gif) |
//! | [`offscreen_buffer()`] 📺 | Renders to separate buffer        | N/A |
//!
//! Additional effects can be created by implementing the [Shader](crate::Shader) trait.

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Offset, Size};
use ratatui::style::{Color, Style};

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
use crate::fx::temporary::TemporaryEffect;
use crate::fx::translate_buffer::TranslateBuffer;
use crate::{CellIterator, ColorSpace, Duration, Motion, RefCount, ThreadSafetyMarker};
pub use direction::*;
pub use glitch::Glitch;
use ping_pong::PingPong;
use prolong::{Prolong, ProlongPosition};
pub use repeat::RepeatMode;
pub use shader_fn::*;
use slide::SlideCell;
pub use temporary::IntoTemporaryEffect;

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
pub(crate) mod unique;
mod explode;
mod alpha_xform;

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
///    let alpha = context.alpha();
///    let mut fg_cache: LruCache<Color, Color, 4> = LruCache::default();
///
///    for (_pos, cell) in cell_iter {
///        // context.timer.progress() is already interpolated, so we can linearly lerp to the target color
///        let color = fg_cache.memoize(&cell.fg, |c| c.lerp(&Color::Indexed(35), alpha));
///        cell.set_fg(color);
///    }
/// }).filter(CellFilter::FgColor(Color::DarkGray));
/// ```
///
/// In this example, the custom effect function interpolates the foreground color of each
/// cell to a new color over the specified duration. The effect is only applied to cells with
/// a foreground color of `Color::DarkGray`.
///
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/effect_fn.gif)
///
/// ```no_run
/// use std::time::Instant;
/// use ratatui::style::Color;
/// use tachyonfx::{color_from_hsl, fx};
///
/// fx::effect_fn(Instant::now(), 1000, |state, _ctx, cell_iter| {
///     let cycle: f32 = (state.elapsed().as_millis() % 3600) as f32;
///     cell_iter
///         .filter(|(_, cell)| cell.symbol() != " ")
///         .enumerate()
///         .for_each(|(i, (_pos, cell))| {
///             let hue = (2.0 * i as f32 + cycle * 0.2) % 360.0;
///             let color = color_from_hsl(hue, 100.0, 50.0);
///             cell.set_fg(color);
///     });
/// });
/// ```
///
/// This example creates an effect that runs indefinitely and cycles the color of each
/// foreground cell based on the elapsed time. Each cell's color is slightly offset by
/// the cell's position.
///
pub fn effect_fn<F, S, T>(state: S, timer: T, f: F) -> Effect
where
    S: Clone + ThreadSafetyMarker + 'static,
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
///
/// # Examples
///
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/effect_fn_buf.gif)
///
/// ```no_run
/// use ratatui::style::Color;
/// use tachyonfx::*;
///
/// let timer = EffectTimer::from_ms(1000, Interpolation::Linear);
/// let no_state = (); // no state to keep track of
///
/// fx::effect_fn_buf(no_state, timer, |_state, context, buf| {
///     let offset = context.timer.remaining().as_millis() as usize;
///
///     let cell_pred = context.filter.unwrap_or(CellFilter::All).selector(buf.area);
///     for (i, pos) in buf.area.positions().enumerate() {
///         let cell = &mut buf[pos];
///         if !cell_pred.is_valid(pos, &cell) {
///             continue;
///         }
///         cell.set_fg(Color::Indexed(((offset + i) % 256) as u8));
///     }
/// }).filter(CellFilter::Text);
/// ```
///
/// This example creates an effect that runs for 1s and cycles the color of the
/// text based on the elapsed time. Each cell's color is slightly offset by
/// the cell's position.
pub fn effect_fn_buf<F, S, T>(state: S, timer: T, f: F) -> Effect
where
    S: Clone + ThreadSafetyMarker + 'static,
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
///
/// # Examples
///
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/hsl_shift.gif)
///
/// ```no_run
/// // shift the hue of the entire area
/// use tachyonfx::{fx, Interpolation};
///
/// let timer = (1000, Interpolation::Linear);
/// let fg_shift = [120.0, 25.0, 25.0];
/// let bg_shift = [-40.0, -50.0, -50.0];
/// fx::hsl_shift(Some(fg_shift), Some(bg_shift), timer);
/// ```
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
///
/// # Examples
///
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/hsl_shift_fg.gif)
///
/// ```no_run
/// use tachyonfx::{fx, Interpolation};
///
/// // shift the hue of the entire area
/// let timer = (1000, Interpolation::Linear);
/// let fg_shift = [120.0, 25.0, 25.0];
/// fx::hsl_shift(Some(fg_shift), None, timer);
/// ```
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

/// Creates an explosion effect where content disperses outward from the center.
///
/// This effect simulates an explosion by moving cells away from the center of the
/// specified area, with their appearance changing over time to represent debris.
///
/// The original cells are replaced with the `Color::Black` for both foreground and
/// background. No modifiers are retained.
///
/// # Arguments
///
/// * `force` - Base explosion force determining how far cells move outward. Higher values
///             create more dramatic explosions with cells moving farther from the center.
///
/// * `force_rng_factor` - Randomization factor for explosion force. Higher values create
///                        more varied and chaotic explosions, with some cells moving faster
///                        than others. Set to 0.0 for uniform movement.
///
/// * `timer` - Controls the duration and interpolation of the effect.
///
/// # Returns
///
/// * An `Effect` that creates an explosion animation when processed.
///
/// # Examples
///
/// ```no_run
/// use tachyonfx::{fx, Interpolation::Linear};
/// use ratatui::layout::Rect;
/// use ratatui::style::Color;
///
/// let timer = (1000, Linear);
///
/// fx::parallel(&[
///     fx::fade_to_fg(Color::from_u32(0x404040), timer),
///     fx::explode(15.0, 2.0, timer),
/// ]);
/// ```
pub fn explode(
    force: f32,
    force_rng_factor: f32,
    timer: impl Into<EffectTimer>,
) -> Effect {
    let mut replacement_cell = Cell::default();
    replacement_cell.set_fg(Color::Black);
    replacement_cell.set_bg(Color::Black);
    Explode::new(force, force_rng_factor, replacement_cell, timer.into()).into_effect()
}

/// Freezes an effect at a specific alpha (transition) value.
///
/// # Arguments
///
/// * `alpha` - The alpha value to freeze the effect at (between 0.0 and 1.0)
/// * `set_raw_alpha` - If true, bypasses interpolation and sets raw alpha
/// * `fx` - The effect to freeze
///
/// # Returns
///
/// An `Effect` that shows the inner effect frozen at the specified alpha
pub fn freeze_at(
    alpha: f32,
    set_raw_alpha: bool,
    effect: Effect,
) -> Effect {
    FreezeAt::new(alpha, set_raw_alpha, effect).into_effect()
}

/// Remaps an effect's alpha progression to operate within a smaller range.
///
/// This is useful for:
/// - Creating partially completed effects
/// - Skipping uninteresting parts of a transition
/// - Focusing on the most visually appealing portion of an effect
/// - Creating sequential effects that join seamlessly at specific transition points
///
/// # Arguments
///
/// * `alpha_start` - The lower bound of the alpha range (0.0-1.0). Values less than 0.0 are clamped to 0.0.
/// * `alpha_end` - The upper bound of the alpha range (0.0-1.0). Values greater than 1.0 are clamped to 1.0.
/// * `effect` - The effect to remap.
///
/// # Returns
///
/// A new effect that remaps the original effect's alpha progression to the specified range.
pub fn remap_alpha(
    alpha_start: f32,
    alpha_end: f32,
    effect: Effect,
) -> Effect {
    let range = alpha_start.max(0.0)..alpha_end.min(1.0);
    RemapAlpha::new(range, effect).into_effect()
}

/// Repeat the effect indefinitely or for a specified number of times or duration.
///
/// # Arguments
/// * `effect` - The effect to repeat
/// * `mode` - Controls how the effect repeats:
///   - `RepeatMode::Forever` - Repeats indefinitely
///   - `RepeatMode::Times(n)` - Repeats n times
///   - `RepeatMode::Duration(d)` - Repeats for duration d
///
/// # Examples
/// ```no_run
/// use tachyonfx::{fx, fx::RepeatMode, Duration, EffectTimer, Interpolation};
/// use ratatui::style::Color;
///
/// // Repeat a fade effect 3 times
/// let fade = fx::fade_to_fg(Color::Red, EffectTimer::from_ms(1000, Interpolation::Linear));
/// let repeated = fx::repeat(fade, RepeatMode::Times(3));
///
/// // Repeat an effect for 5 seconds
/// let fade = fx::fade_to_fg(Color::Red, EffectTimer::from_ms(1000, Interpolation::Linear));
/// let repeat_duration = fx::repeat(fade, RepeatMode::Duration(Duration::from_secs(5)));
/// ```
pub fn repeat(effect: Effect, mode: RepeatMode) -> Effect {
    Repeat::new(effect, mode).into_effect()
}

/// Plays the effect forwards and then backwards, creating a ping-pong animation effect.
///
/// This is useful for creating oscillating animations where an effect needs to smoothly
/// reverse back to its starting state. The total duration will be twice the original
/// effect's duration.
///
/// # Arguments
/// * `effect` - The effect to play forwards and backwards
///
/// # Examples
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/ping_pong.gif)
///
/// ```no_run
/// use tachyonfx::{fx, Interpolation};
///
/// let timer = (500, Interpolation::CircOut);
/// fx::ping_pong(fx::coalesce(timer));
/// ```
pub fn ping_pong(effect: Effect) -> Effect {
    PingPong::new(effect).into_effect()
}

/// Repeat the effect indefinitely.
///
/// This is a convenience wrapper around `repeat(effect, RepeatMode::Forever)`.
///
/// # Arguments
/// * `effect` - The effect to repeat indefinitely
///
/// # Examples
/// ```no_run
/// use tachyonfx::{fx, EffectTimer, Interpolation};
/// use ratatui::style::Color;
///
/// // Create an endless color cycling effect
/// let fade = fx::fade_to_fg(Color::Red, EffectTimer::from_ms(1000, Interpolation::Linear));
/// let endless = fx::repeating(fade);
/// ```
pub fn repeating(effect: Effect) -> Effect {
    repeat(effect, RepeatMode::Forever)
}

/// Creates an effect that sweeps out from a specified color with optional randomness.
///
/// Refer to [`sweep_in`](fn.sweep_in.html) for more information.
pub fn sweep_out<T: Into<EffectTimer>, C: Into<Color>>(
    direction: Motion,
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
///   - `Motion::LeftToRight`
///   - `Motion::RightToLeft`
///   - `Motion::UpToDown`
///   - `Motion::DownToUp`
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
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/sweep_in.gif)
///
/// ```no_run
/// use ratatui::prelude::Color;
/// use tachyonfx::*;
/// let c = Color::from_u32(0x1d2021);
/// let timer = (1000, Interpolation::Linear);
/// fx::sweep_in(Motion::LeftToRight, 10, 0, c, timer);
/// ```
///
/// Sweep in from the left with a gradient length of 10 and no randomness.
///
/// Basic usage:
/// ```
/// use tachyonfx::{fx, EffectTimer, Interpolation, Motion};
/// use ratatui::style::Color;
///
/// let sweep_effect = fx::sweep_in(
///     Motion::LeftToRight,
///     10,
///     0,
///     Color::Blue,
///     EffectTimer::from_ms(1000, Interpolation::Linear)
/// );
/// ```
///
/// With randomness:
/// ```
/// use tachyonfx::{fx, EffectTimer, Interpolation, Motion};
/// use ratatui::style::Color;
///
/// let sweep_effect = fx::sweep_in(
///     Motion::UpToDown,
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
    direction: Motion,
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
///
/// # Examples
///
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/slide_in.gif)
///
/// ```no_run
/// use ratatui::prelude::Color;
/// use tachyonfx::*;
///
/// let c = Color::from_u32(0x1d2021);
/// let timer = (1000, Interpolation::Linear);
/// fx::slide_in(Motion::UpToDown, 10, 0, c, timer);
/// ```
/// Slides in from the top, with no randomness
pub fn slide_in<T: Into<EffectTimer>, C: Into<Color>>(
    direction: Motion,
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
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/slide_out.gif)
///
/// ```no_run
/// use ratatui::prelude::Color;
/// use tachyonfx::*;
///
/// // slide in from the top, with no randomness
/// let c = Color::from_u32(0x1d2021);
/// let timer = (1000, Interpolation::Linear);
/// fx::slide_in(Motion::UpToDown, 10, 0, c, timer);
/// ```
pub fn slide_out<T: Into<EffectTimer>, C: Into<Color>>(
    direction: Motion,
    gradient_length: u16,
    randomness: u16,
    color_behind_cells: C,
    timer: T,
) -> Effect {
    let timer: EffectTimer = timer.into();
    let timer = match direction {
        Motion::LeftToRight => timer,
        Motion::RightToLeft => timer.reversed(),
        Motion::UpToDown    => timer,
        Motion::DownToUp    => timer.reversed(),
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
///
/// # Examples
///
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/sequence.gif)
///
/// ```no_run
/// use ratatui::prelude::Color;
/// use tachyonfx::*;
///
/// // fade in the entire area from the out-of-bounds color
/// let c = Color::from_u32(0x504945);
/// let timer = (500, Interpolation::CircOut);
/// fx::sequence(&[
///     fx::fade_from_fg(c, timer),
///     fx::dissolve(timer),
/// ]);
/// ```
pub fn sequence(effects: &[Effect]) -> Effect {
    SequentialEffect::new(effects.into()).into_effect()
}

/// Runs the effects in parallel, all at the same time. Reports completion
/// once all effects have completed.
///
/// # Examples
///
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/parallel.gif)
///
/// ```no_run
/// use ratatui::prelude::Color;
/// use tachyonfx::*;
/// 
/// let c = Color::from_u32(0x504945);
/// let timer = (1000, Interpolation::CircOut);
/// fx::parallel(&[
///     fx::fade_from_fg(c, timer),
///     fx::coalesce(timer),
/// ]);
/// ```
/// Fade in the entire area from the out-of-bounds color.
pub fn parallel(effects: &[Effect]) -> Effect {
    ParallelEffect::new(effects.into()).into_effect()
}

/// Dissolves the current text into the new text over the specified duration. The
/// `cycle_len` parameter specifies the number of cell states are tracked before
/// it cycles and repeats.
///
/// # Examples
///
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/dissolve.gif)
///
/// ```no_run
/// use tachyonfx::{fx, Interpolation};
///
/// fx::dissolve(1000); // linear interpolation
/// ```
pub fn dissolve<T: Into<EffectTimer>>(timer: T) -> Effect {
    Dissolve::new(timer.into())
        .into_effect()
}

/// Dissolves both the text and background to the specified style over the specified duration.
///
/// This is similar to [`dissolve()`] but also transitions the background to match the target style.
///
/// # Arguments
/// * `timer` - Controls the duration and interpolation of the effect
/// * `style` - The target style to dissolve to
pub fn dissolve_to<T: Into<EffectTimer>>(style: Style, timer: T) -> Effect {
    Dissolve::with_style(style, timer.into())
        .into_effect()
}

/// The reverse of [dissolve()].
///
/// # Examples
///
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/coalesce.gif)
///
/// ```no_run
/// use tachyonfx::{fx, Interpolation};
///
/// fx::coalesce((1000, Interpolation::BounceOut));
/// ```
pub fn coalesce<T: Into<EffectTimer>>(timer: T) -> Effect {
    Dissolve::new(timer.into().reversed())
        .into_effect()
}

/// Reforms both the text and background to the specified style over the specified duration.
/// The reverse of [dissolve_to()].
///
/// This is similar to [`coalesce`] but also transitions the background to match the target style.
///
/// # Arguments
/// * `timer` - Controls the duration and interpolation of the effect
/// * `style` - The target style to dissolve to
///
/// /// # Examples
///
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/coalesce_from.gif)
///
/// ```no_run
/// use ratatui::prelude::{Color, Style};
/// use tachyonfx::*;
///
/// let c = Color::from_u32(0x1d2021);
/// let style = Style::default().bg(c);
/// fx::coalesce_from(style, (1000, Interpolation::ExpoInOut));
/// ```
pub fn coalesce_from<T: Into<EffectTimer>>(style: Style, timer: T) -> Effect {
    Dissolve::with_style(style, timer.into().reversed())
        .into_effect()
}

/// Fades the foreground color to the specified color over the specified duration.
///
/// # Examples
///
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/fade_to_fg.gif)
///
/// ```no_run
/// use ratatui::prelude::Color;
/// use tachyonfx::*;
///
/// let c = Color::from_u32(0x504945);
/// let filter = CellFilter::FgColor(Color::from_u32(0xfabd2f));
/// fx::fade_to_fg(c, (1000, Interpolation::CircOut))
///     .filter(filter);
/// ```
///
/// Fade out blake by targeting the author fg color.
pub fn fade_to_fg<T: Into<EffectTimer>, C: Into<Color>>(
    fg: C,
    timer: T,
) -> Effect {
    fade(Some(fg), None, timer.into(), false)
}

/// Fades the foreground color from the specified color over the specified duration.
///
/// # Examples
///
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/fade_from_fg.gif)
///
/// ```no_run
/// use ratatui::prelude::{Color, Margin};
/// use tachyonfx::*;
///
/// let c = Color::from_u32(0x504945);
/// let filter = CellFilter::Inner(Margin::new(1, 1));
/// fx::fade_from_fg(c, (1000, Interpolation::QuadInOut))
///     .filter(filter);
/// ```
/// Fade in content, excluding borders, from the bg color.
pub fn fade_from_fg<T: Into<EffectTimer>, C: Into<Color>>(
    fg: C,
    timer: T,
) -> Effect {
    fade(Some(fg), None, timer.into(), true)
}

/// Fades to the specified the background and foreground colors over the specified duration.
///
/// ## Example
///
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/fade_to.gif)
///
/// ```no_run
/// use ratatui::prelude::Color;
/// use tachyonfx::*;
///
/// let c = Color::from_u32(0x1d2021);
/// fx::fade_to(c, c, (1000, Interpolation::CircOut));
/// ```
///
/// Fade the entire area to the out-of-bounds color.
pub fn fade_to<T: Into<EffectTimer>, C: Into<Color>>(
    fg: C,
    bg: C,
    timer: T,
) -> Effect {
    fade(Some(fg), Some(bg), timer.into(), false)
}

/// Fades from the specified the background and foreground colors over the specified duration.
///
/// # Examples
///
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/fade_from.gif)
///
/// ```no_run
/// use ratatui::prelude::Color;
/// use tachyonfx::*;
/// let c = Color::from_u32(0x1d2021);
/// fx::fade_from(c, c, (1000, Interpolation::CircOut));
/// ```
///
/// fade in the entire area from the out-of-bounds color
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
/// # Example
///
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/delay.gif)
///
/// ```no_run
/// use tachyonfx::fx;
///
/// // wait 800ms before dissolving the content
/// fx::delay(800, fx::dissolve(200));
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
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/prolong_start.gif)
///
/// ```no_run
/// use ratatui::prelude::Color;
/// use tachyonfx::*;
///
/// let c = Color::from_u32(0x504945);
/// let timer = (500, Interpolation::CircOut);
/// fx::prolong_start(timer, fx::fade_from_fg(c, timer));
/// ```
///  This example holds the initial state of the fade effect for 500ms before starting the fade.
///
/// ```
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
/// Example from `examples/effect-showcase.rs`
///
/// ![animation](https://raw.githubusercontent.com/junkdog/tachyonfx/development/docs/assets/prolong_start.gif)
///
/// ```no_run
/// use ratatui::prelude::Color;
/// use tachyonfx::*;
///
/// let c = Color::from_u32(0x504945);
/// let timer = (500, Interpolation::CircOut);
/// fx::prolong_end(timer, fx::fade_to_fg(c, timer));
/// ```
/// This example holds the final state of the fade effect for another 500ms after it completes.
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
/// effectively making it run indefinitely.
///
/// Once the wrapped effect reaches its end state, it will:
/// - Continue processing without advancing its internal timer
/// - Maintain its final visual state
/// - Never report completion
/// - Continue consuming processing ticks
///
/// This is useful for:
/// - Creating persistent visual states
/// - Preventing effect chains from advancing
/// - Maintaining effects that need to run indefinitely
///
/// # Arguments
/// * `effect` - The effect to run indefinitely
///
/// # Examples
/// ```no_run
/// use tachyonfx::{fx, EffectTimer, Interpolation};
/// use ratatui::style::Color;
///
/// // Create a permanent color change over 1 second
/// let fade = fx::fade_to_fg(Color::Red, EffectTimer::from_ms(1000, Interpolation::Linear));
/// let permanent = fx::never_complete(fade);
/// ```
pub fn never_complete(effect: Effect) -> Effect {
    NeverComplete::new(effect).into_effect()
}

/// Wraps an effect and enforces a maximum duration on it. Once the duration has
/// elapsed or the wrapped effect has finished, the effect will be marked as complete.
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
        .color_space(ColorSpace::default())
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
use crate::fx::alpha_xform::{FreezeAt, RemapAlpha};
use crate::fx::explode::Explode;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Shader;
    use ratatui::prelude::Color;

    const DIRECTIONS: [Motion; 4] = [
        Motion::DownToUp,
        Motion::UpToDown,
        Motion::LeftToRight,
        Motion::RightToLeft,
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
            Motion::DownToUp,
            Motion::UpToDown,
            Motion::LeftToRight,
            Motion::RightToLeft,
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
    #[cfg(target_pointer_width = "64")]
    #[cfg(not(feature = "std-duration"))]
    fn assert_sizes() {
        let verify_size = |actual: usize, expected: usize| {
            assert_eq!(actual, expected);
        };

        use crate::fx::{offscreen_buffer::OffscreenBuffer, translate::Translate};

        verify_size(size_of::<EffectTimer>(),      12);
        verify_size(size_of::<Ansi256>(),          10);
        verify_size(size_of::<ConsumeTick>(),       1);
        verify_size(size_of::<Dissolve>(),         88);
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
