use std::any::Any;
use std::collections::BTreeMap;
use ratatui::style::Color;
use crate::{CellFilter, Duration, Effect, EffectTimer};

#[derive(Clone, Debug, PartialEq)]
pub enum FxArg {
    Color(Color),
    String(String),
    U32(u32), // can also repr EffectTimer and Duration
    F32(f32),
    Duration(Duration),
    Timer(EffectTimer),
    ArrayRef(Vec<FxArg>),
    Fx { name: String, parameters: Vec<FxArg> }
}

pub enum ScriptError {
    InvalidElement,
    InvalidFxParameters,
    InvalidFxName(String),
}


mod parse {
    use anpa::combinators::{attempt, many, many_to_vec, middle, no_separator, or_diff, right, separator, succeed, times};
    use anpa::core::{Parser, ParserExt, StrParser};
    use anpa::parsers::{item_if, item_while, skip, take, until};
    use anpa::{defer_parser, greedy_or, left, or, right, skip, take, tuplify, until};
    use anpa::number::float;
    use anpa::prefix::Prefix;
    use anpa::slicelike::SliceLike;
    use anpa::whitespace::skip_whitespace;
    use ratatui::layout::{Margin, Rect};
    use crate::{Duration, EffectTimer, Interpolation, Motion};
    use crate::script::parser::FxArg;

    fn trim<'a>(prefix: &str) -> impl StrParser<'a, ()> + use<'a, '_>{
        right!(
            skip_whitespace(),
            skip!(prefix),
            skip_whitespace()
        )
    }

    fn fx_statement<'a>() -> impl StrParser<'a, FxArg> {
        let name = right!(
            succeed(attempt(skip!("fx::"))),
            item_while(|c: char| c.is_ascii_alphabetic() || c == '_'),
        );

        let parameters = middle(trim("("), arguments(), trim(")"));

        tuplify!(name, parameters).map(|(name, parameters)|
            FxArg::Fx {
                name: name.to_string(),
                parameters
            }
        )
    }

    fn unescaped_string<'a>() -> impl StrParser<'a, String> {
        let unicode = right(skip!('u'), times(4, item_if(|c: char| c.is_ascii_hexdigit())));
        let escaped = right(skip!('\\'), or_diff(unicode, item_if(|c: char| "\"\\/bfnrt".contains(c))));
        let valid_char = item_if(|c: char| c != '"' && c != '\\' && !c.is_control());
        let not_end = or_diff(valid_char, escaped);

        middle(skip!('"'), many(not_end, true, no_separator()), skip!('"'))
            .map(|s: &str| s.to_string())
            .map(|s: String| s.replace("\\\"", "\""))
    }

    fn array_ref<'a>() -> impl StrParser<'a, FxArg> {
        middle(
            trim("&["),
            many_to_vec(argument(), true, separator(trim(","), false)),
            trim("]")
        ).map(FxArg::ArrayRef)
    }

    fn argument<'a>() -> impl StrParser<'a, FxArg> {
        // must defer to avoid recursive opaqueness
        defer_parser! {
            // `parse_f32` must come after `parse_u32` due to how float() is
            // implemented, as such we use greedy_or to ensure that `parse_u32`
            // isn't chosen over `parse_f32`.
            greedy_or!(
                unescaped_string().map(FxArg::String),
                parse_u32().map(FxArg::U32),
                parse_f32().map(FxArg::F32),
                effect_timer().map(FxArg::Timer),
                duration().map(FxArg::Duration),
                array_ref(), // e.g. &[fx1, fx2, fx3]
                fx_statement(),
            )
        }
    }

    fn arguments<'a>() -> impl StrParser<'a, Vec<FxArg>> {
        many_to_vec(argument(), true, separator(trim(","), false))
    }

    fn parse_u32<'a>() -> impl StrParser<'a, u32> {
        many(item_if(|c: char| c.is_ascii_digit()), false, no_separator())
            .map(|s: &str| s.parse().unwrap())
    }

    fn parse_u16<'a>() -> impl StrParser<'a, u16> {
        many(item_if(|c: char| c.is_ascii_digit()), false, no_separator())
            .map(|s: &str| s.parse().unwrap())
    }

    fn parse_f32<'a>() -> impl StrParser<'a, f32> {
        float()
    }

    fn effect_timer<'a>() -> impl StrParser<'a, EffectTimer> {
        // raw int: ms with linear interpolation
        let from_u32 = parse_u32()
            .map(|v| EffectTimer::from_ms(v, Interpolation::Linear));

        let into_duration = or!(duration(), parse_u32().map(|ms| Duration::from_millis(ms as _)));

        // tuple: (u32, interpolation)
        let from_tuple = tuplify!(
            right!(trim("("), into_duration),
            middle(trim(","), interpolation(), trim(")")),
        ).map(|(duration, interpolation)| EffectTimer::new(duration, interpolation));

        // ctor: EffectTimer::new(duration, interpolation)
        let from_new = middle(
            trim("EffectTimer::new("),
            tuplify!(
                duration(),
                right!(trim(","), interpolation()),
            ),
            trim(")")
        ).map(|(duration, interpolation)| EffectTimer::new(duration, interpolation));

        // from ms: EffectTimer::from_ms(u32, Interpolation)
        let from_ms = tuplify!(
            right!(trim("EffectTimer::from_ms("), parse_u32()),
            middle(trim(","), interpolation(), trim(")")),
        ).map(|(ms, interpolation)| EffectTimer::from_ms(ms, interpolation));

        or!(
            from_u32,
            from_tuple,
            from_new,
            from_ms,
        )
    }

    fn rect<'a>() -> impl StrParser<'a, Rect> {
        let new = middle(
            trim("Rect::new("),
            tuplify!(
                parse_u16(),
                right!(trim(","), parse_u16()),
                right!(trim(","), parse_u16()),
                right!(trim(","), parse_u16()),
            ),
            trim(")")
        ).map(|(x, y, w, h)| Rect::new(x, y, w, h));

        let raw = middle(
            right!(trim("Rect"), trim("{")),
            tuplify!(
                middle(trim("x:"), parse_u16(), trim(",")),
                middle(trim("y:"), parse_u16(), trim(",")),
                middle(trim("width:"), parse_u16(), trim(",")),
                right!(trim("height:"), parse_u16()),
            ),
            trim("}")
        ).map(|(x, y, w, h)| Rect::new(x, y, w, h));

        or!(new, raw)
    }

    fn margin<'a>() -> impl StrParser<'a, Margin> {
        // ctor: Margin::new(u32, u32)
        let new = middle(
            trim("Margin::new("),
            tuplify!(
                parse_u16(),
                right!(trim(","), parse_u16())
            ),
            trim(")")
        ).map(|(x, y)| Margin::new(x, y));

        // ctor: Margin::new(u32)
        let construct = middle(
            right!(trim("Margin"), trim("{")),
            tuplify!(
                middle(trim("horizontal:"), parse_u16(), trim(",")),
                right!(trim("vertical:"), parse_u16()),
            ),
            trim("}")
        ).map(|(horizontal, vertical)| Margin { horizontal, vertical });

        or!(new, construct)
    }

    fn duration<'a>() -> impl StrParser<'a, Duration> {
        // ctor from_millis
        let from_millis = middle(
            trim("Duration::from_millis("),
            parse_u32(),
            trim(")"),
        ).map(|ms| Duration::from_millis(ms as _));

        // ctor from_secs_f32
        let from_secs = middle(
            trim("Duration::from_secs_f32("),
            float(),
            trim(")"),
        ).map(|seconds| Duration::from_secs_f32(seconds));

        or!(from_millis, from_secs)
    }

    fn motion<'a>() -> impl StrParser<'a, Motion> {
        right!(
            succeed(attempt(skip!("Motion::"))),
            item_while(|c: char| c.is_ascii_alphabetic()),
        ).map_if(|s: &str| match s {
            "DownToUp"    => Some(Motion::DownToUp),
            "UpToDown"    => Some(Motion::UpToDown),
            "LeftToRight" => Some(Motion::LeftToRight),
            "RightToLeft" => Some(Motion::RightToLeft),
            _             => None,
        })
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

    // fn comment<'a>() -> impl StrParser<'a, ()> {
    //     right!(skip!("//"), until(skip!('\n')), skip_whitespace())
    // }

    #[cfg(test)]
    mod tests {
        use anpa::core::{parse, AnpaResult, StrParser};
        use ratatui::layout::{Margin, Rect};
        use crate::{Duration, EffectTimer, Interpolation, Motion};
        use crate::script::parser::FxArg;

        fn assert_parser_eq<T: PartialEq + std::fmt::Debug>(
            result: AnpaResult<&str, T>,
            expected: T
        ) {
            assert_eq!(result.state, "", "Expected parser to consume the entire input");
            assert_eq!(result.result, Some(expected));
        }

        #[test]
        fn skip_trim() {
            let input = "  Hello  ";
            assert_parser_eq(
                parse(super::trim("Hello"), input),
                ()
            );
        }

        #[test]
        fn test_margin() {
            let input = "Margin::new(10, 20)";
            assert_parser_eq(
                parse(super::margin(), input),
                Margin::new(10, 20)
            );

            let input = r#"Margin {
                horizontal: 10,
                vertical: 20
            }"#;
            assert_parser_eq(
                parse(super::margin(), input),
                Margin::new(10, 20)
            );
        }

        #[test]
        fn test_rect() {
            let input = "Rect::new(10, 20, 30, 40)";
            assert_parser_eq(
                parse(super::rect(), input),
                Rect::new(10, 20, 30, 40)
            );

            let input = r#"Rect {
                x: 10,
                y: 20,
                width: 30,
                height: 40
            }"#;
            assert_parser_eq(
                parse(super::rect(), input),
                Rect::new(10, 20, 30, 40)
            );
        }

        #[test]
        fn test_motion() {
            let input = "Motion::DownToUp";
            assert_parser_eq(
                parse(super::motion(), input),
                Motion::DownToUp
            );

            let input = "UpToDown";
            assert_parser_eq(
                parse(super::motion(), input),
                Motion::UpToDown
            );

            let input = "LeftToRight";
            assert_parser_eq(
                parse(super::motion(), input),
                Motion::LeftToRight
            );

            let input = "RightToLeft";
            assert_parser_eq(
                parse(super::motion(), input),
                Motion::RightToLeft
            );
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

            let input = r#"Duration::from_millis(
                               321
                           )"#;
            assert_parser_eq(
                parse(super::duration(), input),
                Duration::from_millis(321)
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
        fn parse_array_ref() {
            let input = "&[\"Hello, World!\", 1337, 3.14, (1000, SineIn)]";
            assert_parser_eq(
                parse(super::array_ref(), input),
                FxArg::ArrayRef(vec![
                    FxArg::String("Hello, World!".to_string()),
                    FxArg::U32(1337),
                    FxArg::F32(3.14),
                    FxArg::Timer(EffectTimer::from_ms(1000, Interpolation::SineIn))
                ])
            );
        }

        #[test]
        fn parse_effect_timer() {
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

            let input = "(Duration::from_millis(1337), Reverse)";
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

        #[test]
        fn parse_string() {
            let input = "\"Hello, World!\"";
            assert_parser_eq(
                parse(super::unescaped_string(), input),
                "Hello, World!".to_string()
            );

            // let input = r#""Hello, \"World!\"""#;
            let input = "\"Hello, \\\"World!\\\"\"";
            assert_parser_eq(
                parse(super::unescaped_string(), input),
                "Hello, \"World!\"".to_string()
            );
        }

        #[test]
        fn parse_parameter() {
            let input = "\"Hello, World!\"";
            assert_parser_eq(
                parse(super::argument(), input),
                FxArg::String("Hello, World!".to_string())
            );

            let input = "1337";
            assert_parser_eq(
                parse(super::argument(), input),
                FxArg::U32(1337)
            );

            let input = "3.14";
            assert_parser_eq(
                parse(super::argument(), input),
                FxArg::F32(3.14)
            );

            let input = "EffectTimer::from_ms(1000, Interpolation::Linear)";
            assert_parser_eq(
                parse(super::argument(), input),
                FxArg::Timer(EffectTimer::from_ms(1000, Interpolation::Linear))
            );

            let input = "Duration::from_millis(1000)";
            assert_parser_eq(
                parse(super::argument(), input),
                FxArg::Duration(Duration::from_millis(1000))
            );
        }

        #[test]
        fn parse_parameters() {
            let input = "\"Hello, World!\", 1337, 3.14, (1000, SineIn)";
            assert_parser_eq(
                parse(super::arguments(), input),
                vec![
                    FxArg::String("Hello, World!".to_string()),
                    FxArg::U32(1337),
                    FxArg::F32(3.14),
                    FxArg::Timer(EffectTimer::from_ms(1000, Interpolation::SineIn))
                ]
            );
        }

        #[test]
        fn parse_fx_statement() {
            let input = "coalesce(Duration::from_millis(220))";
            assert_parser_eq(
                parse(super::fx_statement(), input),
                FxArg::Fx {
                    name: "coalesce".to_string(),
                    parameters: vec![
                        FxArg::Duration(Duration::from_millis(220)),
                    ]
                }
            );

            let input = "fx::dissolve((Duration::from_millis(220), ElasticOut))";
            assert_parser_eq(
                parse(super::fx_statement(), input),
                FxArg::Fx {
                    name: "dissolve".to_string(),
                    parameters: vec![
                        FxArg::Timer(EffectTimer::from_ms(220, Interpolation::ElasticOut)),
                    ]
                }
            );

            let input = "fx::ping_pong(fx::coalesce((500, CircOut)))";
            assert_parser_eq(
                parse(super::fx_statement(), input),
                FxArg::Fx {
                    name: "ping_pong".to_string(),
                    parameters: vec![
                        FxArg::Fx {
                            name: "coalesce".to_string(),
                            parameters: vec![
                                FxArg::Timer(EffectTimer::from_ms(500, Interpolation::CircOut)),
                            ]
                        }
                    ]
                }
            );
        }
    }
}

pub fn load_script(
    source: &str,
) -> Result<Effect, ScriptError> {
    unimplemented!()
}

