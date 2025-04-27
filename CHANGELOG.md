# Changelog

## tachyonfx 0.15.0 - 2025-04-27

### Added
- `fx::freeze_at`: freezes another effect at a specific alpha (transition) value.
- `fx::remap_alpha`: rescales an effect's alpha progression to a smaller window.
- trait `IntoTemporaryEffect`: previously not exposed although it was implemented for `Effect`.

### DSL
- the parsers now recognize boolean values.
- `.with_duration()` is now available on effects.
- `fx::freeze_at` added to the DSL.
- `fx::remap_alpha` added to the DSL.

### Changed`
- `fx::explode`: cells "behind" the explosion now have `Color::Black` instead of `Color::Reset` for
  for both foreground and background colors. This change makes it easier to apply later effects
  to the area underneath the explosion.

### Breaking Changes
- Changed how effect filters are applied and stored:
  - Effect filters are now stored as `Option<CellFilter>` instead of `CellFilter` directly
  - `Effect::with_filter` now uses filter propagation to preserve existing filters
  - Filters set on individual effects won't be overwritten during effect composition
  - Custom `Shader` implementations will need to update to store `cell_filter` as `Option<CellFilter>`
  - `default_shader_impl!(@filter)` macro users must update their field type from `CellFilter` to `Option<CellFilter>`


## tachyonfx 0.14.0 - 2025-04-21

### Changed`
- With the introduction of `LruCache::memoize_ref`, values no longer have to implement `Clone`.  

### Effect DSL
- Enhanced DslParseError with improved multi-line error handling and contextual error display.
- Update DslError messages to be more informative and user-friendly.
- Missing brackets are now reported with less misleading error messages.
- Improved error messages for missing semicolons.
- Added error messages for missing commas in DSL expressions.


## tachyonfx 0.13.0 - 2025-03-30

### Added
- `fx::explode`: explodes the content outward from the center.
- `LruCache::memoize_ref`: returns a reference to the cached value instead of cloning it.

### Fixes
- `CellFilter::AnyOf`, `CellFilter::NoneOf`: now correctly filter cells based on the provided filters.

### Improvements
- Optimized CellFilter evaluation, reducing overhead by ~30-50% depending on the filter type.


## tachyonfx 0.12.0 - 2025-03-26

### DSL Improvements

#### Improved Error Handling and Diagnostics

- **Improved Error Reporting**: Added source location tracking for DSL errors, making it easier to identify and fix issues in DSL expressions
  - New `DslParseError` type provides detailed context including:
    - Line and column numbers where errors occur
    - Visual context of the problematic code
    - Underlined error locations in the original source
  - `DslCompiler::compile()` now returns `Result<Effect, DslParseError>` instead of `Result<Effect, DslError>`

- **Source Position Tracking**: Added `ExprSpan` to track source locations throughout the parsing pipeline
  - Each expression node now includes its source position for accurate error reporting
  - Enables pinpointing specific tokens in error messages
  - Note that this feature has room for further improvements in future releases. It will occasionally point to the wrong
    token, depending on where and which category of parser intercepts the failure.

#### Internal Parser Improvements

- **Tokenization Pipeline**: Separated lexical analysis (tokenization) from syntax analysis (parsing).
- **Simplified AST**: Streamlined internal Abstract Syntax Tree representation
- **Source Position Tracking**: Added source position tracking to all AST nodes
- **Expression Promotion**: Converts qualified identifiers like `Motion::LeftToRight` to corresponding literal values.
- **DSL Serialization**: Improved DSL serialization with better formatting and source position tracking

#### Other Changes

- **Improved DSL Writer**: Enhanced DSL serialization with smarter line breaking and indentation.

### Added
- `CellFilter::Area`: filters cells within a specified rectangular area.
- `ColorSpace`: enum with `Rgb`, `Hsl`, and `Hsv` options for controlling color interpolation
  - Eliminates overhead of converting to/from colorsys representation; ~1/3 faster color conversions
- `color_from_hsl()`, `color_from_hsv()`, `color_to_hsl()`, `color_to_hsv()`: utility functions
- Added `Effect::with_color_space` to set the color space used for color interpolation
  - Modified effects to respect or propagate the selected color space
- `LruCache<K, V, N>`: const-capacity LRU cache for storing color conversions etc.

### Breaking Changes
- **Error Handling**: The error type for `DslCompiler::compile` has changed from `DslError` to `DslParseError`.

### Deprecated
- `HslConvertable`: deprecated in favor of new color space utilities
- `ColorMapper`: superseded by `LruCache`.

### Fixed
- `CellFilter::Not` now correctly inverts the behavior for all filter types.

## tachyonfx 0.11.1 - 2025-03-02

### Fixed
- Build now works with `default-features = false`.


## tachyonfx 0.11.0 - 2025-03-02

### Added
- New DSL (Domain Specific Language) for effect creation and composition:
  - String-based, rust-like expression syntax for defining effects
  - Support for variable binding and method chaining
  - Serialization of effects to DSL expressions via `Effect::to_dsl`
  - Support for custom effect registration via `EffectDsl::register`
- New `dsl` feature flag (enabled by default):
  - Adds DSL capabilities to the library
  - Depends on the [`anpa`](https://github.com/habbbe/anpa-rs) crate for parsing
- New example: `dsl-playground` for interactive testing of DSL expressions
- `EffectManager`: New component for managing collections of effects with lifecycle handling
  - Support for regular effects that run until completion
  - Support for unique effects that can be cancelled/replaced by new effects with the same ID
  - Automatic cleanup of completed effects and orphaned contexts
- New `web-time` feature flag for WebAssembly compatibility (thanks [@orhun](https://github.com/orhun/) for the contribution)
  - Adds support for using `web_time` crate instead of `std::time` when targeting WASM

### Changed
- Made crossterm backend optional via feature flags
  - Added `crossterm` feature (enabled by default)
  - Changed ratatui dependency to disable default features

### Breaking Changes
- Renamed HSL color conversion methods to avoid conflicts with Ratatui's "palette" feature:
  - `Color::from_hsl(h, s, l)` → `Color::from_hsl_f32(h, s, l)`
  - `color.to_hsl()` → `color.to_hsl_f32()`
   
  This is a short-term fix to prevent name clashes when using tachyonfx with Ratatui's palette feature enabled.

### Deprecated
- `Shader::set_cell_selection()`: renamed to `Shader::filter()`.
- `Shader::cell_selection()`: renamed to `Shader::cell_filter()`.

### Fixed
- `SimpleRng::gen_usize()`: Fixed panic on 32bit architectures 


## tachyonfx 0.10.1 - 2024-12-08

### Documentation
- Improved code examples with complete imports and explicit color values instead of theme references
- Updated motion-related documentation to use `Motion` enum instead of `Direction`

### Fixed
- `fx::effect_fn`/`fx::effect_fn_buf`: removed `Debug` requirement for state parameter.

### Breaking Changes introduced in 0.10.0
- Added `Debug` requirement to `Shader` trait - any custom shaders must now implement `Debug`


## tachyonfx 0.10.0 - 2024-12-07
### Added
- Implemented `Debug` for all effect types and supporting structs
- `fx::dissolve_to()`: dissolves both the characters and style over the specified duration.
- `fx::coallesce_from()`: reforms both the characters and style over the specified duration.
- Example gifs and better rustdoc for the [fx](https://docs.rs/tachyonfx/latest/tachyonfx/fx/index.html) module.

### Changed/Deprecated
- `Motion` replaces `Direction` to to avoid name clashing with ratatui's `Direction` enum.
  The deprecated `Direction` is a type alias for `Motion`.

### Fixed
- `fx::with_duration`: clarified misleading documentation.


## tachyonfx 0.9.3 - 2024-11-20

### Breaking Changes
- The `Shader` trait now requires the `Debug` trait to be implemented. This means that any
  user-defined effects must also implement `Debug`. 

### Fixed
- sweep and slide effects now honor applied CellFilters.

## tachyonfx 0.9.2 - 2024-11-17

### Fixed
- `Cargo.lock` no longer omitted from the crate package. This was an oversight in previous releases.
- Fixed test build failure when the `std-duration` feature is enabled.

## tachyonfx 0.9.0 - 2024-11-17

### Breaking Changes
#### Shader::execute() Signature Update
**Previous:**
```rust
fn execute(&mut self, alpha: f32, area: Rect, cell_iter: CellIterator)
```
**New:**
```rust
fn execute(&mut self, duration: Duration, area: Rect, buf: &mut Buffer)
```

When implementing the `Shader` trait, you must override one of these methods:

1. `execute()` (automatic timer handling)
    - Effect timer handling is done automatically; use for standard effects that rely on default timer handling
    - Most common implementation choice
2. `process()` (manual timer handling)
    - Use when custom timer handling is needed
    - Gives full control over timing behavior
    - Must report timer overflow via return value

**Important:** The default implementations of both methods are no-ops and cannot be used alone. You must override
at least one of them for a functioning effect.

### Added
- `CellFilter::EvalCell`: filter cells based on a predicate function that takes a `&Cell` as input.
- `blit_buffer_region()`: new function to support copying specific regions from source buffers.
- `render_buffer_region()` method added to `BufferRenderer` trait to enable region-based buffer rendering.

### Changed
- `blit_buffer()`: now omits copying cells where `cell.skip` is true. This behavior 
  also carries over to the `BufferRenderer` trait and `blit_buffer_region()`.

### Fixed
- `std-duration` feature: mismatched types error when building the glitch effect. Thanks 
  to [@Veetaha](https://github.com/Veetaha) for reporting. 

## tachyonfx 0.8.0 - 2024-10-21
This is just a tiny release in order to be compatible with the latest `ratatui` version.

### Added
- new `minimal` example demonstrating how to get started with tachyonfx. Thanks to @orhun for the contribution!

### Changed
- `Color::to_rgb`: updated rgb values of standard terminal colors to be more conformant.

### Breaking
- `ratatui` updated to 0.29.0. This is also the minimum version required for tachyonfx.

### Fixed
- `fx::repeat`: visibility of `RepeatMode` is now public.

## tachyonfx 0.7.0 - 2024-09-22

### Added
- `sendable` feature: Enables the `Send` trait for effects, shaders, and associated parameters. This allows effects to
be safely transferred across thread boundaries. Note that enabling this feature requires all `Shader` implementations
to be `Send`, which may impose additional constraints on custom shader implementations.
- `ref_count()`: wraps a value in an `Rc<RefCell<T>>` or an `Arc<Mutex<T>>` depending on the `sendable` feature.

### Changed
- `SlidingWindowAlpha`: Now uses multiplication instead of division when calculating alpha values for the gradient.
- `EffectTimer::alpha`: removed two redundant divisions.

### Fixed
- `EffectTimer::alpha` now correctly returns 0.0 for reversed timers with zero duration.
- `CellIterator` now uses the intersection of the given area and the buffer's area, preventing panics from
  out-of-bounds access.
- `fx::sweep_in`, `fx::sweep_out`, `fx::slide_in`, `fx::slide_out`: now uses a "safe area" calculated as the
  intersection of the effect area and buffer area, preventing out-of-bounds access.

## tachyonfx 0.6.0 - 2024-09-07

This release introduces a lot of breaking changes in the form of added and removed parameters.
Sorry for any inconvenience this may cause, I'll try to tread more carefully in the future.

### Added
- New "std-duration" feature to opt-in to using `std::time::Duration`, which is the same behavior as before.
- New `tachyon::Duration` type: a 4-byte wrapper around u32 milliseconds. When the "std-duration" feature is enabled,
  it becomes an alias for the 16-byte `std::time::Duration`.

### Changed
- Replaced `rand` crate dependency with a fast `SimpleRng` implementation.
- `render_as_ansi_string()` produces a more compact output by reducing redundant ANSI escape codes.

### Breaking
- `tachyonfx::Duration` is now the default duration type.
- Replace usage of `std::time::Duration` with `tachyonfx::Duration`.
- `fx::sweep_in`, `fx::sweep_out`, `fx::slide_in`, `fx::slide_out`: added `randomness` parameter.
- `fx::dissolve`, `fx::coalesce`: removed `cycle_len` parameter, as cell visibility is recalculated on the fly.
- `fx::sequence`, `fx::parallel`: now parameterized with `&[Effect]` instead of `Vec<Effect>`.

### Deprecated
- `EffectTimeline::from` is deprecated in favor of `EffectTimeline::builder`. 


## tachyonfx 0.5.0 - 2024-08-21

![effect-timeline-widget](images/effect-timeline-widget.png)
The effect timeline widget visualizes the composition of effects. It also supports rendering the
widget as an ansi-escaped string, suitable for saving to a file or straight to `println!()`.

### Added
- `fx::delay()`: delays the start of an effect by a specified duration.
- `fx::offscreen_buffer()`: wraps an existing effect and redirects its rendering
  to a separate buffer.  This allows for more complex effect compositions and can
  improve performance for certain types of effects.
- `fx::prolong_start`: extends the start of an effect by a specified duration.
- `fx::prolong_end`: extends the end of an effect by a specified duration.
- `fx::translate_buf()`: translates the contents of an auxiliary buffer onto the main buffer.
- `widget::EffectTimeline`: a widget for visualizing the composition of effects.
- `EffectTimeline::save_to_file()`: saves the effect timeline to a file.
- `BufferRenderer` trait: enables rendering of one buffer onto another with offset support.
  This allows for more complex composition of UI elements and effects.
- fn `blit_buffer()`: copies the contents of a source buffer onto a destination buffer with a specified offset.
- fn `render_as_ansi_string()`: converts a buffer to a string containing ANSI escape codes for styling.
- new example: `fx-chart`.

### Breaking
- Shader trait now requires `name()`, `timer()` and `as_effect_span()` methods.
- `ratatui` updated to 0.28.0. This is also the minimum version required for tachyonfx.


## tachyonfx 0.4.0 - 2024-07-14

### Added
- `CellFilter::PositionFn`: filter cells based on a predicate function.
- `EffectTimer::durtion()` is now public.
- `fx::slide_in()` and `fx::slide_out()`: slides in/out cells by "shrinking" the cells horizontally or
  vertically along the given area.
- `fx::effect_fn_buf()`: to create custom effects operating on a `Buffer` instead of `CellIterator`.
- `Shader::reset`: reinitializes the shader(*) to its original state. Previously, the approach was to
  clone the shader from a copy of the original instance, occasionally resulting in unintended behavior
  when certain internal states were expected to persist through resets.

*: _Note that "shader" here is used loosely, as no GPU is involved, only terminal cells._

### Breaking
- `fx::resize_area`:  signature updated with `initial_size: Size`, replacing the u16 tuple.

### Fixed
- `fx::translate()`: translate can now move out-of-bounds.
- `fx::translate()`: hosted effects with extended duration no longer end prematurely.
- `fx::effect_fn()`: effect state now correctly resets between iterations when using `fx::repeat()`, `fx::repeating()`
  and `fx::ping_pong()`. 
- `fx::resize_area()`: fixed numerous problems.

## tachyonfx 0.3.0 - 2024-06-30

### Changed
- `fx::effect_fn()`: updated the function signature to include an initial state parameter and `ShaderFnContext`
  context parameter. The custom effect closure now takes three parameters: mutable state, `ShaderFnContext`, and a
  cell iterator.
- `ratatui` updated to 0.27.0. This is also the minimum version required for tachyonfx.

## tachyonfx 0.2.0 - 2024-06-23

### Added
- `fx::effect_fn()`: creates custom effects from user-defined functions.
- Add `CellFilter::AnyOf(filters)` and `CellFilter::NoneOf(filters)` variants.
- Implemented `ToRgbComponents` trait for `Color` to standardize extraction of RGB components.

### Fixed
- `fx::translate()`: replace `todo!()` in cell_selection().
- 16 and 256 color spaces no longer output black when interpolating to a different color.

## tachyonfx 0.1.0 - 2024-06-20

Initial release of the library.
