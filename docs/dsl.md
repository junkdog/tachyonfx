# tachyonfx Effect DSL Documentation

## Overview

The tachyonfx Effect DSL (Domain Specific Language) provides a text-based way to create, combine, and manipulate
terminal effects. It mirrors regular Rust syntax while focusing specifically on effect creation and manipulation.

Valid tachyonfx Effect DSL code is valid Rust code with the appropriate imports. This intentional
design choice makes the DSL immediately familiar and enables flexible development workflows.

## Purpose

The tachyonfx Effect DSL serves several key purposes:

1. **Runtime Configuration**: Define effects in config files that can be loaded, parsed, and applied without
   recompilation
2. **Live Reloading**: Update effects while your application is running
3. **Serialization**: Convert effects to/from string representations for storage or transmission
4. **Rapid Prototyping**: Experiment with different effect combinations through text editing
5. **User Customization**: Allow end-users to define their own effects without modifying your codebase

## Basic Usage

The entry point to using the Effect DSL is the `EffectDsl` struct, which manages a registry of effect compilers:

```rust
use tachyonfx::dsl::EffectDsl;
use ratatui::style::Color;
use tachyonfx::Interpolation;

// Create a new DSL compiler with all standard effects registered
let dsl = EffectDsl::new();

// Compile a simple dissolve effect
let effect = dsl.compiler()
    .compile("fx::dissolve(500)")
    .expect("Valid effect");
```

### DSL Expressions Are Valid Rust Code

Any valid tachyonfx Effect DSL expression is also valid Rust code when the appropriate types are imported:

```rust
// This Rust code:
use tachyonfx::fx;
use tachyonfx::Interpolation;
use ratatui::style::{Color, Style};

let effect = fx::fade_to_fg(Color::Red, (1000, Interpolation::QuadOut));

// Is equivalent to this DSL expression:
// "fx::fade_to_fg(Color::Red, (1000, QuadOut))"
```

### Variable Binding

You can bind variables to use within your DSL expressions:

```rust
use tachyonfx::dsl::EffectDsl;
use tachyonfx::{EffectTimer, Motion, Interpolation};
use ratatui::style::Color;

let dsl = EffectDsl::new();
let effect = dsl.compiler()
    .bind("motion", Motion::LeftToRight)
    .bind("bg_color", Color::Blue)
    .bind("timer", EffectTimer::from_ms(500, Interpolation::SineInOut))
    .compile("fx::sweep_in(motion, 10, 0, bg_color, timer)")
    .expect("Valid effect");
```

### Let Bindings

You can define variables within the DSL expression itself using `let` bindings:

```rust
use tachyonfx::dsl::EffectDsl;

let dsl = EffectDsl::new();
let effect = dsl.compiler().compile(r#"
    // These bindings work just like in Rust
    let color = Color::from_u32(0xff5500);
    let timer = (500, CircOut);
    
    // Use the bound variables in the effect
    fx::fade_to_fg(color, timer)
"#).expect("Valid effect");
```

### Method Chaining

Effects can be configured using method chaining, just like in regular Rust code:

```rust
use tachyonfx::dsl::EffectDsl;

let dsl = EffectDsl::new();
let effect = dsl.compiler().compile(r#"
    fx::dissolve(1000)
        .with_filter(CellFilter::Text)
        .with_area(Rect::new(10, 10, 20, 5))
        .with_color_space(ColorSpace::Hsv)
"#).expect("Valid effect");
```
### Composing Effects

The Effect DSL supports both sequence and parallel composition of effects:

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

```rust
use tachyonfx::dsl::EffectDsl;

let dsl = EffectDsl::new();
let effect = dsl.compiler().compile(r#"
    fx::parallel(&[
        fx::dissolve(300),
        fx::fade_to(Color::Red, Color::Black, (400, BounceOut))
    ])
"#).expect("Valid effect");
```

### Text and Character Effects

#### Dissolve and Coalesce Effects

```rust
use tachyonfx::dsl::EffectDsl;

let dsl = EffectDsl::new();

// Basic dissolve effect - makes text disappear gradually
let dissolve_effect = dsl.compiler().compile(r#"
    fx::dissolve((1000, Linear))
"#).expect("Valid effect");

// Dissolve to specific style - transitions both text and background
let dissolve_to_effect = dsl.compiler().compile(r#"
    fx::dissolve_to(
        Style::default().fg(Color::Red).bg(Color::Black), 
        (1500, QuadOut)
    )
"#).expect("Valid effect");

// Coalesce - reverse of dissolve (text appears gradually)
let coalesce_effect = dsl.compiler().compile(r#"
    fx::coalesce((1000, BounceOut))
"#).expect("Valid effect");

// Coalesce from specific style - reforms text from given appearance
let coalesce_from_effect = dsl.compiler().compile(r#"
    fx::coalesce_from(
        Style::default().fg(Color::DarkGray).bg(Color::Black),
        (1000, ExpoInOut)
    )
"#).expect("Valid effect");
```

#### Explosion Effects

```rust
use tachyonfx::dsl::EffectDsl;

let dsl = EffectDsl::new();

// Basic explosion - content disperses outward from center
let explode_effect = dsl.compiler().compile(r#"
    fx::explode(15.0, 2.0, (1000, Linear))
"#).expect("Valid effect");

// Combined with fade for dramatic effect
let dramatic_explosion = dsl.compiler().compile(r#"
    fx::parallel(&[
        fx::fade_to_fg(Color::from_u32(0x404040), (1000, Linear)),
        fx::explode(20.0, 3.0, (1000, Linear))
    ])
"#).expect("Valid effect");
```

#### Slide and Sweep Effects  

```rust
use tachyonfx::dsl::EffectDsl;

let dsl = EffectDsl::new();

// Slide in from specified direction with gradient
let slide_in_effect = dsl.compiler().compile(r#"
    fx::slide_in(
        Motion::LeftToRight,
        10,  // gradient length
        5,   // randomness
        Color::from_u32(0x1d2021),  // color behind cells
        (1000, Linear)
    )
"#).expect("Valid effect");

// Slide out in specified direction
let slide_out_effect = dsl.compiler().compile(r#"
    fx::slide_out(
        Motion::UpToDown,
        15,  // gradient length  
        0,   // no randomness for uniform effect
        Color::Black,
        (1500, QuadOut)
    )
"#).expect("Valid effect");

// Sweep in - transitions from specified color to original content
let sweep_in_effect = dsl.compiler().compile(r#"
    fx::sweep_in(
        Motion::RightToLeft,
        8,   // gradient length
        3,   // randomness for organic feel
        Color::Blue,  // faded color
        (1200, CubicInOut)
    )
"#).expect("Valid effect");

// Sweep out - transitions from original content to specified color
let sweep_out_effect = dsl.compiler().compile(r#"
    fx::sweep_out(
        Motion::DownToUp,
        12,  // gradient length
        2,   // slight randomness
        Color::from_u32(0x504945),
        (800, BounceOut)
    )
"#).expect("Valid effect");
```

### Stretch Effects

The `stretch` effect creates expanding or contracting animations using block characters, perfect for progress bars and layout transitions:

```rust
use tachyonfx::{dsl::EffectDsl, Motion};
use ratatui::style::{Color, Style};

let dsl = EffectDsl::new();

// Basic stretch from left to right
let stretch_effect = dsl.compiler().compile(r#"
    fx::stretch(
        Motion::LeftToRight, 
        Style::default().fg(Color::White).bg(Color::Blue), 
        (1000, Linear)
    )
"#).expect("Valid effect");

// Stretch upward with custom styling
let upward_stretch = dsl.compiler().compile(r#"
    fx::stretch(
        Motion::DownToUp,
        Style::default().bg(Color::Green),
        (2000, QuadOut)
    )
"#).expect("Valid effect");

// Using variables for reusable stretch configurations
let stretch_with_vars = dsl.compiler()
    .bind("direction", Motion::RightToLeft)
    .bind("style", Style::default().fg(Color::Yellow).bg(Color::DarkGray))
    .compile(r#"
        fx::stretch(direction, style, (1500, CubicOut))
    "#)
    .expect("Valid effect");
```

### Color Effects

#### HSL Color Manipulation

```rust
use tachyonfx::dsl::EffectDsl;

let dsl = EffectDsl::new();

// Shift both foreground and background colors in HSL space
let hsl_shift_effect = dsl.compiler().compile(r#"
    fx::hsl_shift(
        Some([120.0, 25.0, 25.0]),  // foreground HSL shifts: [hue, saturation, lightness]
        Some([-40.0, -50.0, -50.0]), // background HSL shifts
        (1000, Linear)
    )
"#).expect("Valid effect");

// Shift only foreground color
let fg_hsl_shift = dsl.compiler().compile(r#"
    fx::hsl_shift_fg(
        [180.0, 0.0, -20.0],  // shift hue by 180°, reduce lightness by 20%
        (1500, SineInOut)
    )
"#).expect("Valid effect");

// Using variables for reusable color shifts
let color_shift_with_vars = dsl.compiler()
    .bind("hue_shift", 90.0f32)
    .bind("saturation_boost", 30.0f32)
    .compile(r#"
        let fg_shift = [hue_shift, saturation_boost, 0.0];
        fx::hsl_shift_fg(fg_shift, (2000, QuadInOut))
    "#)
    .expect("Valid effect");
```

#### Basic Color Fading

```rust
use tachyonfx::dsl::EffectDsl;

let dsl = EffectDsl::new();

// Fade foreground to specific color
let fade_fg_effect = dsl.compiler().compile(r#"
    fx::fade_to_fg(Color::Red, (1000, Linear))
"#).expect("Valid effect");

// Fade from specific foreground color to original
let fade_from_fg_effect = dsl.compiler().compile(r#"
    fx::fade_from_fg(Color::from_u32(0x504945), (1000, QuadInOut))
"#).expect("Valid effect");

// Fade both foreground and background colors
let fade_both_effect = dsl.compiler().compile(r#"
    fx::fade_to(Color::White, Color::Black, (1500, CircOut))
"#).expect("Valid effect");

// Fade from both colors to original
let fade_from_both_effect = dsl.compiler().compile(r#"
    fx::fade_from(Color::Blue, Color::from_u32(0x000080), (1200, BounceOut))
"#).expect("Valid effect");
```

### Timing and Control Effects

#### Effect Repetition and Loops

```rust
use tachyonfx::dsl::EffectDsl;

let dsl = EffectDsl::new();

// Repeat effect specific number of times
let repeated_fade = dsl.compiler().compile(r#"
    fx::repeat(
        fx::fade_to_fg(Color::Red, (500, Linear)),
        RepeatMode::Times(3)
    )
"#).expect("Valid effect");

// Repeat effect for specific duration (using milliseconds)
let duration_repeat = dsl.compiler().compile(r#"
    fx::repeat(
        fx::fade_to_fg(Color::Blue, (200, Linear)), 
        RepeatMode::Duration(5000)
    )
"#).expect("Valid effect");

// Repeat indefinitely (shorthand for RepeatMode::Forever)
let endless_pulse = dsl.compiler().compile(r#"
    fx::repeating(fx::fade_to_fg(Color::Green, (300, SineInOut)))
"#).expect("Valid effect");

// Ping-pong effect - plays forward then backward
let ping_pong_fade = dsl.compiler().compile(r#"
    fx::ping_pong(fx::fade_to_fg(Color::Yellow, (500, CircOut)))
"#).expect("Valid effect");
```

#### Effect Duration Control

```rust
use tachyonfx::dsl::EffectDsl;

let dsl = EffectDsl::new();

// Never complete - effect runs indefinitely once finished
let permanent_effect = dsl.compiler().compile(r#"
    fx::never_complete(fx::fade_to_fg(Color::Red, (1000, Linear)))
"#).expect("Valid effect");

// Timed never complete - runs indefinitely but with duration limit (using milliseconds)
let timed_permanent = dsl.compiler().compile(r#"
    fx::timed_never_complete(
        10000,
        fx::fade_to_fg(Color::from_u32(0x800080), (1000, Linear))
    )
"#).expect("Valid effect");

// Apply duration limit to any effect (using milliseconds)
let duration_limited = dsl.compiler().compile(r#"
    fx::with_duration(2000, fx::repeating(fx::dissolve(500)))
"#).expect("Valid effect");

// Prolong effect duration at start or end
let prolonged_start = dsl.compiler().compile(r#"
    fx::prolong_start((500, Linear), fx::fade_to_fg(Color::Cyan, (1000, Linear)))
"#).expect("Valid effect");

let prolonged_end = dsl.compiler().compile(r#"
    fx::prolong_end((800, Linear), fx::coalesce((600, BounceOut)))
"#).expect("Valid effect");
```

#### Advanced Effect Control

```rust
use tachyonfx::dsl::EffectDsl;

let dsl = EffectDsl::new();

// Freeze effect at specific alpha (transition point)
let frozen_effect = dsl.compiler().compile(r#"
    fx::freeze_at(0.75, true, fx::fade_to_fg(Color::from_u32(0xFFA500), (1000, Linear)))
"#).expect("Valid effect");

// Remap alpha progression to smaller range
let remapped_effect = dsl.compiler().compile(r#"
    fx::remap_alpha(0.2, 0.8, fx::dissolve((1000, QuadInOut)))
"#).expect("Valid effect");

// Run effect exactly once (useful for zero-duration effects in sequences)
let run_once_effect = dsl.compiler().compile(r#"
    fx::run_once(fx::consume_tick())
"#).expect("Valid effect");

// Consume single tick (minimal delay)
let tick_delay = dsl.compiler().compile(r#"
    fx::consume_tick()
"#).expect("Valid effect");

// Add delay before effect starts
let delayed_effect = dsl.compiler().compile(r#"
    fx::delay((800, Linear), fx::explode(10.0, 1.5, (1200, QuadOut)))
"#).expect("Valid effect");

// Simple sleep/pause effect
let pause_effect = dsl.compiler().compile(r#"
    fx::sleep((1000, Linear))
"#).expect("Valid effect");
```

## Supported Types and Methods

The Effect DSL supports all the types and methods needed to create tachyonfx effects:

### Basic Types

In the Effect DSL, these types work exactly like their Rust counterparts:

```rust
use tachyonfx::Duration;
use ratatui::style::Color;
use tachyonfx::ColorSpace;

// Numbers
let n1 = 42;        // u32
let n2 = -5;        // i32
let n3 = 3.14;      // f32

// Colors
let c1 = Color::Red;
let c2 = Color::from_u32(0xff5500);
let c3 = Color::Rgb(255, 0, 0);
let c4 = Color::Indexed(16);

// Duration
let d1 = Duration::from_millis(500);
let d2 = Duration::from_secs_f32(0.5);

// Strings
let s = "hello world";
```

### Effect-Related Types

### Effect-Related Types

The Effect DSL support all tachyonfx effect-related types:

```rust
use ratatui::style::Color;
use tachyonfx::{Duration, EffectTimer, Interpolation, Interpolation::QuadOut, fx::RepeatMode, Motion, ColorSpace};

// EffectTimer (with shorthand syntax)
let t1 = EffectTimer::from_ms(500, Interpolation::Linear);
let t2 = (500, QuadOut);  // Shorthand for EffectTimer

// Motion
let m = Motion::LeftToRight;  // Also: RightToLeft, UpToDown, DownToUp

// Interpolation - All types are supported
let i1 = Interpolation::Linear;
let i2 = Interpolation::BounceOut;
let i3 = Interpolation::CubicInOut;
// ...and many more

// RepeatMode
let r1 = RepeatMode::Forever;
let r2 = RepeatMode::Times(3);
let r3 = RepeatMode::Duration(Duration::from_millis(1000));

// ColorSpace
let cs1 = ColorSpace::Rgb;   // Linear RGB interpolation (fastest)
let cs2 = ColorSpace::Hsl;   // HSL interpolation (default - balance of performance and quality)
let cs3 = ColorSpace::Hsv;   // HSV interpolation
```

### Layout Types

ratatui layout types work the same in the Effect DSL:

```rust
use ratatui::prelude::{Constraint, Margin, Layout, Rect};

// Rect
let rect = Rect::new(0, 0, 10, 10);
let inner = rect.inner(Margin::new(1, 1));

// Layout
let layout = Layout::horizontal([
    Constraint::Percentage(50),
    Constraint::Percentage(50)
]).spacing(1);

// Margin
let margin = Margin::new(1, 1);
```

### Cell Filters

All CellFilter variants are supported in the Effect DSL:

```rust
use tachyonfx::{CellFilter, Duration};
use ratatui::prelude::{Color, Margin};

// Basic filters
let f1 = CellFilter::Text;
let f2 = CellFilter::All;
let f3 = CellFilter::FgColor(Color::Red);
let f4 = CellFilter::BgColor(Color::Blue);
let f5 = CellFilter::Inner(Margin::new(1, 1));
let f6 = CellFilter::Outer(Margin::new(1, 1));

// Compound filters
let f7 = CellFilter::AllOf(vec![CellFilter::Text, CellFilter::FgColor(Color::Red)]);
let f8 = CellFilter::AnyOf(vec![CellFilter::Text, CellFilter::BgColor(Color::Blue)]);
let f9 = CellFilter::Not(Box::new(CellFilter::Text));
```

### Style and Modifiers

Style and Modifier types are fully supported:

```rust
use ratatui::style::{Style, Color, Modifier};

// Style
let style = Style::new()
    .fg(Color::Red)
    .bg(Color::Blue)
    .add_modifier(Modifier::BOLD);

// Modifiers
let m1 = Modifier::BOLD;
let m2 = Modifier::ITALIC;
```

## Shorthand Syntax in DSL

The Effect DSL provides several conveniences for compact, readable code:

1. **Optional `fx::` Prefix**: All effect functions like `dissolve()`, `fade_to()`, etc. can be used without the `fx::` prefix
2. **Unqualified Enum Variants**: Enum variants like `CellFilter::Text` can be used as just `Text`
3. **Timer Shorthand**: Instead of writing `EffectTimer::from_ms(500, Linear)`, you can use the shorthand `(500, Linear)`

This makes DSL expressions more concise and less verbose, especially for complex combinations of effects.


## Converting Between Code and DSL

You can convert between programmatic effect creation and DSL expressions:

### From Code to DSL

```rust
use tachyonfx::fx;
use ratatui::style::Color;
use tachyonfx::{Effect, Shader};

// Create an effect programmatically
let effect = fx::sequence( & [
    fx::fade_from(Color::Black, Color::Reset, 500),
    fx::dissolve(300)
]);

// Convert it to a DSL expression string
let expression = effect.to_dsl().expect("Valid DSL expression");
let expression_str = expression.to_string();
println!("{}", expression_str);
// Output:
// fx::sequence(&[
//     fx::fade_from(Color::Black, Color::Reset, 500),
//     fx::dissolve(300)
// ])
```

### From DSL to Code

```rust
use tachyonfx::dsl::EffectDsl;

// Parse a DSL expression into an effect
let dsl = EffectDsl::new();
let effect = dsl.compiler()
    .compile("fx::sequence(&[fx::fade_from(Color::Black, Color::Reset, 500), fx::dissolve(300)])")
    .expect("Valid effect");
```

## Extending the Effect DSL

You can extend the Effect DSL with custom effects by registering your own compilers:

```rust
use tachyonfx::dsl::{EffectDsl, Arguments, DslError};
use tachyonfx::{fx, Effect, Shader, ColorSpace};
use ratatui::style::Color;

// Create a custom effect function with color space support
fn color_pulse_effect(color: Color, duration: u32, color_space: ColorSpace) -> Effect {
    fx::sequence(&[
        fx::fade_from_fg(color, duration / 2),
        fx::fade_to_fg(color, duration / 2)
    ]).with_color_space(color_space)
}

let dsl = EffectDsl::new()
    .register("color_pulse", | args: &mut Arguments| {
        // Parse arguments from the DSL expression
        let color = args.color()?;
        let duration = args.read_u32()?;
        let color_space = args.option(Arguments::color_space)?.unwrap_or(ColorSpace::Hsl);

        // Return the custom effect
        Ok(color_pulse_effect(color, duration, color_space))
    });

// Now you can use your custom effect in DSL expressions
let effect = dsl.compiler().compile(r#"
    fx::color_pulse(Color::Blue, 1000, Some(Hsv))
"#).expect("Valid effect");
```

## Implementing `to_dsl` for Custom Effects

To enable DSL serialization of your custom effects, implement the `Shader::to_dsl` method:

```rust,ignore
use tachyonfx::{Shader, Effect, Duration, EffectTimer};
use tachyonfx::dsl::{DslFormat, DslError, EffectExpression, Shader};
use ratatui::style::Color;

#[derive(Debug)]
struct PulseShader {
    color: Color,
    timer: EffectTimer,
    // other fields...
}

impl Shader for PulseShader {
    // Implement other Shader methods...

    fn to_dsl(&self) -> Result<EffectExpression, DslError> {
        // Use DslFormat trait to get the DSL representation of color and duration
        let expr = format!("fx::pulse({}, {})",
            self.color.dsl_format(),
            self.timer.duration().as_millis()
        );

        // Parse the string into an EffectExpression
        EffectExpression::parse(&expr)
    }
}
```

## Complete Example: Building a Complex Animation

Here's a complete example showing how to build a complex animation with the Effect DSL:

```rust
use tachyonfx::{Effect, dsl::EffectDsl, ColorSpace};

let animation_dsl = r#"
    // Define variables for reuse
    let timer = (1000, QuadOut);
    let color = Color::from_u32(0x3366ff);
    let color_space = ColorSpace::Rgb;  

    // Create a parallel sequence of effects
    fx::parallel(&[
        // Fade in text
        fx::fade_from_fg(Color::Black, timer)
            .with_filter(CellFilter::Text)
            .with_color_space(color_space),

        // Add some color shifting 
        fx::hsl_shift_fg([30.0, 0.0, 0.0], (500, SineInOut))
            .with_color_space(color_space),

        // Create a stretch effect from left to right
        fx::stretch(Motion::LeftToRight, Style::default().bg(color), timer),

        // After 1s, fade everything out
        fx::prolong_start(1000, fx::fade_to(Color::Black, Color::Black, timer))
    ])
"#;

// Compile the DSL expression into an effect
let dsl = EffectDsl::new();
let effect = dsl.compiler().compile(animation_dsl).expect("Valid effect");
```

## Effects Not Available in DSL

Some tachyonfx effects are intentionally not available in the DSL due to their complexity or requirements for runtime-specific data:

### Custom Function Effects
- `fx::effect_fn` - Requires Rust closures which cannot be represented in string-based DSL
- `fx::effect_fn_buf` - Same limitation as above

### Geometry and Buffer Effects  
- `fx::translate` - Requires complex offset calculations
- `fx::translate_buf` - Works with `RefCount<Buffer>` which cannot be constructed from DSL
- `fx::resize_area` - Works with `Size` parameters that need runtime calculation
- `fx::offscreen_buffer` - Requires `RefCount<Buffer>` parameter

### Advanced Layout Effects
- `fx::dynamic_area` - Requires `RefRect` shared references
- `fx::dispatch_event` - Requires `mpsc::Sender<T>` channels and generic event types

### Special Composition Effects
Note that `fx::sequence` and `fx::parallel` are available in DSL but use special syntax rather than function calls:

```rust
use tachyonfx::dsl::EffectDsl;

let dsl = EffectDsl::new();

// Use this syntax in DSL for composition:
let sequence_effect = dsl.compiler().compile(r#"
    fx::sequence(&[
        fx::fade_to_fg(Color::Red, 500),
        fx::dissolve(300)
    ])
"#).expect("Valid effect");

let parallel_effect = dsl.compiler().compile(r#"
    fx::parallel(&[
        fx::fade_to_fg(Color::Blue, 1000),
        fx::coalesce(800)
    ])
"#).expect("Valid effect");

// These are not regular function registrations like other effects
```

These limitations ensure the DSL remains simple and focused on effects that can be fully described with basic data types.

## Limitations and Considerations

When working with the Effect DSL, be aware of the following limitations:

- **No Mutable Variables:** The Effect DSL only supports immutable variables.
- **Limited Function Support:** The Effect DSL primarily supports method calls and object construction, not defining
  custom functions internally.
- **No Control Flow:** The Effect DSL does not support if/else, match, or loop constructs.
- **Comments:** Both line comments `//` and block comments `/* */` are supported in the Effect DSL but are not preserved
  when serializing back to DSL.
- **Runtime Dependencies:** Effects requiring runtime-specific data (channels, buffers, closures) are not available.

