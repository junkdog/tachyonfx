use ratatui::style::Color;
use crate::fx::RepeatMode;
use crate::{Duration, EffectTimer, Interpolation, Motion};
use crate::color_ext::ToRgbComponents;

pub trait DslFormat {
    fn dsl_format(&self) -> String;
}

impl DslFormat for Color {
    fn dsl_format(&self) -> String {
        let (r, g, b) = self.to_rgb();
        format!("Color::from_u32(0x{:02x}{:02x}{:02x})", r, g, b)
    }
}

impl DslFormat for RepeatMode {
    fn dsl_format(&self) -> String {
        match self {
            RepeatMode::Forever =>
                "RepeatMode::Forever".to_string(),
            RepeatMode::Times(n) =>
                format!("RepeatMode::Times({})", n),
            RepeatMode::Duration(d) =>
                format!("RepeatMode::Duration(Duration::from_millis({}))", d.as_millis()),
        }
    }
}

impl DslFormat for Motion {
    fn dsl_format(&self) -> String {
        match self {
            Motion::LeftToRight => "Motion::LeftToRight".to_string(),
            Motion::RightToLeft => "Motion::RightToLeft".to_string(),
            Motion::UpToDown    => "Motion::UpToDown".to_string(),
            Motion::DownToUp    => "Motion::DownToUp".to_string(),
        }
    }
}

impl DslFormat for Interpolation {
    fn dsl_format(&self) -> String {
        match self {
            Interpolation::BackIn => "Interpolation::BackIn".to_string(),
            Interpolation::BackOut => "Interpolation::BackOut".to_string(),
            Interpolation::BackInOut => "Interpolation::BackInOut".to_string(),
            Interpolation::BounceIn => "Interpolation::BounceIn".to_string(),
            Interpolation::BounceOut => "Interpolation::BounceOut".to_string(),
            Interpolation::BounceInOut => "Interpolation::BounceInOut".to_string(),
            Interpolation::CircIn => "Interpolation::CircIn".to_string(),
            Interpolation::CircOut => "Interpolation::CircOut".to_string(),
            Interpolation::CircInOut => "Interpolation::CircInOut".to_string(),
            Interpolation::CubicIn => "Interpolation::CubicIn".to_string(),
            Interpolation::CubicOut => "Interpolation::CubicOut".to_string(),
            Interpolation::CubicInOut => "Interpolation::CubicInOut".to_string(),
            Interpolation::ElasticIn => "Interpolation::ElasticIn".to_string(),
            Interpolation::ElasticOut => "Interpolation::ElasticOut".to_string(),
            Interpolation::ElasticInOut => "Interpolation::ElasticInOut".to_string(),
            Interpolation::ExpoIn => "Interpolation::ExpoIn".to_string(),
            Interpolation::ExpoOut => "Interpolation::ExpoOut".to_string(),
            Interpolation::ExpoInOut => "Interpolation::ExpoInOut".to_string(),
            Interpolation::Linear => "Interpolation::Linear".to_string(),
            Interpolation::QuadIn => "Interpolation::QuadIn".to_string(),
            Interpolation::QuadOut => "Interpolation::QuadOut".to_string(),
            Interpolation::QuadInOut => "Interpolation::QuadInOut".to_string(),
            Interpolation::QuartIn => "Interpolation::QuartIn".to_string(),
            Interpolation::QuartOut => "Interpolation::QuartOut".to_string(),
            Interpolation::QuartInOut => "Interpolation::QuartInOut".to_string(),
            Interpolation::QuintIn => "Interpolation::QuintIn".to_string(),
            Interpolation::QuintOut => "Interpolation::QuintOut".to_string(),
            Interpolation::QuintInOut => "Interpolation::QuintInOut".to_string(),
            Interpolation::Reverse => "Interpolation::Reverse".to_string(),
            Interpolation::SineIn => "Interpolation::SineIn".to_string(),
            Interpolation::SineOut => "Interpolation::SineOut".to_string(),
            Interpolation::SineInOut => "Interpolation::SineInOut".to_string(),
        }
    }
}

impl DslFormat for EffectTimer {
    fn dsl_format(&self) -> String {
        format!("EffectTimer::from_ms({}, {})",
            self.duration().as_millis(),
            self.interpolation().dsl_format(),
        )
    }
}

impl DslFormat for Duration {
    fn dsl_format(&self) -> String {
        format!("Duration::from_millis({})", self.as_millis())
    }
}