# tachyonfx DSL documentation

## Overview

The tachyonfx DSL provides a high-level, declarative way to create, combine, and manipulate terminal effects.
It offers a text-based representation that's both human-readable and machine-parseable, enabling rapid effect
prototyping, configuration, and serialization.

## Purpose

The TachyonFX DSL serves two primary purposes:

1. **Rapid Iteration**: The DSL is intentionally designed to look and feel like idiomatic Rust code, providing several key benefits:
  - **Familiar Syntax**: Anyone familiar with Rust can immediately understand and write DSL expressions
  - **Live Reloading**: Effects can be loaded from configuration files and reloaded at runtime without recompiling your application
  - **Interactive Development**: Modify effects in real-time while your application is running
  - **Rapid Prototyping**: Experiment with different parameters, timings, and combinations without code changes
  - **Visual Scripting**: Create a visual editor that generates DSL expressions
  - **User Customization**: Allow end-users to customize effects without modifying source code

2. **Serialization Format**: Store and exchange effect definitions between applications. Effects can be saved to configuration files, sent over network connections, or generated programmatically.

## Using the DSL

The entry point to the DSL is the `EffectDsl` struct, which manages a registry of effect compilers. Each compiler knows how to transform a specific DSL expression into a concrete effect instance.

### Basic Usage

```rust
use tachyonfx::dsl::EffectDsl;
use ratatui::style::Color;

// Create a new DSL compiler with all standard effects registered
let dsl = EffectDsl::new();

// Compile a simple dissolve effect
let dissolve_effect = dsl.compiler()
    .compile("fx::dissolve(500)")
    .expect("Valid effect");

// Compile a more complex fade effect with color
let fade_effect = dsl.compiler()
    .compile("fx::fade_to(Color::Red, (1000, QuadOut))")
    .expect("Valid effect");
```

### Variable Binding

You can bind variables to use within your DSL expressions, making them more dynamic and reusable:

```rust
use tachyonfx::dsl::EffectDsl;
use tachyonfx::Motion;
use ratatui::style::Color;

let dsl = EffectDsl::new();
let effect = dsl.compiler()
    .bind("motion", Motion::LeftToRight)
    .bind("bg_color", Color::Blue)
    .bind("duration", 500)
    .compile("fx::sweep_in(motion, 10, 0, bg_color, duration)")
    .expect("Valid effect");
```

### Let Bindings

You can define variables within the DSL expression itself using `let` bindings:

```rust
let dsl = EffectDsl::new();
let effect = dsl.compiler().compile(r#"
    let color = Color::from_u32(0xff5500);
    let timer = (500, CircOut);
    
    fx::fade_to_fg(color, timer)
"#).expect("Valid effect");
```

### Method Chaining

Effects can be configured using method chaining, similar to the API:

```rust
let dsl = EffectDsl::new();
let effect = dsl.compiler().compile(r#"
    fx::dissolve(1000)
        .with_filter(CellFilter::Text)
        .with_area(Rect::new(10, 10, 20, 5))
"#).expect("Valid effect");
```

### Composing Effects

The DSL supports both sequence and parallel composition of effects:

```rust
let dsl = EffectDsl::new();
let effect = dsl.compiler().compile(r#"
    fx::sequence(&[
        fx::fade_from(Color::Black, Color::Blue, 500),
        fx::sleep(200),
        fx::dissolve(300),
        fx::fade_to(Color::Red, Color::Reset, (400, BounceOut))
    ])
"#).expect("Valid effect");
```

```rust
let dsl = EffectDsl::new();
let effect = dsl.compiler().compile(r#"
    fx::parallel(&[
        fx::fade_to_fg(Color::Red, 500).with_filter(CellFilter::Text),
        fx::fade_to_bg(Color::Blue, 500).with_filter(CellFilter::BgColor(Color::Reset))
    ])
"#).expect("Valid effect");
```

## Supported Types and Methods

The DSL supports a wide range of types and methods that mirror the TachyonFX API:

### Basic Types
- **Numbers**: `u32`, `i32`, `f32` (e.g., `42`, `-5`, `3.14`)
- **Color**: `Color::Red`, `Color::from_u32(0xff5500)`, `Color::Rgb(255, 0, 0)`, `Color::Indexed(16)`
- **Duration**: `Duration::from_millis(500)`, `Duration::from_secs_f32(0.5)`
- **String**: `"hello world"`

### Effect-Related Types
- **EffectTimer**: `EffectTimer::from_ms(500, Interpolation::Linear)`, `(500, QuadOut)` (shorthand)
- **Motion**: `Motion::LeftToRight` and other variants
- **Interpolation**: All interpolation types (e.g., `Interpolation::Linear`, `Interpolation::BounceOut`)
- **RepeatMode**: `RepeatMode::Forever`, `RepeatMode::Times(n)`, `RepeatMode::Duration(d)`

### Layout Types
- **Rect**: `Rect::new(x, y, width, height)` with methods like `.inner()`, `.intersection()`, `.union()`, `.offset()`
- **Margin**: `Margin::new(horizontal, vertical)`
- **Layout**: `Layout::horizontal(constraints)`, `Layout::vertical(constraints)` with methods including `.spacing()`, `.margin()`, `.direction()`
- **Constraint**: `Constraint::Length()`, `Constraint::Percentage()`, `Constraint::Ratio()`, etc.
- **Direction**: `Direction::Horizontal`, `Direction::Vertical`

### Cell Filters
- **CellFilter**: All variants including `Text`, `All`, `FgColor()`, `BgColor()`, `Inner()`, `Outer()`, `Layout()`, and compound filters (`AllOf()`, `AnyOf()`, `NoneOf()`, `Not()`)

### Style
- **Style**: `Style::new()` or `Style::default()` with chaining methods:
    - `.fg(color)`, `.bg(color)`
    - `.add_modifier(modifier)`, `.remove_modifier(modifier)`

### Modifier
- **Modifier**: `Modifier::BOLD`, `Modifier::ITALIC`, etc.

## Registering Custom Effects

You can extend the DSL with custom effects by registering your own compilers:

```rust
use tachyonfx::dsl::{EffectDsl, DslError};
use tachyonfx::{fx, Effect};

// Create a custom effect that combines existing effects
fn my_custom_effect(duration: u32, color: ratatui::style::Color) -> Effect {
    fx::sequence(&[
        fx::fade_from_fg(color, duration / 2),
        fx::dissolve(duration / 2)
    ])
}

// Register the custom effect with the DSL
let dsl = EffectDsl::new()
    .register("my_custom_effect", |args| {
        // Parse arguments from the DSL expression
        let duration = args.read_u32()?;
        let color = args.color()?;
        
        // Return the custom effect
        Ok(my_custom_effect(duration, color))
    });

// Now you can use your custom effect in DSL expressions
let effect = dsl.compiler().compile(r#"
    fx::my_custom_effect(1000, Color::Blue)
"#).expect("Valid effect");
```

## Implementing `Shader::to_dsl`

To enable DSL serialization of your custom effects, implement the `Shader::to_dsl` method. This allows your effects to be converted to DSL expressions:

```rust
use tachyonfx::{Shader, Effect};
use tachyonfx::dsl::{DslError, EffectExpression};
use compact_str::ToCompactString;

struct MyCustomShader {
    color: ratatui::style::Color,
    duration: u32,
    // other fields...
}

impl Shader for MyCustomShader {
    // Implement other Shader methods...
    
    // Implement to_dsl to enable serialization
    fn to_dsl(&self) -> Result<EffectExpression, DslError> {
        // Use the DSL format method for types that support it
        let color_expr = self.color.dsl_format();
        
        // Construct a DSL expression string
        let expr = format!("fx::my_custom_effect({}, {})", 
            self.duration, 
            color_expr
        ).to_compact_string();
        
        // Parse the string into an EffectExpression
        EffectExpression::parse(&expr)
    }
}
```

## Examples

### Example 1: Creating a Simple Animation Sequence

```rust
let dsl = EffectDsl::new();
let effect = dsl.compiler().compile(r#"
    let fade_in = fx::fade_from(Color::Black, Color::Reset, (500, QuadOut));
    let wiggle = fx::ping_pong(fx::hsl_shift_fg([10.0, 0.0, 0.0], (300, SineInOut)));
    let fade_out = fx::fade_to(Color::Reset, Color::Black, (500, QuadIn));
    
    fx::sequence(&[fade_in, wiggle, fade_out])
"#).expect("Valid effect");
```

### Example 2: Complex Layout-Based Effects

```rust
let dsl = EffectDsl::new();
let effect = dsl.compiler().compile(r#"
    let layout = Layout::horizontal([Percentage(33), Percentage(34), Percentage(33)])
        .spacing(1)
        .margin(1);
    
    let left_section = CellFilter::Layout(layout, 0);
    let middle_section = CellFilter::Layout(layout, 1);
    let right_section = CellFilter::Layout(layout, 2);
    
    fx::parallel(&[
        fx::slide_in(Motion::LeftToRight, 5, 0, Color::Reset, (700, BackOut))
            .with_filter(left_section),
        fx::dissolve((500, Linear))
            .with_filter(middle_section),
        fx::slide_in(Motion::RightToLeft, 5, 0, Color::Reset, (700, BackOut))
            .with_filter(right_section)
    ])
"#).expect("Valid effect");
```

### Example 3: Serializing and Deserializing Effects

```rust
use tachyonfx::dsl::{EffectDsl, EffectExpression};
use tachyonfx::Shader;

// Create an effect programmatically
let original_effect = fx::sequence(&[
    fx::fade_from(Color::Black, Color::Reset, 500),
    fx::dissolve(300)
]);

// Convert it to a DSL expression
let expression = original_effect.to_dsl().expect("Valid DSL expression");
let expression_str = expression.to_string();

// Save the expression to a file, config, etc.
// ...

// Later, recreate the effect from the expression
let dsl = EffectDsl::new();
let recreated_effect = dsl.compiler()
    .compile(&expression_str)
    .expect("Valid effect");
```

### Limitations and Considerations

When working with the DSL, be aware of the following limitations:

- **No Mutable Variables:** The DSL does not support the mut keyword or mutable variables. All bindings are immutable,
  following a declarative paradigm rather than an imperative one.
- **Layout Serialization:** While Layout objects can be created and used within the DSL, they cannot be converted back
  to DSL format via the `to_dsl()` method. If your effect includes layout-dependent components, these will need special
  handling when serializing.
- **Function Support:** The DSL primarily supports method calls and object construction. It does not support defining
  custom functions or closures within the DSL itself.
- **Limited Control Flow:** The DSL does not support standard Rust control flow constructs such as if, match, loop, or
  other procedural programming features.
- **No Type Inference:** Unlike full Rust, the DSL requires explicit type information in many cases and cannot infer types
  in the same way the Rust compiler does.
- **Custom Shaders:** When implementing to_dsl() for custom shaders, be aware that some complex shader behaviors might
  not be perfectly representable in the DSL. You may need to approximate or simplify certain aspects of your shader's behavior.
- **Error Handling:** The DSL does not support Rust's error handling mechanisms like Result or ? within expressions. Error
  handling happens at the compilation level.
- **Comments:** While the DSL parser does recognize and skip comments (both line comments `//` and block comments `/* */`),
  they are not preserved when serializing effects back to DSL.
- **Performance:** Parsing and compiling DSL expressions incurs a runtime cost. For performance-critical code paths,
  compile expressions once and reuse the resulting effects.
