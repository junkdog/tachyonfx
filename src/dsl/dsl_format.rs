use compact_str::{format_compact, CompactString, ToCompactString};
use ratatui::style::{Color, Modifier, Style};
use crate::fx::RepeatMode;
use crate::{Duration, EffectTimer, Interpolation, Motion};
use crate::color_ext::ToRgbComponents;

/// A trait for converting types into their DSL (Domain Specific Language) string representation.
///
/// This trait enables types to be formatted as valid tachyonfx DSL expressions that can be
/// parsed back into effect definitions. It's primarily used for serializing effects and their
/// parameters into a textual format that matches the tachyonfx DSL syntax.
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
        let (r, g, b) = self.to_rgb();
        format_compact!("Color::from_u32(0x{:02x}{:02x}{:02x})", r, g, b)
    }
}

impl DslFormat for RepeatMode {
    fn dsl_format(&self) -> CompactString {
        match self {
            RepeatMode::Forever =>
                "RepeatMode::Forever".to_compact_string(),
            RepeatMode::Times(n) =>
                format_compact!("RepeatMode::Times({})", n),
            RepeatMode::Duration(d) =>
                format_compact!("RepeatMode::Duration(Duration::from_millis({}))", d.as_millis()),
        }
    }
}

impl DslFormat for Motion {
    fn dsl_format(&self) -> CompactString {
        match self {
            Motion::LeftToRight => "Motion::LeftToRight".to_compact_string(),
            Motion::RightToLeft => "Motion::RightToLeft".to_compact_string(),
            Motion::UpToDown    => "Motion::UpToDown".to_compact_string(),
            Motion::DownToUp    => "Motion::DownToUp".to_compact_string(),
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
            methods.push_str(&format_compact!(".sub_modifier({:?})", m));
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
            Interpolation::SineIn => "Interpolation::SineIn",
            Interpolation::SineOut => "Interpolation::SineOut",
            Interpolation::SineInOut => "Interpolation::SineInOut",
        }.to_compact_string()
    }
}

impl DslFormat for EffectTimer {
    fn dsl_format(&self) -> CompactString {
        format_compact!("EffectTimer::from_ms({}, {})",
            self.duration().as_millis(),
            self.interpolation().dsl_format(),
        )
    }
}

impl DslFormat for Duration {
    fn dsl_format(&self) -> CompactString {
        format_compact!("Duration::from_millis({})", self.as_millis())
    }
}