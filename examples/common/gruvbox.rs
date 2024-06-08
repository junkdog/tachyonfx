use ratatui::prelude::*;


#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[allow(dead_code)]
pub enum Gruvbox {
    Dark0Hard,
    Dark0,
    Dark0Soft,
    Dark1,
    Dark2,
    Dark3,
    Dark4,
    Gray245,
    Gray244,
    Light0Hard,
    Light0,
    Light0Soft,
    Light1,
    Light2,
    Light3,
    Light4,
    RedBright,
    GreenBright,
    YellowBright,
    BlueBright,
    PurpleBright,
    AquaBright,
    OrangeBright,
    Red,
    Green,
    Yellow,
    Blue,
    Purple,
    Aqua,
    Orange,
    RedDim,
    GreenDim,
    YellowDim,
    BlueDim,
    PurpleDim,
    AquaDim,
    OrangeDim,
}

impl Gruvbox {
    const fn color(&self) -> Color {
        match self {
            Gruvbox::Dark0Hard    => Color::from_u32(0x1d2021),
            Gruvbox::Dark0        => Color::from_u32(0x282828),
            Gruvbox::Dark0Soft    => Color::from_u32(0x32302f),
            Gruvbox::Dark1        => Color::from_u32(0x3c3836),
            Gruvbox::Dark2        => Color::from_u32(0x504945),
            Gruvbox::Dark3        => Color::from_u32(0x665c54),
            Gruvbox::Dark4        => Color::from_u32(0x7c6f64),
            Gruvbox::Gray245      => Color::from_u32(0x928374),
            Gruvbox::Gray244      => Color::from_u32(0x928374),
            Gruvbox::Light0Hard   => Color::from_u32(0xf9f5d7),
            Gruvbox::Light0       => Color::from_u32(0xfbf1c7),
            Gruvbox::Light0Soft   => Color::from_u32(0xf2e5bc),
            Gruvbox::Light1       => Color::from_u32(0xebdbb2),
            Gruvbox::Light2       => Color::from_u32(0xd5c4a1),
            Gruvbox::Light3       => Color::from_u32(0xbdae93),
            Gruvbox::Light4       => Color::from_u32(0xa89984),
            Gruvbox::RedBright    => Color::from_u32(0xfb4934),
            Gruvbox::GreenBright  => Color::from_u32(0xb8bb26),
            Gruvbox::YellowBright => Color::from_u32(0xfabd2f),
            Gruvbox::BlueBright   => Color::from_u32(0x83a598),
            Gruvbox::PurpleBright => Color::from_u32(0xd3869b),
            Gruvbox::AquaBright   => Color::from_u32(0x8ec07c),
            Gruvbox::OrangeBright => Color::from_u32(0xfe8019),
            Gruvbox::Red          => Color::from_u32(0xcc241d),
            Gruvbox::Green        => Color::from_u32(0x98971a),
            Gruvbox::Yellow       => Color::from_u32(0xd79921),
            Gruvbox::Blue         => Color::from_u32(0x458588),
            Gruvbox::Purple       => Color::from_u32(0xb16286),
            Gruvbox::Aqua         => Color::from_u32(0x689d6a),
            Gruvbox::Orange       => Color::from_u32(0xd65d0e),
            Gruvbox::RedDim       => Color::from_u32(0x9d0006),
            Gruvbox::GreenDim     => Color::from_u32(0x79740e),
            Gruvbox::YellowDim    => Color::from_u32(0xb57614),
            Gruvbox::BlueDim      => Color::from_u32(0x076678),
            Gruvbox::PurpleDim    => Color::from_u32(0x8f3f71),
            Gruvbox::AquaDim      => Color::from_u32(0x427b58),
            Gruvbox::OrangeDim    => Color::from_u32(0xaf3a03),
        }
    }
}

impl From<Gruvbox> for Color {
    fn from(val: Gruvbox) -> Color { val.color() }
}
