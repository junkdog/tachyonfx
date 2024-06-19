## tachyonfx

tachyonfx is a Ratatui library for creating shader-like effects in terminal UIs.
This library provides a collection of effects that can be used to enhance the
visual appeal of terminal applications, offering capabilities such as color
transformations, animations, and complex effect combinations.

## Installation
Add tachyonfx to your `Cargo.toml`:

```toml
[dependencies]
tachyonfx = "0.1.0"
```

## Overview


### Effects

The library includes a variety of effects, such as:

- **consume_tick:** Consumes a single tick.
- **dissolve:** Dissolves the foreground into empty cells over the specified duration.
- **fade_to_fg:** Fades the foreground color to a specified color.
- **hsl_shift:** Changes the hue, saturation, and lightness of the foreground and background colors.
- **never_complete:** Makes an effect run indefinitely.
- **parallel:** Runs effects in parallel.
- **ping_pong:** Plays the effect forwards and then backwards.
- **repeat:** Repeats an effect indefinitely or for a specified number of times or duration.
- **resize_area:** Resizes the area of the wrapped effect.
- **sequence:** Runs effects in sequence.
- **sleep:** Pauses for a specified duration.
- **sweep_in:** Sweeps in from the specified color.
- **sweep_out:** Sweeps out to the specified color.
- **term256_colors:** Downsamples to 256 color mode.
- **translate:** Moves the effect by a specified amount.

### EffectTimer and Interpolations

### Cell Selection and Area

Effects can be applied to specific cells in the terminal UI, allowing for targeted visual
modifications and animations.

```rust
// only apply to cells with `Light2` foreground color
fx::sweep_in(Direction::UpToDown, 15, Dark0, timer)
    .with_cell_selection(CellFilter::FgColor(Light2.into()))
```

CellFilters can be combined to form complex selection criteria.

```rust
// apply effect to cells on the outer border of the area
let margin = Margin::new(1, 1);
let border_text = CellFilter::AllOf(vec![
    CellFilter::Outer(margin),
    CellFilter::Text
]);

sequence(vec![
    with_duration(duration, never_complete(fx::fade_to(Dark0, Dark0, 0))),
    fx::fade_from(Dark0, Dark0, (320, QuadOut)),
]).with_cell_selection(border_text)
```

## Examples

### Example: `tweens`
![tweens](images/example-tweens.png)

```
cargo run --example=tweens 
```

### Example: `basic-effects`
![basic effeects](images/example-basic-effects.png)
```
cargo run --example=basic-effects 
```


### Example: `open-window`

```
cargo run --example=open-window  
```