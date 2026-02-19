# tachyonfx Effect DSL Documentation

## Overview

The tachyonfx Effect DSL (Domain Specific Language) provides a text-based way to create, combine, and manipulate
terminal effects. It mirrors regular Rust syntax while focusing specifically on effect creation and manipulation.

**Key principle**: Valid tachyonfx Effect DSL code is valid Rust code with the appropriate imports. This makes the 
DSL immediately familiar and enables flexible development workflows.

Note that the DSL is enabled by the `"dsl"` feature. It is part of the default feature set and depends on `"std"`, as
such it does not work in `no_std` environments.

## Purpose

The tachyonfx Effect DSL serves several use cases:

 - **Runtime Configuration**: Define effects in config files that can be loaded, parsed, and applied without
   recompilation
 - **Live Reloading**: Update effects while your application is running
 - **Serialization**: Convert effects to/from string representations for storage or transmission
 - **[Rapid Prototyping][ftl]**: Experiment with different effect combinations through text editing
 - **User Customization**: Allow end-users to define their own effects without modifying your codebase

 [ftl]: https://junkdog.github.io/tachyonfx-ftl/

## Basic Usage

```rust
use tachyonfx::dsl::EffectDsl;

// Create a new DSL compiler with all standard effects registered
let dsl = EffectDsl::new();

// Compile a simple dissolve effect
let effect = dsl.compiler()
    .compile("fx::dissolve(500)")
    .expect("Valid effect");
```

### Variable Binding

Bind external variables for use in DSL expressions:

```rust
use ratatui::style::Color;
use tachyonfx::{dsl::EffectDsl, Motion};

let effect = EffectDsl::new()
    .compiler()
    .bind("motion", Motion::LeftToRight)
    .bind("color", Color::Blue)
    .compile("fx::sweep_in(motion, 10, 0, color, 500)")
    .expect("Valid effect");
```

Use `let` bindings within expressions:

```rust,ignore
let effect = dsl.compiler().compile(r#"
    let color = Color::from_u32(0xff5500);
    let timer = (500, CircOut);
    fx::fade_to_fg(color, timer)
"#).expect("Valid effect");
```

### Method Chaining

Effects support method chaining for configuration:

```rust,ignore
let effect = dsl.compiler().compile(r#"
    fx::dissolve(1000)
        .with_filter(CellFilter::Text)
        .with_area(Rect::new(10, 10, 20, 5))
        .with_color_space(ColorSpace::Hsv)
"#).expect("Valid effect");
```

### Effect Composition

Combine effects with `fx::sequence()` and `fx::parallel()`:

```rust
use tachyonfx::dsl::EffectDsl;

let dsl = EffectDsl::new();
let effect = dsl.compiler().compile(r#"
    fx::sequence(&[
        fx::dissolve(300),
        fx::fade_to_fg(Color::Red, 500),
        fx::fade_to_fg(Color::Blue, 500)
    ])
"#).expect("Valid effect");
```

## Available Effects

The DSL provides access to all standard tachyonfx effects:

### Text and Character Effects

- **Dissolve/Coalesce**: `fx::dissolve()`, `fx::coalesce()`, `fx::dissolve_to()`, `fx::coalesce_from()`
- **Slide/Sweep**: `fx::slide_in()`, `fx::slide_out()`, `fx::sweep_in()`, `fx::sweep_out()`
- **Explosion**: `fx::explode()`
- **Stretch/Expand**: `fx::stretch()`, `fx::expand()`
- **Evolution**: `fx::evolve()`, `fx::evolve_from()`, `fx::evolve_into()` - character transformations
- **Translation**: `fx::translate()`

### Color Effects

- **Fading**: `fx::fade_to()`, `fx::fade_to_fg()`, `fx::fade_from()`, `fx::fade_from_fg()`
- **Painting**: `fx::paint()`, `fx::paint_fg()`, `fx::paint_bg()`
- **HSL Manipulation**: `fx::hsl_shift()`, `fx::hsl_shift_fg()`
- **Saturation**: `fx::saturate()`, `fx::saturate_fg()`
- **Lightness**: `fx::lighten()`, `fx::lighten_fg()`, `fx::darken()`, `fx::darken_fg()`

### Timing and Control Effects

- **Repetition**: `fx::repeat()`, `fx::repeating()`, `fx::ping_pong()`
- **Duration Control**: `fx::never_complete()`, `fx::timed_never_complete()`, `fx::with_duration()`
- **Delays**: `fx::delay()`, `fx::sleep()`, `fx::consume_tick()`
- **Advanced Control**: `fx::freeze_at()`, `fx::remap_alpha()`, `fx::run_once()`
- **Prolonging**: `fx::prolong_start()`, `fx::prolong_end()`

## Pattern System

Most effects can be manipulated with spatial patterns using `.with_pattern()`:

### Pattern Types

- **RadialPattern**: `RadialPattern::center()`, `RadialPattern::new(x, y)`
- **DiamondPattern**: `DiamondPattern::center()`, `DiamondPattern::new(x, y)` — Manhattan distance-based diamond reveals
- **SpiralPattern**: `SpiralPattern::center()`, `SpiralPattern::new(x, y)` — spiral arm reveals with configurable arm count
- **DiagonalPattern**: `DiagonalPattern::top_left_to_bottom_right()`, etc.
- **CheckerboardPattern**: `CheckerboardPattern::default()`, `CheckerboardPattern::with_cell_size()`
- **SweepPattern**: `SweepPattern::left_to_right()`, `SweepPattern::right_to_left()`, etc.
- **Organic Patterns**: `CoalescePattern::new()`, `DissolvePattern::new()`
- **WavePattern**: `WavePattern::new(wave_layer)` — complex wave interference patterns with FM/AM modulation
- **CombinedPattern**: `CombinedPattern::multiply(a, b)`, `CombinedPattern::max(a, b)`, `CombinedPattern::min(a, b)`,
  `CombinedPattern::average(a, b)` — combine two patterns with binary operations
- **BlendPattern**: `BlendPattern::new(a, b)` — crossfade between two patterns over effect lifetime
- **InvertedPattern**: `InvertedPattern::new(pattern)` — inverts a pattern's output

### Pattern Configuration

Patterns support method chaining:

```rust,ignore
RadialPattern::center()
    .with_transition_width(2.5)
    .with_center(0.3, 0.7)

SpiralPattern::center()
    .with_arms(6)
    .with_transition_width(1.5)
```

### Wave System

`WavePattern` is built from composable wave layers and oscillators:

- **Oscillator**: `Oscillator::sin(kx, ky, kt)`, `Oscillator::cos(kx, ky, kt)`,
  `Oscillator::triangle(kx, ky, kt)`, `Oscillator::sawtooth(kx, ky, kt)`
  - Methods: `.phase(f32)`, `.modulated_by(Modulator)`
- **Modulator**: `Modulator::sin(kx, ky, kt)`, `Modulator::cos(kx, ky, kt)`,
  `Modulator::triangle(kx, ky, kt)`, `Modulator::sawtooth(kx, ky, kt)`
  - Methods: `.phase(f32)`, `.intensity(f32)`, `.on_phase()`, `.on_amplitude()`
- **WaveLayer**: `WaveLayer::new(oscillator)`
  - Methods: `.multiply(oscillator)`, `.average(oscillator)`, `.max(oscillator)`,
    `.amplitude(f32)`, `.power(i32)`, `.abs()`

```rust,ignore
WavePattern::new(
    WaveLayer::new(Oscillator::sin(2.0, 0.0, 1.0))
        .multiply(Oscillator::cos(0.0, 3.0, 0.5).modulated_by(
            Modulator::sin(1.0, 1.0, 0.25).intensity(0.5)
        ))
        .amplitude(0.8)
).with_contrast(2)
```

## Supported Types

The DSL supports all types needed for effect creation:

### Basic Types

- **Numbers**: `u8`, `u16`, `u32`, `i32`, `f32`, `bool`
- **Strings**: `String`
- **Colors**: `Color::Red`, `Color::from_u32(0xff5500)`, `Color::Rgb(255, 0, 0)`, `Color::Indexed(16)`
- **Duration**: `Duration::from_millis(500)`, `Duration::from_secs_f32(0.5)`, or bare `u32` for milliseconds

### Effect System Types

- **EffectTimer**: `EffectTimer::from_ms(500, Linear)`, `EffectTimer::new(duration, interpolation)`, or tuple shorthand
  `(500, Linear)`
- **Motion**: `Motion::LeftToRight`, `Motion::RightToLeft`, `Motion::UpToDown`, `Motion::DownToUp`
- **Interpolation**: All interpolation curves (`Linear`, `QuadOut`, `BounceIn`, `SmoothStep`, `Spring`, etc.)
- **RepeatMode**: `RepeatMode::Forever`, `RepeatMode::Times(3)`, `RepeatMode::Duration(duration)`
- **ColorSpace**: `ColorSpace::Rgb`, `ColorSpace::Hsl`, `ColorSpace::Hsv`
- **EvolveSymbolSet**: `EvolveSymbolSet::BlocksHorizontal`, `EvolveSymbolSet::BlocksVertical`,
  `EvolveSymbolSet::CircleFill`, `EvolveSymbolSet::Circles`, `EvolveSymbolSet::Quadrants`,
  `EvolveSymbolSet::Shaded`, `EvolveSymbolSet::Squares`
- **SimpleRng**: `SimpleRng::new(seed)`, `SimpleRng::default()` — for pattern randomization

### Layout Types

- **Rect**: `Rect::new(x, y, width, height)` or struct syntax `Rect { x: 0, y: 0, width: 10, height: 20 }`
- **Layout**: `Layout::horizontal([...])`, `Layout::vertical([...])`, `Layout::new(direction, constraints)`
- **Constraint**: `Constraint::Min(10)`, `Constraint::Max(100)`, `Constraint::Length(50)`, `Constraint::Percentage(25)`,
  `Constraint::Fill(1)`, `Constraint::Ratio(1, 3)`
- **Flex**: `Flex::Legacy`, `Flex::Start`, `Flex::End`, `Flex::Center`, `Flex::SpaceBetween`, `Flex::SpaceAround`,
  `Flex::SpaceEvenly`
- **Other**: `Margin`, `Offset`, `Size`, `RefRect`, `Direction`

### Cell Filters

Complete `CellFilter` support including:

- **Basic**: `CellFilter::All`, `CellFilter::Text`, `CellFilter::NonEmpty`, `CellFilter::FgColor(color)`,
  `CellFilter::BgColor(color)`
- **Spatial**: `CellFilter::Area(rect)`, `CellFilter::RefArea(ref_rect)`, `CellFilter::Inner(margin)`,
  `CellFilter::Outer(margin)`
- **Compound**: `CellFilter::AllOf([...])`, `CellFilter::AnyOf([...])`, `CellFilter::NoneOf([...])`,
  `CellFilter::Not(box)`
- **Advanced**: `CellFilter::Layout(layout, index)`, `CellFilter::Static(box)`, function-based filters

### Style and Modifiers

- **Style**: `Style::new()`, `Style::default()` with method chaining
- **Modifier**: All modifier variants (`Modifier::BOLD`, `Modifier::ITALIC`, etc.)

### Container Types

- **Arrays/Vectors**: `[1.0, 2.0, 3.0]`, `vec![effect1, effect2]`, `&[constraint1, constraint2]`
- **Options**: `Some(value)`, `None`
- **2-sized Tuples**: `(1000, QuadOut)`, `(0.5, 0.7)`
- **Boxed**: `Box::new(value)`

## Shorthand Syntax

The DSL provides conveniences for readable code:

1. **Optional `fx::` Prefix**: Effect functions can be used without `fx::` prefix
2. **Unqualified Enum Variants**: `CellFilter::Text` can be written as just `Text`
3. **Timer Shorthand**: `(500, Linear)` instead of `EffectTimer::from_ms(500, Linear)`

## Error Handling

DSL compilation returns `DslParseError` on failure, providing detailed location information:

```rust
use tachyonfx::dsl::EffectDsl;

let result = EffectDsl::new().compiler().compile("fx::invalid_effect(500)");

match result {
    Ok(effect) => { /* use effect */ }
    Err(parse_error) => {
        eprintln!("Error at line {}:{}: {}",
            parse_error.start_line(),
            parse_error.start_column(),
            parse_error.source
        );
        eprintln!("{}", parse_error.context()); // Shows code with error highlighted
    }
}
```

`DslParseError` provides:
- **Line/column location**: `start_line()`, `start_column()`, `end_line()`, `end_column()`
- **Error context**: `context()` - surrounding code with error highlighted
- **Error text**: `error_text()` - the specific problematic text
- **Underlying cause**: `source` field contains the detailed `DslError`

## Converting Between Code and DSL

### From Code to DSL

```rust
use tachyonfx::fx;
use ratatui::style::Color;

let effect = fx::sequence(&[
    fx::fade_from(Color::Black, Color::Reset, 500),
    fx::dissolve(300)
]);

// Convert to DSL string
let expression = effect.to_dsl().expect("Valid DSL expression");
println!("{}", expression);
```

### From DSL to Code

```rust,ignore
let effect = dsl.compiler()
    .compile("fx::sequence(&[fx::fade_from(Color::Black, Color::Reset, 500), fx::dissolve(300)])")
    .expect("Valid effect");
```

## Extending the DSL

Register custom effects by providing a compiler function:

```rust
use tachyonfx::dsl::{EffectDsl, Arguments, DslError};
use tachyonfx::{fx, Effect};
use ratatui::style::Color;

let dsl = EffectDsl::new()
    .register("color_pulse", | args: &mut Arguments| {
        let color = args.color()?;
        let duration = args.read_u32()?;

        Ok(fx::sequence(&[
            fx::fade_from_fg(color, duration / 2),
            fx::fade_to_fg(color, duration / 2)
        ]))
    });

// Use the custom effect
let effect = dsl.compiler().compile(r#"
    fx::color_pulse(Color::Blue, 1000)
"#).expect("Valid effect");
```

## Effects Not Available in DSL

Some effects are intentionally excluded due to complexity or runtime requirements:

- **Function-based**: `fx::effect_fn`, `fx::effect_fn_buf` (require closures)
- **Buffer-based**: `fx::translate_buf`, `fx::offscreen_buffer` (require `RefCount<Buffer>`)
- **Geometry**: `fx::resize_area` (deprecated)
- **Advanced**: `fx::dynamic_area`, `fx::dispatch_event` (require shared references or channels)

## Limitations

- **No Mutable Variables**: Only immutable bindings are supported
- **Limited Functions**: Primarily supports method calls and object construction
- **No Control Flow**: No if/else, match, or loop constructs
- **Comments**: Supported but not preserved during serialization
- **Runtime Dependencies**: Effects requiring closures, buffers, or channels are not available

The DSL focuses on declarative effect creation using basic data types, ensuring simplicity and broad compatibility.