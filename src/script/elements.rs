use ratatui::style::Color;
use crate::{Effect, EffectTimer};

pub enum Element {
    Color(Color),
    U32(u32),
    F32(f32),
    Timer(EffectTimer),
    Fx { name: String, parameters: Box<Element> }
}

pub enum ParseError {
    InvalidElement,
    InvalidFxParameters,
    InvalidFxName(String),
}

mod parse {
    use anpa::combinators::{attempt, many, middle, no_separator, succeed};
    use anpa::core::{ParserExt, StrParser};
    use anpa::parsers::{item_if, item_while, skip, take, until};
    use anpa::{left, or, right, skip, take, tuplify, until};
    use crate::{Duration, EffectTimer, Interpolation};

    fn effect_timer<'a>() -> impl StrParser<'a, EffectTimer> {
        let parse_u32 = many(item_if(|c: char| c.is_ascii_digit()), false, no_separator())
            .map(|s: &str| s.parse().unwrap());

        // raw int (linear)
        let from_u32 = parse_u32
            .map(|v| EffectTimer::from_ms(v, Interpolation::Linear));

        // tuple (u32, interpolation)
        let from_tuple = tuplify!(
            right!(skip!('('), parse_u32),
            middle(skip!(", "), interpolation(), skip!(')')),
        ).map(|(ms, interpolation)| EffectTimer::from_ms(ms, interpolation));

        // EffectTimer::new(duration, interpolation)
        let from_new = right!(
            skip!("EffectTimer::new"),
            tuplify!(
                right!(skip!('('), duration()),
                middle(skip!(", "), interpolation(), skip!(')')),
            ),
        ).map(|(duration, interpolation)| EffectTimer::new(duration, interpolation));

        // EffectTimer::from_ms(u32, Interpolation)
        let from_ms = tuplify!(
            right!(skip!("EffectTimer::from_ms("), parse_u32),
            middle(skip!(", "), interpolation(), skip!(')')),
        ).map(|(ms, interpolation)| EffectTimer::from_ms(ms, interpolation));

        or!(
            from_u32,
            from_tuple,
            from_new,
            from_ms,
        )
    }

    fn duration<'a>() -> impl StrParser<'a, Duration> {
        // ctor from_millis
        let from_millis = middle(
            skip!("Duration::from_millis("),
            item_while(|c| c != ')' && c != ' '),
            skip!(')'),
        ).map(|s: &str| Duration::from_millis(s.parse().unwrap()));

        // ctor from_secs_f32
        let from_secs = middle(
            skip!("Duration::from_secs_f32("),
            item_while(|c| c != ')' && c != ' '),
            skip!(')'),
        ).map(|s: &str| Duration::from_secs_f32(s.parse().unwrap()));

        or!(from_millis, from_secs)
    }

    fn interpolation<'a>() -> impl StrParser<'a, Interpolation> {
        right!(
            succeed(attempt(skip!("Interpolation::"))),
            item_while(|c: char| c.is_ascii_alphabetic()),
        ).map_if(|s: &str| match s {
            "Linear"       => Some(Interpolation::Linear),
            "Reverse"      => Some(Interpolation::Reverse),
            "BackIn"       => Some(Interpolation::BackIn),
            "BackOut"      => Some(Interpolation::BackOut),
            "BackInOut"    => Some(Interpolation::BackInOut),
            "BounceIn"     => Some(Interpolation::BounceIn),
            "BounceOut"    => Some(Interpolation::BounceOut),
            "BounceInOut"  => Some(Interpolation::BounceInOut),
            "CircIn"       => Some(Interpolation::CircIn),
            "CircOut"      => Some(Interpolation::CircOut),
            "CircInOut"    => Some(Interpolation::CircInOut),
            "CubicIn"      => Some(Interpolation::CubicIn),
            "CubicOut"     => Some(Interpolation::CubicOut),
            "CubicInOut"   => Some(Interpolation::CubicInOut),
            "ElasticIn"    => Some(Interpolation::ElasticIn),
            "ElasticOut"   => Some(Interpolation::ElasticOut),
            "ElasticInOut" => Some(Interpolation::ElasticInOut),
            "ExpoIn"       => Some(Interpolation::ExpoIn),
            "ExpoOut"      => Some(Interpolation::ExpoOut),
            "ExpoInOut"    => Some(Interpolation::ExpoInOut),
            "QuadIn"       => Some(Interpolation::QuadIn),
            "QuadOut"      => Some(Interpolation::QuadOut),
            "QuadInOut"    => Some(Interpolation::QuadInOut),
            "QuartIn"      => Some(Interpolation::QuartIn),
            "QuartOut"     => Some(Interpolation::QuartOut),
            "QuartInOut"   => Some(Interpolation::QuartInOut),
            "QuintIn"      => Some(Interpolation::QuintIn),
            "QuintOut"     => Some(Interpolation::QuintOut),
            "QuintInOut"   => Some(Interpolation::QuintInOut),
            "SineIn"       => Some(Interpolation::SineIn),
            "SineOut"      => Some(Interpolation::SineOut),
            "SineInOut"    => Some(Interpolation::SineInOut),
            _              => None
        })
    }

    #[cfg(test)]
    mod tests {
        use anpa::core::{parse, AnpaResult, StrParser};
        use crate::{Duration, EffectTimer, Interpolation};

        fn assert_parser_eq<T: PartialEq + std::fmt::Debug>(
            result: AnpaResult<&str, T>,
            expected: T
        ) {
            assert_eq!(result.state, "", "Expected parser to consume the entire input");
            assert_eq!(result.result, Some(expected));
        }

        #[test]
        fn test_duration() {
            let input = "Duration::from_millis(1000)";
            assert_parser_eq(
                parse(super::duration(), input),
                Duration::from_millis(1000)
            );

            let input = "Duration::from_secs_f32(0.5)";
            assert_parser_eq(
                parse(super::duration(), input),
                Duration::from_secs_f32(0.5)
            );
        }

        #[test]
        fn parse_interpolation() {
            [
                ("BackIn",                      Interpolation::BackIn),
                ("BackOut",                     Interpolation::BackOut),
                ("BackInOut",                   Interpolation::BackInOut),
                ("BounceIn",                    Interpolation::BounceIn),
                ("BounceOut",                   Interpolation::BounceOut),
                ("BounceInOut",                 Interpolation::BounceInOut),
                ("CircIn",                      Interpolation::CircIn),
                ("CircOut",                     Interpolation::CircOut),
                ("CircInOut",                   Interpolation::CircInOut),
                ("CubicIn",                     Interpolation::CubicIn),
                ("CubicOut",                    Interpolation::CubicOut),
                ("Interpolation::CubicInOut",   Interpolation::CubicInOut),
                ("Interpolation::ElasticIn",    Interpolation::ElasticIn),
                ("Interpolation::ElasticOut",   Interpolation::ElasticOut),
                ("Interpolation::ElasticInOut", Interpolation::ElasticInOut),
                ("Interpolation::ExpoIn",       Interpolation::ExpoIn),
                ("Interpolation::ExpoOut",      Interpolation::ExpoOut),
                ("Interpolation::ExpoInOut",    Interpolation::ExpoInOut),
                ("Interpolation::QuadIn",       Interpolation::QuadIn),
                ("Interpolation::QuadOut",      Interpolation::QuadOut),
                ("Interpolation::QuadInOut",    Interpolation::QuadInOut),
                ("Interpolation::QuartIn",      Interpolation::QuartIn),
                ("Interpolation::QuartOut",     Interpolation::QuartOut),
                ("Interpolation::QuartInOut",   Interpolation::QuartInOut),
                ("Interpolation::QuintIn",      Interpolation::QuintIn),
                ("Interpolation::QuintOut",     Interpolation::QuintOut),
                ("Interpolation::QuintInOut",   Interpolation::QuintInOut),
                ("Interpolation::SineIn",       Interpolation::SineIn),
                ("Interpolation::SineOut",      Interpolation::SineOut),
                ("Interpolation::SineInOut",    Interpolation::SineInOut),
            ].into_iter().for_each(|(input, expected)| {
                assert_parser_eq(
                    parse(super::interpolation(), input),
                    expected
                );
            });
        }

        #[test]
        fn test_effect_timer() {
            let input = "EffectTimer::from_ms(1000, Interpolation::Linear)";
            assert_parser_eq(
                parse(super::effect_timer(), input),
                EffectTimer::from_ms(1000, Interpolation::Linear)
            );

            let input = "EffectTimer::new(Duration::from_millis(1000), Linear)";
            assert_parser_eq(
                parse(super::effect_timer(), input),
                EffectTimer::new(Duration::from_millis(1000), Interpolation::Linear)
            );

            let input = "EffectTimer::new(Duration::from_secs_f32(0.5), Linear)";
            assert_parser_eq(
                parse(super::effect_timer(), input),
                EffectTimer::new(Duration::from_secs_f32(0.5), Interpolation::Linear)
            );

            let input = "(1337, Reverse)";
            assert_parser_eq(
                parse(super::effect_timer(), input),
                EffectTimer::from_ms(1337, Interpolation::Reverse)
            );

            let input = "1234";
            assert_parser_eq(
                parse(super::effect_timer(), input),
                EffectTimer::from_ms(1234, Interpolation::Linear)
            );
        }
    }
}

pub fn load_script(
    source: &str,
) -> Result<Effect, ParseError> {
    unimplemented!()
}

