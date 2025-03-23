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

    // Create a parallel sequenece of effects
    fx::parallel(&[
        // Fade in text
        fx::fade_from_fg(Color::Black, timer)
            .with_filter(CellFilter::Text)
            .with_color_space(color_space),

        // Add some color shifting 
        fx::hsl_shift_fg([30.0, 0.0, 0.0], (500, SineInOut))
            .with_color_space(color_space),

        // After 1s, fade everything out
        fx::prolong_start(1000, fx::fade_to(Color::Black, Color::Black, timer))
    ])
"#;

// Compile the DSL expression into an effect
let dsl = EffectDsl::new();
let effect = dsl.compiler().compile(animation_dsl).expect("Valid effect");
```

## Limitations and Considerations

When working with the Effect DSL, be aware of the following limitations:

- **No Mutable Variables:** The Effect DSL only supports immutable variables.
- **Limited Function Support:** The Effect DSL primarily supports method calls and object construction, not defining
  custom functions internally.
- **No Control Flow:** The Effect DSL does not support if/else, match, or loop constructs.
- **Comments:** Both line comments `//` and block comments `/* */` are supported in the Effect DSL but are not preserved
  when serializing back to DSL.

