use std::collections::HashMap;

use ratatui::{layout::Flex, prelude::*};

use super::types::CallableItem;
use crate::{
    dsl::completions::macros::{ctor, method},
    fx::{EvolveSymbolSet, ExpandDirection, RepeatMode},
    pattern::*,
    CellFilter, ColorSpace, Duration, Effect, EffectTimer, Interpolation, Motion, RefRect,
    SimpleRng,
};

pub(super) fn effect_types() -> HashMap<&'static str, CallableItem> {
    macro_rules! effect {
        ($name:literal $(, $param:expr)*) => {
            ($name, ctor!("fx", $name $(, $param)*))
        };
    }

    // This maps each effect name to its function signature as a CallableItem
    // The type "fx" is used as the declaring type for all effects
    const FX_TYPES: [(&str, CallableItem); 41] = [
        effect!("sequence", "&[Effect]"),
        effect!("parallel", "&[Effect]"),
        effect!("term256_colors"),
        effect!("coalesce", "EffectTimer"),
        effect!("coalesce_from", "Style", "EffectTimer"),
        effect!("consume_tick"),
        effect!("delay", "EffectTimer", "Effect"),
        effect!("dissolve", "EffectTimer"),
        effect!("dissolve_to", "Style", "EffectTimer"),
        effect!("evolve", "EvolveSymbolSet", "EffectTimer"),
        effect!("evolve_into", "EvolveSymbolSet", "EffectTimer"),
        effect!("evolve_from", "EvolveSymbolSet", "EffectTimer"),
        effect!("expand", "ExpandDirection", "Style", "EffectTimer"),
        effect!("explode", "f32", "f32", "EffectTimer"),
        effect!("fade_from", "Color", "Color", "EffectTimer"),
        effect!("fade_from_fg", "Color", "EffectTimer"),
        effect!("fade_to", "Color", "Color", "EffectTimer"),
        effect!("fade_to_fg", "Color", "EffectTimer"),
        effect!("freeze_at", "f32", "bool", "Effect"),
        effect!(
            "hsl_shift",
            "Option<[f32; 3>",
            "Option<[f32; 3>",
            "EffectTimer"
        ),
        effect!("hsl_shift_fg", "[f32; 3", "EffectTimer"),
        effect!("never_complete", "Effect"),
        effect!("paint", "Color", "Color", "EffectTimer"),
        effect!("paint_bg", "Color", "EffectTimer"),
        effect!("paint_fg", "Color", "EffectTimer"),
        effect!("ping_pong", "Effect"),
        effect!("prolong_end", "EffectTimer", "Effect"),
        effect!("prolong_start", "EffectTimer", "Effect"),
        effect!("remap_alpha", "f32", "f32", "Effect"),
        effect!("repeat", "Effect", "RepeatMode"),
        effect!("run_once", "Effect"),
        effect!("sleep", "EffectTimer"),
        effect!("repeating", "Effect"),
        effect!("slide_in", "Motion", "u16", "u16", "Color", "EffectTimer"),
        effect!("slide_out", "Motion", "u16", "u16", "Color", "EffectTimer"),
        effect!("stretch", "Motion", "Style", "EffectTimer"),
        effect!("sweep_in", "Motion", "u16", "u16", "Color", "EffectTimer"),
        effect!("sweep_out", "Motion", "u16", "u16", "Color", "EffectTimer"),
        effect!("with_duration", "Duration", "Effect"),
        effect!("timed_never_complete", "Duration", "Effect"),
        effect!("translate", "Effect", "Offset", "EffectTimer"),
    ];

    HashMap::from(FX_TYPES)
}

pub(super) trait DslType {
    const TYPE_NAME: &'static str;

    fn constants() -> &'static [&'static str];
    fn constructors() -> &'static [CallableItem];
    fn methods() -> &'static [CallableItem];
}

// Marker types for completion - these represent types available in the DSL

macro_rules! impl_dsl_type_map {
    ($fn_name:ident, $method:ident, $item_type:ty) => {
        pub(super) fn $fn_name() -> HashMap<&'static str, &'static [$item_type]> {
            HashMap::from([
                // Core types
                (Effect::TYPE_NAME, Effect::$method()),
                (Rect::TYPE_NAME, Rect::$method()),
                (Color::TYPE_NAME, Color::$method()),
                (Layout::TYPE_NAME, Layout::$method()),
                (Style::TYPE_NAME, Style::$method()),
                // Filter types
                (CellFilter::TYPE_NAME, CellFilter::$method()),
                // Layout types
                (Constraint::TYPE_NAME, Constraint::$method()),
                (Margin::TYPE_NAME, Margin::$method()),
                (RefRect::TYPE_NAME, RefRect::$method()),
                (Size::TYPE_NAME, Size::$method()),
                // Time types
                (Duration::TYPE_NAME, Duration::$method()),
                (EffectTimer::TYPE_NAME, EffectTimer::$method()),
                (RepeatMode::TYPE_NAME, RepeatMode::$method()),
                // Random types
                (SimpleRng::TYPE_NAME, SimpleRng::$method()),
                // Pattern types
                (
                    CheckerboardPattern::TYPE_NAME,
                    CheckerboardPattern::$method(),
                ),
                (CoalescePattern::TYPE_NAME, CoalescePattern::$method()),
                (DiagonalPattern::TYPE_NAME, DiagonalPattern::$method()),
                (DissolvePattern::TYPE_NAME, DissolvePattern::$method()),
                (RadialPattern::TYPE_NAME, RadialPattern::$method()),
                (SweepPattern::TYPE_NAME, SweepPattern::$method()),
                // Enum types
                (Motion::TYPE_NAME, Motion::$method()),
                (ColorSpace::TYPE_NAME, ColorSpace::$method()),
                (Direction::TYPE_NAME, Direction::$method()),
                (Flex::TYPE_NAME, Flex::$method()),
                (ExpandDirection::TYPE_NAME, ExpandDirection::$method()),
                (Modifier::TYPE_NAME, Modifier::$method()),
                (EvolveSymbolSet::TYPE_NAME, EvolveSymbolSet::$method()),
                (Interpolation::TYPE_NAME, Interpolation::$method()),
            ])
        }
    };
}

impl_dsl_type_map!(all_methods, methods, CallableItem);
impl_dsl_type_map!(all_constructors, constructors, CallableItem);
impl_dsl_type_map!(all_constants, constants, &'static str);

impl DslType for CellFilter {
    const TYPE_NAME: &'static str = "CellFilter";

    fn constants() -> &'static [&'static str] {
        &["All", "Text"]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "CellFilter";

        const CTORS: &[CallableItem] = &[
            ctor!(T, "Area", "Rect"),
            ctor!(T, "RefArea", "RefRect"),
            ctor!(T, "FgColor", "Color"),
            ctor!(T, "BgColor", "Color"),
            ctor!(T, "Inner", "Margin"),
            ctor!(T, "Outer", "Margin"),
            ctor!(T, "AllOf", "Vec<CellFilter>"),
            ctor!(T, "AnyOf", "Vec<CellFilter>"),
            ctor!(T, "NoneOf", "Vec<CellFilter>"),
            ctor!(T, "Not", "Box<CellFilter>"),
            ctor!(T, "Static", "Box<CellFilter>"),
            ctor!(T, "Layout", "Layout", "u16"),
            ctor!(T, "PositionFn", "var"),
            ctor!(T, "EvalCell", "var"),
        ];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        const T: &str = "CellFilter";

        const MEMBERS: &[CallableItem] =
            &[method!(T, "clone"), method!(T, "negated"), method!(T, "into_static")];

        MEMBERS
    }
}

impl DslType for Effect {
    const TYPE_NAME: &'static str = "Effect";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        &[]
    }

    fn methods() -> &'static [CallableItem] {
        const T: &str = "Effect";

        const METHODS: &[CallableItem] = &[
            method!(T, "clone"),
            method!(T, "reversed"),
            method!(T, "with_area", "Rect"),
            method!(T, "with_color_space", "ColorSpace"),
            method!(T, "with_duration", "Duration"),
            method!(T, "with_filter", "CellFilter"),
            method!(T, "filter", "CellFilter"),
            method!(T, "with_pattern", "AnyPattern"),
        ];

        METHODS
    }
}

impl DslType for Rect {
    const TYPE_NAME: &'static str = "Rect";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "Rect";
        const CTORS: &[CallableItem] = &[ctor!(T, "new", "u16", "u16", "u16", "u16")];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        const T: &str = "Rect";
        const METHODS: &[CallableItem] = &[
            method!(T, "clone"),
            method!(T, "clamp", "Rect"),
            method!(T, "inner", "Margin"),
            method!(T, "intersection", "Rect"),
            method!(T, "union", "Rect"),
            method!(T, "offset", "Offset"),
        ];

        METHODS
    }
}

impl DslType for Color {
    const TYPE_NAME: &'static str = "Color";

    fn constants() -> &'static [&'static str] {
        &[
            "Reset",
            "Black",
            "Red",
            "Green",
            "Yellow",
            "Blue",
            "Magenta",
            "Cyan",
            "Gray",
            "DarkGray",
            "LightRed",
            "LightGreen",
            "LightYellow",
            "LightBlue",
            "LightMagenta",
            "LightCyan",
            "White",
        ]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "Color";
        const CTORS: &[CallableItem] = &[
            ctor!(T, "Rgb", "u8", "u8", "u8"),
            ctor!(T, "from_u32", "u32"),
            ctor!(T, "Indexed", "u8"),
        ];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for Layout {
    const TYPE_NAME: &'static str = "Layout";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "Layout";
        const CTORS: &[CallableItem] = &[
            ctor!(T, "default"),
            ctor!(T, "horizontal", "Vec<Constraint>"),
            ctor!(T, "vertical", "Vec<Constraint>"),
            ctor!(T, "new", "Direction", "Vec<Constraint>"),
        ];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        const T: &str = "Layout";
        const METHODS: &[CallableItem] = &[
            method!(T, "clone"),
            method!(T, "direction", "Direction"),
            method!(T, "flex", "Flex"),
            method!(T, "constraints", "Vec<Constraint>"),
            method!(T, "margin", "u16"),
            method!(T, "horizontal_margin", "u16"),
            method!(T, "vertical_margin", "u16"),
            method!(T, "spacing", "u16"),
        ];

        METHODS
    }
}

impl DslType for Style {
    const TYPE_NAME: &'static str = "Style";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "Style";
        const CTORS: &[CallableItem] = &[ctor!(T, "new"), ctor!(T, "default")];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        const T: &str = "Style";
        const METHODS: &[CallableItem] = &[
            method!(T, "clone"),
            method!(T, "fg", "Color"),
            method!(T, "bg", "Color"),
            method!(T, "add_modifier", "Modifier"),
            method!(T, "remove_modifier", "Modifier"),
        ];

        METHODS
    }
}

impl DslType for Constraint {
    const TYPE_NAME: &'static str = "Constraint";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "Constraint";
        const CTORS: &[CallableItem] = &[
            ctor!(T, "Min", "u16"),
            ctor!(T, "Max", "u16"),
            ctor!(T, "Length", "u16"),
            ctor!(T, "Percentage", "u16"),
            ctor!(T, "Fill", "u16"),
            ctor!(T, "Ratio", "u32", "u32"),
        ];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for Duration {
    const TYPE_NAME: &'static str = "Duration";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "Duration";
        const CTORS: &[CallableItem] =
            &[ctor!(T, "from_millis", "u64"), ctor!(T, "from_secs_f32", "f32")];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for EffectTimer {
    const TYPE_NAME: &'static str = "EffectTimer";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "EffectTimer";
        const CTORS: &[CallableItem] = &[
            ctor!(T, "from_ms", "u32", "Interpolation"),
            ctor!(T, "new", "Duration", "Interpolation"),
        ];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for Margin {
    const TYPE_NAME: &'static str = "Margin";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "Margin";

        const CTORS: &[CallableItem] = &[ctor!(T, "new", "u16", "u16")];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for RefRect {
    const TYPE_NAME: &'static str = "RefRect";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "RefRect";
        const CTORS: &[CallableItem] = &[ctor!(T, "new", "Rect"), ctor!(T, "default")];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for Size {
    const TYPE_NAME: &'static str = "Size";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "Size";
        const CTORS: &[CallableItem] = &[ctor!(T, "new", "u16", "u16")];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for RepeatMode {
    const TYPE_NAME: &'static str = "RepeatMode";

    fn constants() -> &'static [&'static str] {
        &["Forever"]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "RepeatMode";
        const CTORS: &[CallableItem] =
            &[ctor!(T, "Times", "u32"), ctor!(T, "Duration", "Duration")];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for SimpleRng {
    const TYPE_NAME: &'static str = "SimpleRng";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "SimpleRng";
        const CTORS: &[CallableItem] = &[ctor!(T, "new", "u32"), ctor!(T, "default")];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for CheckerboardPattern {
    const TYPE_NAME: &'static str = "CheckerboardPattern";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "CheckerboardPattern";
        const CTORS: &[CallableItem] = &[ctor!(T, "default"), ctor!(T, "with_cell_size", "u16")];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        const T: &str = "CheckerboardPattern";
        const METHODS: &[CallableItem] =
            &[method!(T, "clone"), method!(T, "with_transition_width", "f32")];

        METHODS
    }
}

impl DslType for CoalescePattern {
    const TYPE_NAME: &'static str = "CoalescePattern";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "CoalescePattern";
        const CTORS: &[CallableItem] = &[ctor!(T, "new"), ctor!(T, "default")];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        const T: &str = "CoalescePattern";
        const METHODS: &[CallableItem] = &[method!(T, "clone")];

        METHODS
    }
}

impl DslType for DiagonalPattern {
    const TYPE_NAME: &'static str = "DiagonalPattern";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "DiagonalPattern";
        const CTORS: &[CallableItem] = &[
            ctor!(T, "top_left_to_bottom_right"),
            ctor!(T, "top_right_to_bottom_left"),
            ctor!(T, "bottom_left_to_top_right"),
            ctor!(T, "bottom_right_to_top_left"),
        ];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        const T: &str = "DiagonalPattern";
        const METHODS: &[CallableItem] =
            &[method!(T, "clone"), method!(T, "with_transition_width", "f32")];

        METHODS
    }
}

impl DslType for DissolvePattern {
    const TYPE_NAME: &'static str = "DissolvePattern";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "DissolvePattern";
        const CTORS: &[CallableItem] = &[ctor!(T, "new"), ctor!(T, "default")];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        const T: &str = "DissolvePattern";
        const METHODS: &[CallableItem] = &[method!(T, "clone")];

        METHODS
    }
}

impl DslType for RadialPattern {
    const TYPE_NAME: &'static str = "RadialPattern";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "RadialPattern";
        const CTORS: &[CallableItem] = &[
            ctor!(T, "center"),
            ctor!(T, "new", "f32", "f32"),
            ctor!(T, "with_transition", "(f32, f32)", "f32"),
        ];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        const T: &str = "RadialPattern";
        const METHODS: &[CallableItem] = &[
            method!(T, "clone"),
            method!(T, "with_transition_width", "f32"),
            method!(T, "with_center", "f32", "f32"),
        ];

        METHODS
    }
}

impl DslType for SweepPattern {
    const TYPE_NAME: &'static str = "SweepPattern";

    fn constants() -> &'static [&'static str] {
        &[]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "SweepPattern";
        const CTORS: &[CallableItem] = &[
            ctor!(T, "left_to_right", "u16"),
            ctor!(T, "right_to_left", "u16"),
            ctor!(T, "up_to_down", "u16"),
            ctor!(T, "down_to_up", "u16"),
        ];

        CTORS
    }

    fn methods() -> &'static [CallableItem] {
        const T: &str = "SweepPattern";
        const METHODS: &[CallableItem] = &[method!(T, "clone")];

        METHODS
    }
}

impl DslType for Motion {
    const TYPE_NAME: &'static str = "Motion";

    fn constants() -> &'static [&'static str] {
        &["LeftToRight", "RightToLeft", "UpToDown", "DownToUp"]
    }

    fn constructors() -> &'static [CallableItem] {
        &[]
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for ColorSpace {
    const TYPE_NAME: &'static str = "ColorSpace";

    fn constants() -> &'static [&'static str] {
        &["Rgb", "Hsl", "Hsv"]
    }

    fn constructors() -> &'static [CallableItem] {
        &[]
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for Direction {
    const TYPE_NAME: &'static str = "Direction";

    fn constants() -> &'static [&'static str] {
        &["Horizontal", "Vertical"]
    }

    fn constructors() -> &'static [CallableItem] {
        &[]
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for Flex {
    const TYPE_NAME: &'static str = "Flex";

    fn constants() -> &'static [&'static str] {
        &["Legacy", "Start", "End", "Center", "SpaceBetween", "SpaceAround", "SpaceEvenly"]
    }

    fn constructors() -> &'static [CallableItem] {
        &[]
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for ExpandDirection {
    const TYPE_NAME: &'static str = "ExpandDirection";

    fn constants() -> &'static [&'static str] {
        &["Horizontal", "Vertical"]
    }

    fn constructors() -> &'static [CallableItem] {
        &[]
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for Modifier {
    const TYPE_NAME: &'static str = "Modifier";

    fn constants() -> &'static [&'static str] {
        &[
            "BOLD",
            "DIM",
            "ITALIC",
            "UNDERLINED",
            "SLOW_BLINK",
            "RAPID_BLINK",
            "REVERSED",
            "HIDDEN",
            "CROSSED_OUT",
        ]
    }

    fn constructors() -> &'static [CallableItem] {
        &[]
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for EvolveSymbolSet {
    const TYPE_NAME: &'static str = "EvolveSymbolSet";

    fn constants() -> &'static [&'static str] {
        &[
            "BlocksHorizontal",
            "BlocksVertical",
            "CircleFill",
            "Circles",
            "Quadrants",
            "Shaded",
            "Squares",
        ]
    }

    fn constructors() -> &'static [CallableItem] {
        &[]
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}

impl DslType for Interpolation {
    const TYPE_NAME: &'static str = "Interpolation";

    fn constants() -> &'static [&'static str] {
        &[
            "BackIn",
            "BackOut",
            "BackInOut",
            "BounceIn",
            "BounceOut",
            "BounceInOut",
            "CircIn",
            "CircOut",
            "CircInOut",
            "CubicIn",
            "CubicOut",
            "CubicInOut",
            "ElasticIn",
            "ElasticOut",
            "ElasticInOut",
            "ExpoIn",
            "ExpoOut",
            "ExpoInOut",
            "Linear",
            "QuadIn",
            "QuadOut",
            "QuadInOut",
            "QuartIn",
            "QuartOut",
            "QuartInOut",
            "QuintIn",
            "QuintOut",
            "QuintInOut",
            "Reverse",
            "SineIn",
            "SineOut",
            "SineInOut",
        ]
    }

    fn constructors() -> &'static [CallableItem] {
        &[]
    }

    fn methods() -> &'static [CallableItem] {
        &[]
    }
}
