## tachyonfx

[![Crate Badge]][Crate] [![API Badge]][API] [![Deps.rs
Badge]][Deps.rs]

tachyonfx (_ˈtakɪɒn ˌɛfˈɛks_) is a [ratatui][ratatui] library for creating shader-like effects in terminal UIs. It
provides a collection of stateful effects that can enhance the visual appeal of terminal applications through color
transformations, animations, and complex effect combinations.

![demo](images/demo-0.6.0.gif)

[ratatui]: https://ratatui.rs/

## Try it out in your browser
[TachyonFX FTL][tfx-ftl] is a browser-based editor for creating and tweaking effects.

## Installation
Add tachyonfx to your `Cargo.toml`:

```toml
tachyonfx = "0.15.0"
```

## Core Concepts

### Effects and State
Effects in tachyonfx are stateful objects that evolve over time. When you create an effect, it typically maintains:

- An internal timer or flag tracking progress
- Effect-specific state (like transition progress)
- Configuration (like styling, directions, or interpolation methods)

```rust
use tachyonfx::fx::{fade_to_fg, Color};

// Create the effect once
let mut fade_effect = fx::fade_to_fg(Color::Red, Duration::from_millis(1000));

// In your render loop:
loop {
    widget.render(area, buf);

    // Process the same effect each frame, updating its state
    fade_effect.process(frame_duration, buf, area);
    // Or use the helper trait:
    // frame.render_effect(&mut fade_effect, area, frame_duration);
}
```

### Effects and Widgets

Effects in tachyonfx operate on terminal cells after widgets have been rendered to the screen. When an effect is
applied, it modifies properties of the already-rendered cells - like their colors, characters, or visibility. This means
the typical flow is:

1. Render your widget to the screen
2. Apply effects to transform the rendered content

### Effect DSL (Domain-Specific Language)

tachyonfx includes a rust-looking DSL for defining effects as text expressions that can be compiled at runtime.
This enables:

- Fast iteration and prototyping of effects
- Creating effects from configuration files or user input
- Storing and serializing effect definitions

```rust
use tachyonfx::dsl::EffectDsl;

// Create a DSL compiler and bind variables
let effect = EffectDsl::new().compiler()
    .bind("color", Color::Red)
    .compile("fx::fade_to_fg(color, (1000, QuadOut))")
    .expect("valid effect from dsl");

// Complex compositions
let expression = r#"
    fx::sequence(&[
        fx::fade_from(Color::Black, Color::Red, (500, LinearOut)),
        fx::dissolve((300, BounceOut))
    ])
"#;

let effect = EffectDsl::new()
    .compiler()
    .compile(expression)
    .expect("valid effect from dsl");
```

The DSL supports let bindings, [method chaining][docs-supported-types], and serialization of effects
with `Effect::to_dsl`:


 [docs-supported-types]: https://docs.rs/tachyonfx/latest/tachyonfx/dsl/index.html#supported-types-and-methods

### TachyonFX FTL
[TachyonFX FTL][tfx-ftl] is a browser-based editor for creating and tweaking effects. It allows you to visualize
effects in real-time, making it easier to understand how they work and how to use them in your applications.

### Types of Effects

The library includes a variety of effects, loosely categorized as follows:

#### Color Effects
- **fade_from:**      Fades from the specified background and foreground colors
- **fade_from_fg:**   Fades the foreground color from a specified color.
- **fade_to:**        Fades to the specified background and foreground colors.
- **fade_to_fg:**     Fades the foreground color to a specified color.
- **hsl_shift:**      Changes the hue, saturation, and lightness of the foreground and background colors.
- **hsl_shift_fg:**   Shifts the foreground color by the specified hue, saturation, and lightness over the specified duration.
- **term256_colors:** Downsamples to 256 color mode.

#### Text/Character Effects
- **coalesce:**   The reverse of dissolve, coalesces text over the specified duration.
- **dissolve:**   Dissolves the current text over the specified duration.
- **explode:**    Explodes the content dispersing it outward from the center.
- **slide_in:**   Applies a directional sliding in effect to terminal cells.
- **slide_out:**  Applies a directional sliding out effect to terminal cells.
- **sweep_in:**   Sweeps in from the specified color.
- **sweep_out:**  Sweeps out to the specified color.

#### Timing and Control Effects
- **consume_tick:**         Consumes a single tick.
- **freeze_at:**            Freezes another effect at a specific alpha (transition) value.
- **never_complete:**       Makes an effect run indefinitely.
- **ping_pong:**            Plays the effect forwards and then backwards.
- **prolong_start**:        Extends the start of an effect by a specified duration.
- **prolong_end**:          Extends the end of an effect by a specified duration.
- **remap_alpha:**          Remaps an effect's alpha progression to operate within a smaller range.
- **repeat:**               Repeats an effect indefinitely or for a specified number of times or duration.
- **repeating:**            Repeats the effect indefinitely.
- **sleep:**                Pauses for a specified duration.
- **timed_never_complete:** Creates an effect that runs indefinitely but has an enforced duration.
- **with_duration:**        Wraps an effect and enforces a maximum duration on it.

#### Geometry Effects
- **translate:**     Moves the effect area by a specified amount.
- **translate_buf:** Copies the contents from an aux buffer, moving it by a specified amount.
- **resize_area:**   Resizes the area of the wrapped effect.

#### Combination Effects
- **parallel:** Runs effects in parallel, all at the same time. Reports completion once all effects have completed.
- **sequence:** Runs effects in sequence, one after the other. Reports completion once the last effect has completed.

#### Other Effects
- **effect_fn:**        Creates custom effects from user-defined functions, operating over `CellIterator`.
- **effect_fn_buf:**    Creates custom effects from functions, operating over `Buffer`.
- **offscreen_buffer:** Wraps an existing effect and redirects its rendering to a separate buffer.
- **unique:**           A unique effect that will cancel any existing effect with the same key.

Additional effects can be created by implementing the `Shader` trait.


### EffectTimer and Interpolations

Most effects are driven by an `EffectTimer` that controls their duration and interpolation. It
allows for precise timing and synchronization of visual effects within your application.

```rust
fx::dissolve(EffectTimer::from_ms(500, BounceOut))
fx::dissolve((500, BounceOut)) // shorthand for the above
fx::dissolve(500)              // linear interpolation
```

### Cell Selection and Area

Effects can be applied to specific cells in the terminal UI, allowing for targeted visual
modifications and animations.

```rust
// only apply to cells with `Light2` foreground color
fx::sweep_in(Direction::UpToDown, 15, 0, Dark0, timer)
    .with_filter(CellFilter::FgColor(Light2.into()))
```

`CellFilter`s can be combined to form complex selection criteria.

```rust
// apply effect to cells on the outer border of the area
let margin = Margin::new(1, 1);
let border_text = CellFilter::AllOf(&[
    CellFilter::Outer(margin),
    CellFilter::Text
]);

prolong_start(duration, fx::fade_from(Dark0, Dark0, (320, QuadOut)),
    .with_filter(border_text)
```

### Features
- `dsl`: Enables the Effect DSL, allowing for runtime compilation of effect expressions. Enabled by default.
- `sendable`: Enables the `Send` trait for effects, shaders, and associated parameters. This allows effects to be
  safely transferred across thread boundaries. Note that enabling this feature requires all `Shader` implementations
  to be `Send`, which may impose additional constraints on custom shader implementations.
- `std-duration`:  Uses `std::time::Duration` instead of a custom 32-bit duration type.
- `web-time`: Enables WebAssembly compatibility by providing alternative time handling implementations. This allows
  tachyonfx to be used in browser-based WebAssembly applications where `std::time` is not available.


## Examples

### Example: [minimal](examples/minimal.rs)
```
cargo run --release --example=minimal 
```

### Example: [tweens](examples/tweens.rs)
![tweens](images/example-tweens.png)

```
cargo run --release --example=tweens 
```

### Example: [basic-effects](examples/basic-effects.rs)
![basic effeects](images/example-basic-effects.png)
```
cargo run --release --example=basic-effects 
```


### Example: [open-window](examples/open-window.rs)

```
cargo run --release --example=open-window  
```

### Example: [fx-chart](examples/fx-chart.rs)
![fx-chart](images/effect-timeline.gif)

A demo of the `EffectTimelineWidget` showcasing the composition of effects. The widget is a "plain" widget
without any effects as part of its rendering. The effects are instead applied after rendering the widget.

```
cargo run --release --example=fx-chart
```

### Example: [dsl-playground](examples/dsl-playground.rs)
![dsl-playground](images/example-dsl-playground.gif)
```
cargo run --release --example=dsl-playground
```

A playground for experimenting with the DSL to create and combine effects interactively.

[API Badge]: https://docs.rs/tachyonfx/badge.svg
[API]: https://docs.rs/tachyonfx
[Crate Badge]: https://img.shields.io/crates/v/tachyonfx.svg
[Crate]: https://crates.io/crates/tachyonfx
[Deps.rs Badge]: https://deps.rs/repo/github/junkdog/tachyonfx/status.svg
[Deps.rs]: https://deps.rs/repo/github/junkdog/tachyonfx

[tfx-ftl]: https://junkdog.github.io/tachyonfx-ftl/
