use compact_str::{format_compact, CompactString, CompactStringExt, ToCompactString};
use ratatui_core::{
    layout::{Constraint, Direction, Flex, Margin, Offset, Rect},
    style::{Color, Modifier, Style},
};

use crate::{
    fx::{EvolveSymbolSet, RepeatMode},
    pattern::WavePattern,
    wave::{Combinator, ModTarget, Modulator, Oscillator, PostTransform, WaveLayer},
    CellFilter, ColorSpace, Duration, EffectTimer, Interpolation, Motion,
};

/// Formats an `f32` value so it always contains a decimal point, ensuring the
/// DSL parser tokenizes it as a float rather than an integer.
pub(crate) fn fmt_f32(v: f32) -> CompactString {
    let s = format_compact!("{v}");
    if s.contains('.') {
        s
    } else {
        format_compact!("{s}.0")
    }
}

/// A trait for converting types into their DSL (Domain Specific Language) string
/// representation.
///
/// This trait enables types to be formatted as valid tachyonfx DSL expressions that can
/// be parsed back into effect definitions. It's primarily used for serializing effects
/// and their parameters into a textual format that matches the tachyonfx DSL syntax.
pub trait DslFormat {
    /// Converts the type into its DSL string representation.
    ///
    /// # Returns
    ///
    /// A `CompactString` containing the DSL representation of the type, which should be:
    /// - Valid Rust syntax
    /// - Parseable by the tachyonfx DSL parser
    /// - Complete with all necessary type information
    fn dsl_format(&self) -> CompactString;
}

impl DslFormat for Color {
    fn dsl_format(&self) -> CompactString {
        match self {
            Color::Reset => "Color::Reset",
            Color::Black => "Color::Black",
            Color::Red => "Color::Red",
            Color::Green => "Color::Green",
            Color::Yellow => "Color::Yellow",
            Color::Blue => "Color::Blue",
            Color::Magenta => "Color::Magenta",
            Color::Cyan => "Color::Cyan",
            Color::Gray => "Color::Gray",
            Color::DarkGray => "Color::DarkGray",
            Color::LightRed => "Color::LightRed",
            Color::LightGreen => "Color::LightGreen",
            Color::LightYellow => "Color::LightYellow",
            Color::LightBlue => "Color::LightBlue",
            Color::LightMagenta => "Color::LightMagenta",
            Color::LightCyan => "Color::LightCyan",
            Color::White => "Color::White",
            Color::Indexed(i) => return format_compact!("Color::Indexed({i})"),
            Color::Rgb(r, g, b) => {
                return format_compact!("Color::from_u32(0x{r:02x}{g:02x}{b:02x})")
            },
        }
        .to_compact_string()
    }
}

impl DslFormat for Rect {
    fn dsl_format(&self) -> CompactString {
        format_compact!(
            "Rect::new({}, {}, {}, {})",
            self.x,
            self.y,
            self.width,
            self.height
        )
    }
}

impl DslFormat for ratatui_core::layout::Size {
    fn dsl_format(&self) -> CompactString {
        format_compact!("Size::new({}, {})", self.width, self.height)
    }
}

impl DslFormat for ColorSpace {
    fn dsl_format(&self) -> CompactString {
        format_compact!("{:?}", self)
    }
}

impl DslFormat for bool {
    fn dsl_format(&self) -> CompactString {
        if *self {
            CompactString::const_new("true")
        } else {
            CompactString::const_new("false")
        }
    }
}

impl DslFormat for f32 {
    fn dsl_format(&self) -> CompactString {
        format_compact!("{:}", self)
    }
}

impl DslFormat for RepeatMode {
    fn dsl_format(&self) -> CompactString {
        match self {
            RepeatMode::Forever => "RepeatMode::Forever".to_compact_string(),
            RepeatMode::Times(n) => format_compact!("RepeatMode::Times({n})"),
            RepeatMode::Duration(d) => format_compact!("RepeatMode::Duration({})", d.dsl_format()),
        }
    }
}

impl DslFormat for Motion {
    fn dsl_format(&self) -> CompactString {
        match self {
            Motion::LeftToRight => "Motion::LeftToRight",
            Motion::RightToLeft => "Motion::RightToLeft",
            Motion::UpToDown => "Motion::UpToDown",
            Motion::DownToUp => "Motion::DownToUp",
        }
        .to_compact_string()
    }
}

impl DslFormat for crate::fx::ExpandDirection {
    fn dsl_format(&self) -> CompactString {
        match self {
            crate::fx::ExpandDirection::Horizontal => "ExpandDirection::Horizontal",
            crate::fx::ExpandDirection::Vertical => "ExpandDirection::Vertical",
        }
        .to_compact_string()
    }
}

impl DslFormat for EvolveSymbolSet {
    fn dsl_format(&self) -> CompactString {
        match self {
            EvolveSymbolSet::BlocksHorizontal => "EvolveSymbolSet::BlocksHorizontal",
            EvolveSymbolSet::BlocksVertical => "EvolveSymbolSet::BlocksVertical",
            EvolveSymbolSet::CircleFill => "EvolveSymbolSet::CircleFill",
            EvolveSymbolSet::Circles => "EvolveSymbolSet::Circles",
            EvolveSymbolSet::Quadrants => "EvolveSymbolSet::Quadrants",
            EvolveSymbolSet::Shaded => "EvolveSymbolSet::Shaded",
            EvolveSymbolSet::Squares => "EvolveSymbolSet::Squares",
        }
        .to_compact_string()
    }
}

impl DslFormat for crate::pattern::AnyPattern {
    fn dsl_format(&self) -> CompactString {
        match self {
            crate::pattern::AnyPattern::Identity => "AnyPattern::Identity".to_compact_string(),
            crate::pattern::AnyPattern::Radial(p) => {
                format_compact!("AnyPattern::Radial({})", p.dsl_format())
            },
            crate::pattern::AnyPattern::Diagonal(p) => {
                format_compact!("AnyPattern::Diagonal({})", p.dsl_format())
            },
            crate::pattern::AnyPattern::Checkerboard(p) => {
                format_compact!("AnyPattern::Checkerboard({})", p.dsl_format())
            },
            crate::pattern::AnyPattern::Sweep(p) => {
                format_compact!("AnyPattern::Sweep({})", p.dsl_format())
            },
            crate::pattern::AnyPattern::Coalesce(p) => {
                format_compact!("AnyPattern::Coalesce({})", p.dsl_format())
            },
            crate::pattern::AnyPattern::Dissolve(p) => {
                format_compact!("AnyPattern::Dissolve({})", p.dsl_format())
            },
            crate::pattern::AnyPattern::Wave(p) => {
                format_compact!("AnyPattern::Wave({})", p.dsl_format())
            },
            crate::pattern::AnyPattern::Diamond(p) => {
                format_compact!("AnyPattern::Diamond({})", p.dsl_format())
            },
            crate::pattern::AnyPattern::Spiral(p) => {
                format_compact!("AnyPattern::Spiral({})", p.dsl_format())
            },
            crate::pattern::AnyPattern::Combined(p) => {
                format_compact!("AnyPattern::Combined({})", p.dsl_format())
            },
            crate::pattern::AnyPattern::Inverted(p) => {
                format_compact!("AnyPattern::Inverted({})", p.dsl_format())
            },
            crate::pattern::AnyPattern::Blend(p) => {
                format_compact!("AnyPattern::Blend({})", p.dsl_format())
            },
        }
    }
}

impl DslFormat for Style {
    fn dsl_format(&self) -> CompactString {
        let mut methods = CompactString::new("");

        if let Some(fg) = self.fg {
            methods.push_str(&format_compact!(".fg({})", fg.dsl_format()));
        }

        if let Some(bg) = self.bg {
            methods.push_str(&format_compact!(".bg({})", bg.dsl_format()));
        }

        self.add_modifier.iter().for_each(|m| {
            methods.push_str(&format_compact!(".add_modifier({:?})", m));
        });

        self.sub_modifier.iter().for_each(|m| {
            methods.push_str(&format_compact!(".remove_modifier({:?})", m));
        });

        format_compact!("Style::new(){}", methods)
    }
}

impl DslFormat for Modifier {
    fn dsl_format(&self) -> CompactString {
        format_compact!("{:?}", self)
    }
}

impl DslFormat for Interpolation {
    fn dsl_format(&self) -> CompactString {
        match self {
            Interpolation::BackIn => "Interpolation::BackIn",
            Interpolation::BackOut => "Interpolation::BackOut",
            Interpolation::BackInOut => "Interpolation::BackInOut",
            Interpolation::BounceIn => "Interpolation::BounceIn",
            Interpolation::BounceOut => "Interpolation::BounceOut",
            Interpolation::BounceInOut => "Interpolation::BounceInOut",
            Interpolation::CircIn => "Interpolation::CircIn",
            Interpolation::CircOut => "Interpolation::CircOut",
            Interpolation::CircInOut => "Interpolation::CircInOut",
            Interpolation::CubicIn => "Interpolation::CubicIn",
            Interpolation::CubicOut => "Interpolation::CubicOut",
            Interpolation::CubicInOut => "Interpolation::CubicInOut",
            Interpolation::ElasticIn => "Interpolation::ElasticIn",
            Interpolation::ElasticOut => "Interpolation::ElasticOut",
            Interpolation::ElasticInOut => "Interpolation::ElasticInOut",
            Interpolation::ExpoIn => "Interpolation::ExpoIn",
            Interpolation::ExpoOut => "Interpolation::ExpoOut",
            Interpolation::ExpoInOut => "Interpolation::ExpoInOut",
            Interpolation::Linear => "Interpolation::Linear",
            Interpolation::QuadIn => "Interpolation::QuadIn",
            Interpolation::QuadOut => "Interpolation::QuadOut",
            Interpolation::QuadInOut => "Interpolation::QuadInOut",
            Interpolation::QuartIn => "Interpolation::QuartIn",
            Interpolation::QuartOut => "Interpolation::QuartOut",
            Interpolation::QuartInOut => "Interpolation::QuartInOut",
            Interpolation::QuintIn => "Interpolation::QuintIn",
            Interpolation::QuintOut => "Interpolation::QuintOut",
            Interpolation::QuintInOut => "Interpolation::QuintInOut",
            Interpolation::Reverse => "Interpolation::Reverse",
            Interpolation::SmoothStep => "Interpolation::SmoothStep",
            Interpolation::Spring => "Interpolation::Spring",
            Interpolation::SineIn => "Interpolation::SineIn",
            Interpolation::SineOut => "Interpolation::SineOut",
            Interpolation::SineInOut => "Interpolation::SineInOut",
        }
        .to_compact_string()
    }
}

impl DslFormat for EffectTimer {
    fn dsl_format(&self) -> CompactString {
        // If the timer is reversed, we need to flip the interpolation
        // for correct DSL representation, as effects that are reversed
        // at construction-time are "mirrored" effects; in order to get
        // the same interpolation curve, we need to flip the interpolation.
        let interpolation = if self.is_reversed() {
            self.interpolation().flipped()
        } else {
            self.interpolation()
        };

        format_compact!(
            "EffectTimer::from_ms({}, {})",
            self.duration().as_millis(),
            interpolation.dsl_format(),
        )
    }
}

impl DslFormat for Offset {
    fn dsl_format(&self) -> CompactString {
        format_compact!("Offset {{ x: {}, y: {} }}", self.x, self.y)
    }
}

impl DslFormat for Duration {
    fn dsl_format(&self) -> CompactString {
        format_compact!("Duration::from_millis({})", self.as_millis())
    }
}

impl DslFormat for Constraint {
    fn dsl_format(&self) -> CompactString {
        match self {
            Constraint::Length(n) => format_compact!("Constraint::Length({n})"),
            Constraint::Percentage(n) => format_compact!("Constraint::Percentage({n})"),
            Constraint::Ratio(num, den) => format_compact!("Constraint::Ratio({num}, {den})"),
            Constraint::Min(n) => format_compact!("Constraint::Min({n})"),
            Constraint::Max(n) => format_compact!("Constraint::Max({n})"),
            Constraint::Fill(n) => format_compact!("Constraint::Fill({n})"),
        }
    }
}

impl DslFormat for Direction {
    fn dsl_format(&self) -> CompactString {
        match self {
            Direction::Horizontal => "Direction::Horizontal",
            Direction::Vertical => "Direction::Vertical",
        }
        .to_compact_string()
    }
}

impl DslFormat for Flex {
    fn dsl_format(&self) -> CompactString {
        match self {
            Flex::Legacy => "Flex::Legacy",
            Flex::Start => "Flex::Start",
            Flex::End => "Flex::End",
            Flex::Center => "Flex::Center",
            Flex::SpaceBetween => "Flex::SpaceBetween",
            Flex::SpaceAround => "Flex::SpaceAround",
            Flex::SpaceEvenly => "Flex::SpaceEvenly",
        }
        .to_compact_string()
    }
}

impl DslFormat for Margin {
    fn dsl_format(&self) -> CompactString {
        format_compact!("Margin::new({}, {})", self.horizontal, self.vertical)
    }
}

impl DslFormat for CellFilter {
    fn dsl_format(&self) -> CompactString {
        use core::borrow::Borrow;

        fn format(filters: &[CellFilter]) -> CompactString {
            filters
                .iter()
                .map(CellFilter::to_string)
                .collect::<Vec<String>>()
                .join_compact(", ")
        }

        match self {
            CellFilter::All => CompactString::const_new("CellFilter::All"),
            CellFilter::Area(r) => format_compact!("CellFilter::Area({})", r.dsl_format()),
            CellFilter::RefArea(ref_rect) => format_compact!(
                "CellFilter::RefArea(RefRect::new({}))",
                ref_rect.get().dsl_format()
            ),
            CellFilter::FgColor(color) => {
                format_compact!("CellFilter::FgColor({})", color.dsl_format())
            },
            CellFilter::BgColor(color) => {
                format_compact!("CellFilter::BgColor({})", color.dsl_format())
            },
            CellFilter::Inner(m) => format_compact!("CellFilter::Inner({})", m.dsl_format()),
            CellFilter::Outer(m) => format_compact!("CellFilter::Outer({})", m.dsl_format()),
            CellFilter::Text => CompactString::const_new("CellFilter::Text"),
            CellFilter::NonEmpty => CompactString::const_new("CellFilter::NonEmpty"),
            CellFilter::AllOf(filters) => format_compact!("CellFilter::AllOf({})", format(filters)),
            CellFilter::AnyOf(filters) => format_compact!("CellFilter::AnyOf({})", format(filters)),
            CellFilter::NoneOf(filters) => {
                format_compact!("CellFilter::NoneOf({})", format(filters))
            },
            CellFilter::Not(filter) => {
                let f: &CellFilter = filter.borrow();
                format_compact!("Not(Box::new({}))", f.dsl_format())
            },
            CellFilter::Layout(l, idx) => format_compact!("CellFilter::Layout({l:#?}, {idx})"),
            CellFilter::PositionFn(_) => "CellFilter::PositionFn(fn)".to_compact_string(),
            CellFilter::EvalCell(_) => "CellFilter::EvalCell(fn)".to_compact_string(),
            CellFilter::Static(filter) => {
                format_compact!("CellFilter::Static(Box::new({}))", filter.dsl_format())
            },
        }
    }
}

impl DslFormat for Modulator {
    fn dsl_format(&self) -> CompactString {
        let mut s = format_compact!(
            "Modulator::{}({}, {}, {})",
            self.func_name(),
            fmt_f32(self.kx()),
            fmt_f32(self.ky()),
            fmt_f32(self.kt()),
        );
        if self.phase_offset() != 0.0 {
            s.push_str(&format_compact!(".phase({})", fmt_f32(self.phase_offset())));
        }
        if self.intensity_value() != 1.0 {
            s.push_str(&format_compact!(
                ".intensity({})",
                fmt_f32(self.intensity_value())
            ));
        }
        match self.target() {
            ModTarget::Phase => {},
            ModTarget::Amplitude => s.push_str(".on_amplitude()"),
        }
        s
    }
}

impl DslFormat for Oscillator {
    fn dsl_format(&self) -> CompactString {
        let mut s = format_compact!(
            "Oscillator::{}({}, {}, {})",
            self.func_name(),
            fmt_f32(self.kx()),
            fmt_f32(self.ky()),
            fmt_f32(self.kt()),
        );
        if self.phase_offset() != 0.0 {
            s.push_str(&format_compact!(".phase({})", fmt_f32(self.phase_offset())));
        }
        if let Some(m) = self.modulator() {
            s.push_str(&format_compact!(".modulated_by({})", m.dsl_format()));
        }
        s
    }
}

impl DslFormat for WavePattern {
    fn dsl_format(&self) -> CompactString {
        let mut layers = self.layers().iter();
        let first = layers
            .next()
            .expect("WavePattern must have at least one layer");
        let mut s = format_compact!("WavePattern::new({})", first.dsl_format());
        for layer in layers {
            s.push_str(&format_compact!(".with_layer({})", layer.dsl_format()));
        }
        if self.contrast() != 1 {
            s.push_str(&format_compact!(".with_contrast({})", self.contrast()));
        }
        if (self.transition_width() - 0.15).abs() > f32::EPSILON {
            s.push_str(&format_compact!(
                ".with_transition_width({})",
                fmt_f32(self.transition_width())
            ));
        }
        s
    }
}

impl DslFormat for WaveLayer {
    fn dsl_format(&self) -> CompactString {
        let mut s = format_compact!("WaveLayer::new({})", self.oscillator_a().dsl_format());
        if let Some((combinator, osc_b)) = self.oscillator_b() {
            let method = match combinator {
                Combinator::Multiply => "multiply",
                Combinator::Average => "average",
                Combinator::Max => "max",
            };
            s.push_str(&format_compact!(".{}({})", method, osc_b.dsl_format()));
        }
        if self.amplitude_value() != 1.0 {
            s.push_str(&format_compact!(
                ".amplitude({})",
                fmt_f32(self.amplitude_value())
            ));
        }
        match self.post_transform() {
            PostTransform::None => {},
            PostTransform::Power(n) => s.push_str(&format_compact!(".power({n})")),
            PostTransform::Abs => s.push_str(".abs()"),
        }
        s
    }
}

impl DslFormat for crate::pattern::BlendPattern {
    fn dsl_format(&self) -> CompactString {
        format_compact!(
            "BlendPattern::new({}, {})",
            self.pattern_a().dsl_format(),
            self.pattern_b().dsl_format()
        )
    }
}

#[cfg(test)]
mod tests {
    use ratatui_core::style::{Color, Modifier, Style};

    use crate::{dsl::DslFormat, fx::RepeatMode, Duration, EffectTimer, Interpolation, Motion};

    #[test]
    fn test_color_dsl_format() {
        // Test basic colors
        assert_eq!(Color::Black.dsl_format(), "Color::Black");
        assert_eq!(Color::Red.dsl_format(), "Color::Red");
        assert_eq!(Color::Green.dsl_format(), "Color::Green");
        assert_eq!(Color::Yellow.dsl_format(), "Color::Yellow");
        assert_eq!(Color::Blue.dsl_format(), "Color::Blue");
        assert_eq!(Color::Magenta.dsl_format(), "Color::Magenta");
        assert_eq!(Color::Cyan.dsl_format(), "Color::Cyan");
        assert_eq!(Color::White.dsl_format(), "Color::White");

        // Test RGB colors
        assert_eq!(
            Color::Rgb(255, 127, 63).dsl_format(),
            "Color::from_u32(0xff7f3f)"
        );
        assert_eq!(
            Color::Rgb(0, 0, 0).dsl_format(),
            "Color::from_u32(0x000000)"
        );
        assert_eq!(
            Color::Rgb(255, 255, 255).dsl_format(),
            "Color::from_u32(0xffffff)"
        );

        // Test indexed colors
        let indexed_color = Color::Indexed(42);
        assert_eq!(indexed_color.dsl_format(), "Color::Indexed(42)");
    }

    #[test]
    fn test_repeat_mode_dsl_format() {
        // Test Forever mode
        assert_eq!(RepeatMode::Forever.dsl_format(), "RepeatMode::Forever");

        // Test Times mode
        assert_eq!(RepeatMode::Times(3).dsl_format(), "RepeatMode::Times(3)");
        assert_eq!(RepeatMode::Times(1).dsl_format(), "RepeatMode::Times(1)");
        assert_eq!(
            RepeatMode::Times(100).dsl_format(),
            "RepeatMode::Times(100)"
        );

        // Test Duration mode
        assert_eq!(
            RepeatMode::Duration(Duration::from_millis(500)).dsl_format(),
            "RepeatMode::Duration(Duration::from_millis(500))"
        );
        assert_eq!(
            RepeatMode::Duration(Duration::from_millis(0)).dsl_format(),
            "RepeatMode::Duration(Duration::from_millis(0))"
        );
        assert_eq!(
            RepeatMode::Duration(Duration::from_millis(1000)).dsl_format(),
            "RepeatMode::Duration(Duration::from_millis(1000))"
        );
    }

    #[test]
    fn test_motion_dsl_format() {
        // Test all Motion variants
        assert_eq!(Motion::LeftToRight.dsl_format(), "Motion::LeftToRight");
        assert_eq!(Motion::RightToLeft.dsl_format(), "Motion::RightToLeft");
        assert_eq!(Motion::UpToDown.dsl_format(), "Motion::UpToDown");
        assert_eq!(Motion::DownToUp.dsl_format(), "Motion::DownToUp");
    }

    #[test]
    fn test_style_dsl_format() {
        // Test default style
        assert_eq!(Style::new().dsl_format(), "Style::new()");

        // Test style with foreground color
        let style_fg = Style::new().fg(Color::Red);
        assert_eq!(style_fg.dsl_format(), "Style::new().fg(Color::Red)");

        // Test style with background color
        let style_bg = Style::new().bg(Color::Blue);
        assert_eq!(style_bg.dsl_format(), "Style::new().bg(Color::Blue)");

        // Test style with foreground and background colors
        let style_fg_bg = Style::new().fg(Color::Red).bg(Color::Blue);
        assert_eq!(
            style_fg_bg.dsl_format(),
            "Style::new().fg(Color::Red).bg(Color::Blue)"
        );

        // Test style with modifiers
        let style_mod = Style::new().add_modifier(Modifier::BOLD);
        assert_eq!(style_mod.dsl_format(), "Style::new().add_modifier(BOLD)");

        // Test style with multiple modifiers
        let style_mods = Style::new()
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::ITALIC);
        assert_eq!(
            style_mods.dsl_format(),
            "Style::new().add_modifier(BOLD).add_modifier(ITALIC)"
        );

        // Test style with colors and modifiers
        let style_complex = Style::new()
            .fg(Color::Red)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::UNDERLINED);
        assert_eq!(
            style_complex.dsl_format(),
            "Style::new().fg(Color::Red).bg(Color::Blue).add_modifier(BOLD).add_modifier(UNDERLINED)"
        );

        // Test style with subtracted modifiers
        let style_sub = Style::new().remove_modifier(Modifier::BOLD);
        assert_eq!(style_sub.dsl_format(), "Style::new().remove_modifier(BOLD)");
    }

    #[test]
    fn test_modifier_dsl_format() {
        // Test all modifier variants
        assert_eq!(Modifier::BOLD.dsl_format(), "BOLD");
        assert_eq!(Modifier::DIM.dsl_format(), "DIM");
        assert_eq!(Modifier::ITALIC.dsl_format(), "ITALIC");
        assert_eq!(Modifier::UNDERLINED.dsl_format(), "UNDERLINED");
        assert_eq!(Modifier::SLOW_BLINK.dsl_format(), "SLOW_BLINK");
        assert_eq!(Modifier::RAPID_BLINK.dsl_format(), "RAPID_BLINK");
        assert_eq!(Modifier::REVERSED.dsl_format(), "REVERSED");
        assert_eq!(Modifier::HIDDEN.dsl_format(), "HIDDEN");
        assert_eq!(Modifier::CROSSED_OUT.dsl_format(), "CROSSED_OUT");

        // Test combined modifiers
        let combined = Modifier::BOLD | Modifier::ITALIC;
        assert_eq!(combined.dsl_format(), "BOLD | ITALIC");
    }

    #[test]
    fn test_interpolation_dsl_format() {
        // Test all interpolation variants
        assert_eq!(Interpolation::Linear.dsl_format(), "Interpolation::Linear");
        assert_eq!(
            Interpolation::Reverse.dsl_format(),
            "Interpolation::Reverse"
        );

        // Back family
        assert_eq!(Interpolation::BackIn.dsl_format(), "Interpolation::BackIn");
        assert_eq!(
            Interpolation::BackOut.dsl_format(),
            "Interpolation::BackOut"
        );
        assert_eq!(
            Interpolation::BackInOut.dsl_format(),
            "Interpolation::BackInOut"
        );

        // Bounce family
        assert_eq!(
            Interpolation::BounceIn.dsl_format(),
            "Interpolation::BounceIn"
        );
        assert_eq!(
            Interpolation::BounceOut.dsl_format(),
            "Interpolation::BounceOut"
        );
        assert_eq!(
            Interpolation::BounceInOut.dsl_format(),
            "Interpolation::BounceInOut"
        );

        // Circ family
        assert_eq!(Interpolation::CircIn.dsl_format(), "Interpolation::CircIn");
        assert_eq!(
            Interpolation::CircOut.dsl_format(),
            "Interpolation::CircOut"
        );
        assert_eq!(
            Interpolation::CircInOut.dsl_format(),
            "Interpolation::CircInOut"
        );

        // Cubic family
        assert_eq!(
            Interpolation::CubicIn.dsl_format(),
            "Interpolation::CubicIn"
        );
        assert_eq!(
            Interpolation::CubicOut.dsl_format(),
            "Interpolation::CubicOut"
        );
        assert_eq!(
            Interpolation::CubicInOut.dsl_format(),
            "Interpolation::CubicInOut"
        );

        // Elastic family
        assert_eq!(
            Interpolation::ElasticIn.dsl_format(),
            "Interpolation::ElasticIn"
        );
        assert_eq!(
            Interpolation::ElasticOut.dsl_format(),
            "Interpolation::ElasticOut"
        );
        assert_eq!(
            Interpolation::ElasticInOut.dsl_format(),
            "Interpolation::ElasticInOut"
        );

        // Expo family
        assert_eq!(Interpolation::ExpoIn.dsl_format(), "Interpolation::ExpoIn");
        assert_eq!(
            Interpolation::ExpoOut.dsl_format(),
            "Interpolation::ExpoOut"
        );
        assert_eq!(
            Interpolation::ExpoInOut.dsl_format(),
            "Interpolation::ExpoInOut"
        );

        // Quad family
        assert_eq!(Interpolation::QuadIn.dsl_format(), "Interpolation::QuadIn");
        assert_eq!(
            Interpolation::QuadOut.dsl_format(),
            "Interpolation::QuadOut"
        );
        assert_eq!(
            Interpolation::QuadInOut.dsl_format(),
            "Interpolation::QuadInOut"
        );

        // Quart family
        assert_eq!(
            Interpolation::QuartIn.dsl_format(),
            "Interpolation::QuartIn"
        );
        assert_eq!(
            Interpolation::QuartOut.dsl_format(),
            "Interpolation::QuartOut"
        );
        assert_eq!(
            Interpolation::QuartInOut.dsl_format(),
            "Interpolation::QuartInOut"
        );

        // Quint family
        assert_eq!(
            Interpolation::QuintIn.dsl_format(),
            "Interpolation::QuintIn"
        );
        assert_eq!(
            Interpolation::QuintOut.dsl_format(),
            "Interpolation::QuintOut"
        );
        assert_eq!(
            Interpolation::QuintInOut.dsl_format(),
            "Interpolation::QuintInOut"
        );

        // Sine family
        assert_eq!(Interpolation::SineIn.dsl_format(), "Interpolation::SineIn");
        assert_eq!(
            Interpolation::SineOut.dsl_format(),
            "Interpolation::SineOut"
        );
        assert_eq!(
            Interpolation::SineInOut.dsl_format(),
            "Interpolation::SineInOut"
        );
    }

    #[test]
    fn test_effect_timer_dsl_format() {
        // Test with linear interpolation
        let timer_linear = EffectTimer::from_ms(1000, Interpolation::Linear);
        assert_eq!(
            timer_linear.dsl_format(),
            "EffectTimer::from_ms(1000, Interpolation::Linear)"
        );

        // Test with different interpolation types
        let timer_bounce = EffectTimer::from_ms(500, Interpolation::BounceOut);
        assert_eq!(
            timer_bounce.dsl_format(),
            "EffectTimer::from_ms(500, Interpolation::BounceOut)"
        );

        // Test with different durations
        let timer_short = EffectTimer::from_ms(100, Interpolation::Linear);
        assert_eq!(
            timer_short.dsl_format(),
            "EffectTimer::from_ms(100, Interpolation::Linear)"
        );

        let timer_long = EffectTimer::from_ms(5000, Interpolation::Linear);
        assert_eq!(
            timer_long.dsl_format(),
            "EffectTimer::from_ms(5000, Interpolation::Linear)"
        );

        // Test with zero duration
        let timer_zero = EffectTimer::from_ms(0, Interpolation::Linear);
        assert_eq!(
            timer_zero.dsl_format(),
            "EffectTimer::from_ms(0, Interpolation::Linear)"
        );
    }

    #[test]
    fn test_duration_dsl_format() {
        // Test various durations
        assert_eq!(
            Duration::from_millis(0).dsl_format(),
            "Duration::from_millis(0)"
        );
        assert_eq!(
            Duration::from_millis(1).dsl_format(),
            "Duration::from_millis(1)"
        );
        assert_eq!(
            Duration::from_millis(500).dsl_format(),
            "Duration::from_millis(500)"
        );
        assert_eq!(
            Duration::from_millis(1000).dsl_format(),
            "Duration::from_millis(1000)"
        );
        assert_eq!(
            Duration::from_millis(u32::MAX as _).dsl_format(),
            format!("Duration::from_millis({})", u32::MAX)
        );
    }

    #[test]
    fn test_constraint_dsl_format() {
        use ratatui_core::layout::Constraint;

        // Test all constraint types
        assert_eq!(
            Constraint::Length(10).dsl_format(),
            "Constraint::Length(10)"
        );
        assert_eq!(
            Constraint::Percentage(50).dsl_format(),
            "Constraint::Percentage(50)"
        );
        assert_eq!(
            Constraint::Ratio(1, 3).dsl_format(),
            "Constraint::Ratio(1, 3)"
        );
        assert_eq!(Constraint::Min(5).dsl_format(), "Constraint::Min(5)");
        assert_eq!(Constraint::Max(20).dsl_format(), "Constraint::Max(20)");
        assert_eq!(Constraint::Fill(0).dsl_format(), "Constraint::Fill(0)");
    }

    #[test]
    fn test_direction_dsl_format() {
        use ratatui_core::layout::Direction;

        assert_eq!(Direction::Horizontal.dsl_format(), "Direction::Horizontal");
        assert_eq!(Direction::Vertical.dsl_format(), "Direction::Vertical");
    }

    #[cfg(feature = "dsl")]
    #[test]
    fn test_pattern_dsl_format() {
        use crate::pattern::*;

        // Test RadialPattern
        assert_eq!(
            RadialPattern::center().dsl_format(),
            "RadialPattern::center()"
        );
        assert_eq!(
            RadialPattern::new(0.3, 0.7).dsl_format(),
            "RadialPattern::new(0.3, 0.7)"
        );
        assert_eq!(
            RadialPattern::with_transition((0.2, 0.8), 3.5).dsl_format(),
            "RadialPattern::with_transition((0.2, 0.8), 3.5)"
        );

        // Test DiagonalPattern
        assert_eq!(
            DiagonalPattern::top_left_to_bottom_right().dsl_format(),
            "DiagonalPattern::top_left_to_bottom_right()"
        );
        assert_eq!(
            DiagonalPattern::top_left_to_bottom_right()
                .with_transition_width(4.0)
                .dsl_format(),
            "DiagonalPattern::top_left_to_bottom_right().with_transition_width(4.0)"
        );

        // Test CheckerboardPattern
        assert_eq!(
            CheckerboardPattern::default().dsl_format(),
            "CheckerboardPattern::default()"
        );
        assert_eq!(
            CheckerboardPattern::with_cell_size(3).dsl_format(),
            "CheckerboardPattern::with_cell_size(3)"
        );
        assert_eq!(
            CheckerboardPattern::new(4, 1.5).dsl_format(),
            "CheckerboardPattern::new(4, 1.5)"
        );

        // Test SweepPattern
        assert_eq!(
            SweepPattern::left_to_right(5).dsl_format(),
            "SweepPattern::left_to_right(5)"
        );
        assert_eq!(
            SweepPattern::right_to_left(3).dsl_format(),
            "SweepPattern::right_to_left(3)"
        );
        assert_eq!(
            SweepPattern::up_to_down(7).dsl_format(),
            "SweepPattern::up_to_down(7)"
        );
        assert_eq!(
            SweepPattern::down_to_up(2).dsl_format(),
            "SweepPattern::down_to_up(2)"
        );

        // Test CoalescePattern
        assert_eq!(
            CoalescePattern::new().dsl_format(),
            "CoalescePattern::new()"
        );

        // Test DissolvePattern
        assert_eq!(
            DissolvePattern::new().dsl_format(),
            "DissolvePattern::new()"
        );
    }
}
