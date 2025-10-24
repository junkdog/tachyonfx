use std::collections::HashMap;

use super::types::CallableItem;
use crate::{ctor, method, CellFilter};

pub(super) trait DslType {
    const TYPE_NAME: &'static str;

    // fn contains_constructor(name: &str) -> bool;
    // fn contains_constant(name: &str) -> bool;
    // fn contains_method(name: &str) -> bool;

    fn constants() -> &'static [&'static str];
    fn constructors() -> &'static [CallableItem];
    fn methods() -> &'static [CallableItem];

    fn all_items() -> Vec<CallableItem> {
        [Self::constructors(), Self::methods()].concat()
    }
}

// Marker types for completion - these represent types available in the DSL
pub(super) struct Effect;
pub(super) struct Rect;
pub(super) struct Color;
pub(super) struct Layout;
pub(super) struct Style;
pub(super) struct Constraint;
pub(super) struct Margin;
pub(super) struct RefRect;
pub(super) struct Size;
pub(super) struct Duration;
pub(super) struct EffectTimer;
pub(super) struct RepeatMode;
pub(super) struct CheckerboardPattern;
pub(super) struct CoalescePattern;
pub(super) struct DiagonalPattern;
pub(super) struct DissolvePattern;
pub(super) struct RadialPattern;
pub(super) struct SweepPattern;

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
        &[]
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
        &[]
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

pub fn all_methods() -> HashMap<&'static str, Vec<CallableItem>> {
    HashMap::from([
        // Core types
        (Effect::TYPE_NAME, Effect::all_items()),
        (Rect::TYPE_NAME, Rect::all_items()),
        (Color::TYPE_NAME, Color::all_items()),
        (Layout::TYPE_NAME, Layout::all_items()),
        (Style::TYPE_NAME, Style::all_items()),
        // Filter types
        (CellFilter::TYPE_NAME, CellFilter::all_items()),
        // Layout types
        (Constraint::TYPE_NAME, Constraint::all_items()),
        (Margin::TYPE_NAME, Margin::all_items()),
        (RefRect::TYPE_NAME, RefRect::all_items()),
        (Size::TYPE_NAME, Size::all_items()),
        // Time types
        (Duration::TYPE_NAME, Duration::all_items()),
        (EffectTimer::TYPE_NAME, EffectTimer::all_items()),
        (RepeatMode::TYPE_NAME, RepeatMode::all_items()),
        // Pattern types
        (
            CheckerboardPattern::TYPE_NAME,
            CheckerboardPattern::all_items(),
        ),
        (CoalescePattern::TYPE_NAME, CoalescePattern::all_items()),
        (DiagonalPattern::TYPE_NAME, DiagonalPattern::all_items()),
        (DissolvePattern::TYPE_NAME, DissolvePattern::all_items()),
        (RadialPattern::TYPE_NAME, RadialPattern::all_items()),
        (SweepPattern::TYPE_NAME, SweepPattern::all_items()),
    ])
}
