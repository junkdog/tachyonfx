use std::collections::HashMap;

use crate::{ctor, dsl::completions::completions::CallableItem, method, CellFilter};

pub(super) trait Completable {
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

impl Completable for CellFilter {
    const TYPE_NAME: &'static str = "CellFilter";

    fn constants() -> &'static [&'static str] {
        &["All", "Text"]
    }

    fn constructors() -> &'static [CallableItem] {
        const T: &str = "CellFilter";

        const CTORS: &[CallableItem] = &[
            // Constructors
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

pub fn all_methods() -> HashMap<&'static str, Vec<CallableItem>> {
    let mut methods = HashMap::new();

    use dsl_fns::*;

    // Core types
    methods.insert("Effect", effect());
    methods.insert("Rect", rect());
    methods.insert("Color", color());
    methods.insert("Layout", layout());
    methods.insert("Style", style());

    // Filter types
    methods.insert(CellFilter::TYPE_NAME, CellFilter::all_items());

    // Layout types
    methods.insert("Constraint", constraint());
    methods.insert("Margin", margin());
    methods.insert("RefRect", ref_rect());
    methods.insert("Size", size());

    // Time types
    methods.insert("Duration", duration());
    methods.insert("EffectTimer", effect_timer());
    methods.insert("RepeatMode", repeat_mode());

    // Pattern types
    methods.insert("CheckerboardPattern", checkerboard_pattern());
    methods.insert("CoalescePattern", coalesce_pattern());
    methods.insert("DiagonalPattern", diagonal_pattern());
    methods.insert("DissolvePattern", dissolve_pattern());
    methods.insert("RadialPattern", radial_pattern());
    methods.insert("SweepPattern", sweep_pattern());

    methods
}

mod dsl_fns {
    use std::collections::HashMap;

    use super::*;
    use crate::{
        ctor,
        dsl::completions::{completable::Completable, CallableItem},
        method,
    };

    pub(super) fn rect() -> Vec<CallableItem> {
        const T: &str = "Rect";
        vec![
            // Constructors
            ctor!(T, "new", "u16", "u16", "u16", "u16"),
            // Methods
            method!(T, "clone"),
            method!(T, "clamp", "Rect"),
            method!(T, "inner", "Margin"),
            method!(T, "intersection", "Rect"),
            method!(T, "union", "Rect"),
            method!(T, "offset", "Offset"),
        ]
    }

    pub(super) fn effect() -> Vec<CallableItem> {
        const T: &str = "Effect";
        vec![
            // Methods
            method!(T, "clone"),
            method!(T, "reversed"),
            method!(T, "with_area", "Rect"),
            method!(T, "with_color_space", "ColorSpace"),
            method!(T, "with_duration", "Duration"),
            method!(T, "with_filter", "CellFilter"),
            method!(T, "filter", "CellFilter"),
            method!(T, "with_pattern", "AnyPattern"),
        ]
    }

    pub(super) fn cell_filter() -> Vec<CallableItem> {
        const T: &str = "CellFilter";
        vec![
            // Constructors
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
            // Methods
            method!(T, "clone"),
            method!(T, "negated"),
            method!(T, "into_static"),
        ]
    }

    pub(super) fn color() -> Vec<CallableItem> {
        const T: &str = "Color";
        vec![
            // Constructors
            ctor!(T, "Rgb", "u8", "u8", "u8"),
            ctor!(T, "from_u32", "u32"),
            ctor!(T, "Indexed", "u8"),
        ]
    }

    pub(super) fn layout() -> Vec<CallableItem> {
        const T: &str = "Layout";
        vec![
            // Constructors
            ctor!(T, "default"),
            ctor!(T, "horizontal", "Vec<Constraint>"),
            ctor!(T, "vertical", "Vec<Constraint>"),
            ctor!(T, "new", "Direction", "Vec<Constraint>"),
            // Methods
            method!(T, "clone"),
            method!(T, "direction", "Direction"),
            method!(T, "flex", "Flex"),
            method!(T, "constraints", "Vec<Constraint>"),
            method!(T, "margin", "u16"),
            method!(T, "horizontal_margin", "u16"),
            method!(T, "vertical_margin", "u16"),
            method!(T, "spacing", "u16"),
        ]
    }

    pub(super) fn style() -> Vec<CallableItem> {
        const T: &str = "Style";
        vec![
            // Constructors
            ctor!(T, "new"),
            ctor!(T, "default"),
            // Methods
            method!(T, "clone"),
            method!(T, "fg", "Color"),
            method!(T, "bg", "Color"),
            method!(T, "add_modifier", "Modifier"),
            method!(T, "remove_modifier", "Modifier"),
        ]
    }

    pub(super) fn constraint() -> Vec<CallableItem> {
        const T: &str = "Constraint";
        vec![
            // Constructors
            ctor!(T, "Min", "u16"),
            ctor!(T, "Max", "u16"),
            ctor!(T, "Length", "u16"),
            ctor!(T, "Percentage", "u16"),
            ctor!(T, "Fill", "u16"),
            ctor!(T, "Ratio", "u32", "u32"),
        ]
    }

    pub(super) fn duration() -> Vec<CallableItem> {
        const T: &str = "Duration";
        vec![
            // Constructors
            ctor!(T, "from_millis", "u64"),
            ctor!(T, "from_secs_f32", "f32"),
        ]
    }

    pub(super) fn effect_timer() -> Vec<CallableItem> {
        const T: &str = "EffectTimer";
        vec![
            // Constructors
            ctor!(T, "from_ms", "u32", "Interpolation"),
            ctor!(T, "new", "Duration", "Interpolation"),
        ]
    }

    pub(super) fn margin() -> Vec<CallableItem> {
        const T: &str = "Margin";
        vec![
            // Constructors
            ctor!(T, "new", "u16", "u16"),
        ]
    }

    pub(super) fn ref_rect() -> Vec<CallableItem> {
        const T: &str = "RefRect";
        vec![
            // Constructors
            ctor!(T, "new", "Rect"),
            ctor!(T, "default"),
        ]
    }

    pub(super) fn size() -> Vec<CallableItem> {
        const T: &str = "Size";
        vec![
            // Constructors
            ctor!(T, "new", "u16", "u16"),
        ]
    }

    pub(super) fn repeat_mode() -> Vec<CallableItem> {
        const T: &str = "RepeatMode";
        vec![
            // Constructors
            ctor!(T, "Times", "u32"),
            ctor!(T, "Duration", "Duration"),
        ]
    }

    pub(super) fn checkerboard_pattern() -> Vec<CallableItem> {
        const T: &str = "CheckerboardPattern";
        vec![
            // Constructors
            ctor!(T, "default"),
            ctor!(T, "with_cell_size", "u16"),
            // Methods
            method!(T, "clone"),
            method!(T, "with_transition_width", "f32"),
        ]
    }

    pub(super) fn coalesce_pattern() -> Vec<CallableItem> {
        const T: &str = "CoalescePattern";
        vec![
            // Constructors
            ctor!(T, "new"),
            ctor!(T, "default"),
            // Methods
            method!(T, "clone"),
        ]
    }

    pub(super) fn diagonal_pattern() -> Vec<CallableItem> {
        const T: &str = "DiagonalPattern";
        vec![
            // Constructors
            ctor!(T, "top_left_to_bottom_right"),
            ctor!(T, "top_right_to_bottom_left"),
            ctor!(T, "bottom_left_to_top_right"),
            ctor!(T, "bottom_right_to_top_left"),
            // Methods
            method!(T, "clone"),
            method!(T, "with_transition_width", "f32"),
        ]
    }

    pub(super) fn dissolve_pattern() -> Vec<CallableItem> {
        const T: &str = "DissolvePattern";
        vec![
            // Constructors
            ctor!(T, "new"),
            ctor!(T, "default"),
            // Methods
            method!(T, "clone"),
        ]
    }

    pub(super) fn radial_pattern() -> Vec<CallableItem> {
        const T: &str = "RadialPattern";
        vec![
            // Constructors
            ctor!(T, "center"),
            ctor!(T, "new", "f32", "f32"),
            ctor!(T, "with_transition", "(f32, f32)", "f32"),
            // Methods
            method!(T, "clone"),
            method!(T, "with_transition_width", "f32"),
            method!(T, "with_center", "f32", "f32"),
        ]
    }

    pub(super) fn sweep_pattern() -> Vec<CallableItem> {
        const T: &str = "SweepPattern";
        vec![
            // Constructors
            ctor!(T, "left_to_right", "u16"),
            ctor!(T, "right_to_left", "u16"),
            ctor!(T, "up_to_down", "u16"),
            ctor!(T, "down_to_up", "u16"),
            // Methods
            method!(T, "clone"),
        ]
    }
}
