use anpa::core::parse;
use anpa::combinators::{attempt, many, many_to_vec, middle, no_separator, or_diff, right, separator, succeed, times};
use anpa::core::{ParserExt, StrParser};
use anpa::number::float;
use anpa::parsers::{item_if, item_while};
use anpa::whitespace::skip_whitespace;
use anpa::{defer_parser, greedy_or, or, right, skip, tuplify};
use ratatui::layout::{Margin, Rect};
use ratatui::style::Color;
use crate::{CellFilter, Duration, EffectTimer, Interpolation, Motion};
use crate::dsl::expressions::{Expr, FnCall, Value};
use crate::fx::RepeatMode;

// fixme: parsers should always return Expr instead of concrete types,
// so that we can handle errors more and support variables

pub(super) fn parse_expr(
    input: &str,
) -> Option<Expr> {
    parse(fx_statement(), input)
        .result
}


fn trim<'a>(prefix: &str) -> impl StrParser<'a, ()> + use<'a, '_>{
    right!(
        skip_whitespace(),
        skip!(prefix),
        skip_whitespace()
    )
}

fn fx_statement<'a>() -> impl StrParser<'a, Expr> {
    let name = right!(
        succeed(attempt(skip!("fx::"))),
        item_while(|c: char| c.is_ascii_alphabetic() || c == '_'),
    );

    let args = middle(trim("("), arguments(), trim(")"));

    tuplify!(name, args).map(|(name, arguments)|
        Expr::Fx {
            name: name.to_string(),
            arguments
        }
    )
}

fn string_literal<'a>() -> impl StrParser<'a, Expr> {
    let unicode = right(skip!('u'), times(4, item_if(|c: char| c.is_ascii_hexdigit())));
    let escaped = right(skip!('\\'), or_diff(unicode, item_if(|c: char| "\"\\/bfnrt".contains(c))));
    let valid_char = item_if(|c: char| c != '"' && c != '\\' && !c.is_control());
    let not_end = or_diff(valid_char, escaped);

    middle(skip!('"'), many(not_end, true, no_separator()), skip!('"'))
        .map(|s: &str| s.to_string())
        .map(|s: String| s.replace("\\\"", "\""))
        .map(Value::String)
        .map(Expr::Literal)
}

fn array_ref<'a>() -> impl StrParser<'a, Expr> {
    middle(
        trim("&["),
        many_to_vec(argument(), true, separator(trim(","), false)),
        trim("]")
    ).map(Expr::ArrayRef)
}

fn var<'a>() -> impl StrParser<'a, Expr> {
    item_while(|c: char| matches!(c, 'a'..='z' | '0'..='9' | '_'))
        .map(|s: &str| s.to_string())
        .map(Expr::Var)
}

fn cell_filter<'a>() -> impl StrParser<'a, Expr> {
    // cell id filter
    let cf = |s| right!(
        skip_whitespace(),
        succeed(attempt(skip!("CellFilter::"))),
        skip!(s)
    );

    // Basic filters
    let all = cf("All").map(|_| Expr::Literal(Value::CellFilter(CellFilter::All)));
    let text = cf("Text").map(|_| Expr::Literal(Value::CellFilter(CellFilter::Text)));

    // Color filters
    let fg_color = middle(cf("FgColor("), color(), trim(")"))
        .map(|color| Expr::CellFilter {
            filter_type: "FgColor",
            arguments: vec![color]
        });
    let bg_color = middle(cf("BgColor("), color(), trim(")"))
        .map(|color| Expr::CellFilter {
            filter_type: "BgColor",
            arguments: vec![color]
        });

    // Margin-based filters
    let inner = middle(cf("Inner("), margin(), trim(")"))
        .map(|margin| Expr::CellFilter {
            filter_type: "Inner",
            arguments: vec![margin]
        });
    let outer = middle(cf("Outer("), margin(), trim(")"))
        .map(|margin| Expr::CellFilter {
            filter_type: "Outer",
            arguments: vec![margin]
        });

    // Layout filter
    // let layout = tuplify!(
    //     right!(trim("Layout("), parse_layout()),
    //     right!(trim(","), parse_u16(), trim(")")),
    // ).map(|(layout, idx)| CellFilter::Layout(layout, idx));

    // Compound filters
    let all_of = middle(
        cf("AllOf(vec!["),
        many_to_vec(defer_parser!(cell_filter()), true, separator(trim(","), false)),
        trim("])")
    ).map(|filters| Expr::CellFilter {
        filter_type: "AllOf",
        arguments: filters
    });

    let any_of = middle(
        cf("AnyOf(vec!["),
        many_to_vec(defer_parser!(cell_filter()), true, separator(trim(","), false)),
        trim("])")
    ).map(|filters| Expr::CellFilter {
        filter_type: "AnyOf",
        arguments: filters
    });

    let none_of = middle(
        cf("NoneOf(vec!["),
        many_to_vec(defer_parser!(cell_filter()), true, separator(trim(","), false)),
        trim("])")
    ).map(|filters| Expr::CellFilter {
        filter_type: "NoneOf",
        arguments: filters
    });

    let not = middle(
        cf("Not(Box::new("),
        defer_parser!(cell_filter()),
        trim("))")
    ).map(|filter| Expr::CellFilter {
        filter_type: "Not",
        arguments: vec![filter]
    });

    or!(
        fg_color,
        bg_color,
        inner,
        outer,
        // layout,
        all_of,
        any_of,
        none_of,
        not,
        all,
        text,
    )
}

fn argument<'a>() -> impl StrParser<'a, Expr> {
    // must defer to avoid recursive opaqueness
    defer_parser! {
        // `parse_f32` must come after `parse_u32` due to how float() is
        // implemented, as such we use greedy_or to ensure that `parse_u32`
        // isn't chosen over `parse_f32`.
        greedy_or!(
            string_literal(),
            parse_u32(),
            parse_f32(),
            effect_timer(),
            duration(),
            motion(),
            rect(),
            margin(),
            color(),
            repeat_mode(), // used by fx::repeat
            array_ref(), // e.g. &[fx1, fx2, fx3]
            fx_statement(),
            var(),
        )
    }
}

fn arguments<'a>() -> impl StrParser<'a, Vec<Expr>> {
    many_to_vec(argument(), true, separator(trim(","), false))
}

fn parse_u32<'a>() -> impl StrParser<'a, Expr> {
    let plain = many(item_if(|c: char| c.is_ascii_digit()), false, no_separator())
        .map(|s: &str| s.parse().unwrap());

    let hexadecimal = right!(
        skip!("0x"),
        item_while(|c: char| c.is_ascii_hexdigit())
    ).map_if(|s: &str| {
        match s.len() {
            6 => u32::from_str_radix(s, 16).ok(),  // rrggbb
            8 => u32::from_str_radix(s, 16).ok(),  // aarrggbb
            _ => None
        }
    });

    or!(
        or!(hexadecimal, plain).map(|v| Expr::Literal(Value::U32(v))),
        var()
    )
}

fn parse_u16<'a>() -> impl StrParser<'a, Expr> {
    let literal = many(item_if(|c: char| c.is_ascii_digit()), false, no_separator())
        .map(|s: &str| s.parse::<u16>().unwrap())
        .map(|v| Expr::Literal(Value::U16(v)));

    or!(literal, var())
}

fn parse_f32<'a>() -> impl StrParser<'a, Expr> {
    or!(
        float().map(|f| Expr::Literal(Value::F32(f))),
        var()
    )
}

fn repeat_mode<'a>() -> impl StrParser<'a, Expr> {
    let forever = skip!("RepeatMode::Forever")
        .map(|_| Expr::Literal(Value::RepeatMode(RepeatMode::Forever)));

    let times = middle(
        trim("RepeatMode::Times("),
        parse_u32(),
        trim(")")
    ).map(|times| Expr::Call {
        function: FnCall::RepeatModeTimes,
        args: vec![times] // placeholder
    });

    let duration = middle(
        trim("RepeatMode::Duration("),
        duration(),
        trim(")")
    ).map(|duration| Expr::Call {
        function: FnCall::RepeatModeDuration,
        args: vec![duration] // placeholder
    });

    or!(forever, times, duration)
}

fn effect_timer<'a>() -> impl StrParser<'a, Expr> {
    // raw int: ms with linear interpolation
    let from_u32 = parse_u32()
        .map(|ms| Expr::Call {
            function: FnCall::EffectTimerFromMs,
            args: vec![ms, Expr::Literal(Value::Interpolation(Interpolation::Linear))]
        });

    let into_duration = or!(
        duration(),
        parse_u32().map(|ms| Expr::Call {
            function: FnCall::DurationFromMillis,
            args: vec![ms]
        }),
    );

    // tuple: (u32, interpolation)
    let from_tuple = tuplify!(
        right!(trim("("), into_duration),
        middle(trim(","), interpolation(), trim(")")),
    ).map(|(duration, interpolation)| Expr::Call {
        function: FnCall::EffectTimerNew,
        args: vec![duration, interpolation]
    });

    // ctor: EffectTimer::new(duration, interpolation)
    let from_new = middle(
        trim("EffectTimer::new("),
        tuplify!(
            duration(),
            right!(trim(","), interpolation()),
        ),
        trim(")")
    ).map(|(duration, interpolation)| Expr::Call {
        function: FnCall::EffectTimerNew,
        args: vec![duration, interpolation]
    });

    // from ms: EffectTimer::from_ms(u32, Interpolation)
    let from_ms = tuplify!(
        right!(trim("EffectTimer::from_ms("), parse_u32()),
        middle(trim(","), interpolation(), trim(")")),
    ).map(|(ms, interpolation)| Expr::Call {
        function: FnCall::EffectTimerFromMs,
        args: vec![ms, interpolation]
    });

    or!(
        from_tuple,
        from_new,
        from_ms,
        from_u32,
    )
}

fn rect<'a>() -> impl StrParser<'a, Expr> {
    let new = middle(
        trim("Rect::new("),
        tuplify!(
            parse_u16(),
            right!(trim(","), parse_u16()),
            right!(trim(","), parse_u16()),
            right!(trim(","), parse_u16()),
        ),
        trim(")")
    ).map(|(x, y, w, h)| Expr::Call {
        function: FnCall::RectNew,
        args: vec![x, y, w, h]
    });

    let raw = middle(
        right!(trim("Rect"), trim("{")),
        tuplify!(
            middle(trim("x:"), parse_u16(), trim(",")),
            middle(trim("y:"), parse_u16(), trim(",")),
            middle(trim("width:"), parse_u16(), trim(",")),
            right!(trim("height:"), parse_u16()),
        ),
        trim("}")
    ).map(|(x, y, w, h)| Expr::Call {
        function: FnCall::RectStruct,
        args: vec![x, y, w, h]
    });

    or!(new, raw)
}

fn margin<'a>() -> impl StrParser<'a, Expr> {
    // ctor: Margin::new(u32, u32)
    let new = middle(
        trim("Margin::new("),
        tuplify!(
            parse_u16(),
            right!(trim(","), parse_u16())
        ),
        trim(")")
    ).map(|(x, y)| Expr::Call {
        function: FnCall::MarginNew,
        args: vec![x, y]
    });

    // ctor: Margin::new(u32)
    let construct = middle(
        right!(trim("Margin"), trim("{")),
        tuplify!(
            middle(trim("horizontal:"), parse_u16(), trim(",")),
            right!(trim("vertical:"), parse_u16()),
        ),
        trim("}")
    ).map(|(horizontal, vertical)| Expr::Call {
        function: FnCall::MarginStruct,
        args: vec![horizontal, vertical]
    });

    or!(new, construct)
}

fn duration<'a>() -> impl StrParser<'a, Expr> {
    // ctor from_millis
    let from_millis = middle(
        trim("Duration::from_millis("),
        parse_u32(),
        trim(")"),
    ).map(|ms| Expr::Call {
        function: FnCall::DurationFromMillis,
        args: vec![ms]
    });

    // ctor from_secs_f32
    let from_secs = middle(
        trim("Duration::from_secs_f32("),
        parse_f32(),
        trim(")"),
    ).map(|secs| Expr::Call {
        function: FnCall::DurationFromSeconds,
        args: vec![secs]
    });

    or!(from_millis, from_secs)
}

fn motion<'a>() -> impl StrParser<'a, Expr> {
    let literal = right!(
        succeed(attempt(skip!("Motion::"))),
        item_while(|c: char| c.is_ascii_alphabetic()),
    ).map_if(|s: &str| match s {
        "DownToUp"    => Some(Motion::DownToUp),
        "UpToDown"    => Some(Motion::UpToDown),
        "LeftToRight" => Some(Motion::LeftToRight),
        "RightToLeft" => Some(Motion::RightToLeft),
        _             => None,
    }).map(|motion| Expr::Literal(Value::Motion(motion)));

    or!(literal, var())
}

fn color<'a>() -> impl StrParser<'a, Expr> {
    middle(
        trim("Color::from_u32("),
        parse_u32(),
        trim(")")
    ).map(|u32| Expr::Call {
        function: FnCall::ColorFromU32,
        args: vec![u32]
    })
}

fn interpolation<'a>() -> impl StrParser<'a, Expr> {
    let literal = right!(
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
    }).map(|interpolation| Expr::Literal(Value::Interpolation(interpolation)));

    or!(literal, var())
}

#[cfg(test)]
mod tests {
    use crate::dsl::environment::DslEnv;
    use crate::{CellFilter, Duration, EffectTimer, Interpolation, Motion};
    use anpa::core::{parse, AnpaResult};
    use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
    use ratatui::style::Color;
    use crate::dsl::expressions::{Expr, FnCall, Value};
    use crate::fx::RepeatMode;

    fn assert_parser_eq(
        result: AnpaResult<&str, Expr>,
        expected: Value
    ) {
        assert_eq!(result.state, "", "Expected parser to consume the entire input");
        assert_eq!(result.result, Some(Expr::Literal(expected)));
    }

    fn assert_cell_filter_eq(
        input: &str,
        expected: CellFilter,
    ) {
        let filter = parse(super::cell_filter(), input)
            .result
            .expect("Failed to parse cell filter");

        assert_eq!(filter.to_string(), expected.to_string());
    }

    #[test]
    fn skip_trim() {
        let input = "  Hello  ";
        let result = parse(super::trim("Hello"), input).result;
        assert_eq!(result, Some(()))

    }

    #[test]
    fn test_color() {
        let input = "Color::from_u32(0x1d2021)";
        assert_parser_eq(
            parse(super::color(), input),
            Value::Color(Color::from_u32(0x1d2021))
        );
    }

    #[test]
    fn test_margin() {
        let input = "Margin::new(10, 20)";
        assert_parser_eq(
            parse(super::margin(), input),
            Value::Margin(Margin::new(10, 20))
        );

        let input = r#"Margin {
            horizontal: 10,
            vertical: 20
        }"#;
        assert_parser_eq(
            parse(super::margin(), input),
            Value::Margin(Margin::new(10, 20))
        );
    }

    #[test]
    fn repeat_modes() {
        let input = "RepeatMode::Forever";
        assert_parser_eq(
            parse(super::repeat_mode(), input),
            Value::RepeatMode(RepeatMode::Forever)
        );

        let input = "RepeatMode::Times(10)";
        assert_parser_eq(
            parse(super::repeat_mode(), input),
            Value::RepeatMode(RepeatMode::Times(10))
        );

        let input = "RepeatMode::Duration(Duration::from_millis(1000))";
        assert_parser_eq(
            parse(super::repeat_mode(), input),
            Value::RepeatMode(RepeatMode::Duration(Duration::from_millis(1000)))
        );
    }

    #[test]
    fn test_basic_filters() {
        // Test All filter
        assert_cell_filter_eq("All", CellFilter::All);

        // Test Text filter
        assert_cell_filter_eq("Text", CellFilter::Text);
    }

    #[test]
    fn test_color_filters() {
        // Test FgColor
        assert_cell_filter_eq(
            "FgColor(Color::from_u32(0xFF0000))",
            CellFilter::FgColor(Color::Rgb(255, 0, 0))
        );

        // Test BgColor
        assert_cell_filter_eq(
            "BgColor(Color::from_u32(0x00FF00))",
            CellFilter::BgColor(Color::Rgb(0, 255, 0))
        );
    }

    #[test]
    fn test_margin_filters() {
        // Test Inner margin
        assert_cell_filter_eq(
            "Inner(Margin::new(1, 2))",
            CellFilter::Inner(Margin::new(1, 2))
        );

        // Test Outer margin
        assert_cell_filter_eq(
            "Outer(Margin::new(3, 4))",
            CellFilter::Outer(Margin::new(3, 4))
        );
    }

    #[test]
    fn test_layout_filter() {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)]);

        assert_cell_filter_eq(
            "Layout(Layout::vertical([Length(1), Min(0)]), 1)",
            CellFilter::Layout(layout, 1)
        );
    }

    #[test]
    fn test_compound_filters() {
        // Test AllOf
        assert_cell_filter_eq(
            "AllOf(vec![Text, Inner(Margin::new(1, 1))])",
            CellFilter::AllOf(vec![
                CellFilter::Text,
                CellFilter::Inner(Margin::new(1, 1))
            ])
        );

        // Test AnyOf
        assert_cell_filter_eq(
            "AnyOf(vec![Text, Outer(Margin::new(1, 1))])",
            CellFilter::AnyOf(vec![
                CellFilter::Text,
                CellFilter::Outer(Margin::new(1, 1))
            ])
        );

        // Test NoneOf
        assert_cell_filter_eq(
            "NoneOf(vec![Text, Inner(Margin::new(1, 1))])",
            CellFilter::NoneOf(vec![
                CellFilter::Text,
                CellFilter::Inner(Margin::new(1, 1))
            ])
        );
    }

    #[test]
    fn test_not_filter() {
        assert_cell_filter_eq(
            "CellFilter::Not(Box::new(CellFilter::Text))",
            CellFilter::Not(Box::new(CellFilter::Text))
        );
    }

    #[test]
    fn test_nested_filters() {
        assert_cell_filter_eq(
            "AllOf(vec![
                Not(Box::new(Text)),
                AnyOf(vec![
                    Inner(Margin::new(1, 1)),
                    Outer(Margin::new(2, 2))
                ])
            ])",
            CellFilter::AllOf(vec![
                CellFilter::Not(Box::new(CellFilter::Text)),
                CellFilter::AnyOf(vec![
                    CellFilter::Inner(Margin::new(1, 1)),
                    CellFilter::Outer(Margin::new(2, 2))
                ])
            ])
        );
    }

    #[test]
    fn test_rect() {
        let input = "Rect::new(10, 20, 30, 40)";
        assert_parser_eq(
            parse(super::rect(), input),
            Value::Rect(Rect::new(10, 20, 30, 40))
        );

        let input = r#"Rect {
            x: 10,
            y: 20,
            width: 30,
            height: 40
        }"#;
        assert_parser_eq(
            parse(super::rect(), input),
            Value::Rect(Rect::new(10, 20, 30, 40))
        );
    }

    #[test]
    fn test_motion() {
        let input = "Motion::DownToUp";
        assert_parser_eq(
            parse(super::motion(), input),
            Value::Motion(Motion::DownToUp)
        );

        let input = "UpToDown";
        assert_parser_eq(
            parse(super::motion(), input),
            Value::Motion(Motion::UpToDown)
        );

        let input = "LeftToRight";
        assert_parser_eq(
            parse(super::motion(), input),
            Value::Motion(Motion::LeftToRight)
        );

        let input = "RightToLeft";
        assert_parser_eq(
            parse(super::motion(), input),
            Value::Motion(Motion::RightToLeft)
        );
    }

    #[test]
    fn test_duration() {
        let input = "Duration::from_millis(1000)";
        let result = parse(super::duration(), input).result.unwrap();
        let expected = call_expr(FnCall::DurationFromMillis, &[literal(Value::U32(1000))]);
        assert_eq!(result, expected);

        let input = "Duration::from_secs_f32(0.5)";
        let result = parse(super::duration(), input).result.unwrap();
        let expected = call_expr(FnCall::DurationFromSeconds, &[literal(Value::F32(0.5))]);
        assert_eq!(result, expected);

        let input = r#"Duration::from_millis(
                           321
                       )"#;
        let result = parse(super::duration(), input).result.unwrap();
        let expected = call_expr(FnCall::DurationFromMillis, &[literal(Value::U32(321))]);
        assert_eq!(result, expected);
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
                Value::Interpolation(expected)
            );
        });
    }

    #[test]
    fn parse_var() {
        let input = "my_var";
        let result = parse(super::var(), input).result;
        assert_eq!(result, Some(Expr::Var("my_var".to_string())));
    }

    // #[test]
    // fn parse_array_ref() {
    //     let input = "&[\"Hello, World!\", 1337, 3.14, (1000, SineIn)]";
    //     assert_parser_eq(
    //         parse(super::array_ref(), input),
    //         Expr::ArrayRef(vec![
    //             Expr::String("Hello, World!".to_string()),
    //             Expr::U32(1337),
    //             Expr::F32(3.14),
    //             Expr::Timer(EffectTimer::from_ms(1000, Interpolation::SineIn))
    //         ])
    //     );
    // }

    fn call_expr(function: FnCall, args: &[Expr]) -> Expr {
        Expr::Call { function, args: args.into() }
    }

    fn literal(value: Value) -> Expr {
        Expr::Literal(value)
    }

    #[test]
    fn parse_effect_timer() {
        let input = "EffectTimer::from_ms(1000, Interpolation::Linear)";
        let result = parse(super::effect_timer(), input).result.unwrap();
        let expected = call_expr(FnCall::EffectTimerFromMs, &[
            literal(Value::U32(1000)),
            literal(Value::Interpolation(Interpolation::Linear))
        ]);
        assert_eq!(result, expected);

        let input = "EffectTimer::new(Duration::from_millis(1000), Linear)";
        let result = parse(super::effect_timer(), input).result.unwrap();
        let expected = call_expr(FnCall::EffectTimerNew, &[
            call_expr(FnCall::DurationFromMillis, &[literal(Value::U32(1000))]),
            literal(Value::Interpolation(Interpolation::Linear))
        ]);
        assert_eq!(result, expected);

        let input = "EffectTimer::new(Duration::from_secs_f32(0.5), Linear)";
        let result = parse(super::effect_timer(), input).result.unwrap();
        let expected = call_expr(FnCall::EffectTimerNew, &[
            call_expr(FnCall::DurationFromSeconds, &[literal(Value::F32(0.5))]),
            literal(Value::Interpolation(Interpolation::Linear))
        ]);
        assert_eq!(result, expected);

        let input = "(1337, Reverse)";
        let result = parse(super::effect_timer(), input).result.unwrap();
        let expected = call_expr(FnCall::EffectTimerNew, &[
            call_expr(FnCall::DurationFromMillis, &[literal(Value::U32(1337))]),
            literal(Value::Interpolation(Interpolation::Reverse))
        ]);
        assert_eq!(result, expected);

        let input = "(Duration::from_millis(1337), Reverse)";
        let result = parse(super::effect_timer(), input).result.unwrap();
        let expected = call_expr(FnCall::EffectTimerNew, &[
            call_expr(FnCall::DurationFromMillis, &[literal(Value::U32(1337))]),
            literal(Value::Interpolation(Interpolation::Reverse))
        ]);
        assert_eq!(result, expected);

        let input = "1234";
        let result = parse(super::effect_timer(), input).result.unwrap();
        let expected = call_expr(FnCall::EffectTimerFromMs, &[
            literal(Value::U32(1234)),
            literal(Value::Interpolation(Interpolation::Linear))
        ]);
        assert_eq!(result, expected);
    }

    #[test]
    fn parse_string() {
        let input = "\"Hello, World!\"";
        assert_parser_eq(
            parse(super::string_literal(), input),
            Value::String("Hello, World!".to_string())
        );

        // let input = r#""Hello, \"World!\"""#;
        let input = "\"Hello, \\\"World!\\\"\"";
        assert_parser_eq(
            parse(super::string_literal(), input),
            Value::String("Hello, \"World!\"".to_string())
        );
    }

    #[test]
    fn test_parse_u32() {
        let input = "1337";
        assert_parser_eq(
            parse(super::parse_u32(), input),
            Value::U32(1337)
        );

        let input = "0x1d2021";
        assert_parser_eq(
            parse(super::parse_u32(), input),
            Value::U32(0x1d2021)
        );
    }

    #[test]
    fn parse_argument() {
        let input = "\"Hello, World!\"";
        assert_parser_eq(
            parse(super::argument(), input),
            Value::String("Hello, World!".to_string())
        );

        let input = "1337";
        assert_parser_eq(
            parse(super::argument(), input),
            Value::U32(1337)
        );

        let input = "3.14";
        assert_parser_eq(
            parse(super::argument(), input),
            Value::F32(3.14)
        );

        let input = "EffectTimer::from_ms(1000, Interpolation::Linear)";
        assert_parser_eq(
            parse(super::argument(), input),
            Value::Timer(EffectTimer::from_ms(1000, Interpolation::Linear))
        );

        let input = "Duration::from_millis(1000)";
        assert_parser_eq(
            parse(super::argument(), input),
            Value::Duration(Duration::from_millis(1000))
        );
    }

    #[test]
    fn parse_arguments() {
        let input = "\"Hello, World!\", 1337, 3.14, (1000, SineIn)";
        assert_eq!(
            parse(super::arguments(), input).result,
            Some(vec![
                Expr::Literal(Value::String("Hello, World!".to_string())),
                Expr::Literal(Value::U32(1337)),
                Expr::Literal(Value::F32(3.14)),
                Expr::Literal(Value::Timer(EffectTimer::from_ms(1000, Interpolation::SineIn)))
            ])
        );
    }

    #[test]
    fn parse_fx_statement() {
        let input = "coalesce(Duration::from_millis(220))";
        let result = parse(super::fx_statement(), input).result;
        let expected = Expr::Fx {
            name: "coalesce".to_string(),
            arguments: vec![
                Expr::Literal(Value::Duration(Duration::from_millis(220))),
            ]
        };

        let input = "fx::dissolve((Duration::from_millis(220), ElasticOut))";
        let result = parse(super::fx_statement(), input).result;
        let expected = Expr::Fx {
            name: "dissolve".to_string(),
            arguments: vec![
                Expr::Literal(Value::Timer(EffectTimer::from_ms(220, Interpolation::ElasticOut))),
            ]
        };
        assert_eq!(result, Some(expected));

        let input = "fx::ping_pong(fx::coalesce((500, CircOut)))";
        let result = parse(super::fx_statement(), input).result;
        let expected = Expr::Fx {
            name: "ping_pong".to_string(),
            arguments: vec![
                Expr::Fx {
                    name: "coalesce".to_string(),
                    arguments: vec![
                        Expr::Literal(Value::Timer(EffectTimer::from_ms(500, Interpolation::CircOut))),
                    ]
                }
            ]
        };
        assert_eq!(result, Some(expected));
    }

    // #[test]
    // fn test_parse_and_deserialize() {
    //     let input = r#"fx::sweep_in(
    //         Motion::LeftToRight,
    //         10,
    //         0,
    //         Color::from_u32(0x1d2021),
    //         (1000, QuadOut)
    //     )"#;
    //
    //     let parsed = parse(super::fx_statement(), input).result.unwrap();
    //     let env = DslEnv::new();
    //     let context = EffectDsl::new();
    //     let mut args = InputArgs::new(
    //         match parsed {
    //             Expr::Fx { arguments: parameters, .. } => parameters.into(),
    //             _ => panic!("Expected Fx variant")
    //         },
    //         &context,
    //         &env
    //     );
    //
    //     assert_eq!(args.motion(),   Ok(Motion::LeftToRight));
    //     assert_eq!(args.read_u16(), Ok(10));
    //     assert_eq!(args.read_u16(), Ok(0));
    //     assert_eq!(args.color(),    Ok(Color::from_u32(0x1d2021)));
    //     assert_eq!(args.effect_timer(), Ok(EffectTimer::from_ms(1000, Interpolation::QuadOut)));
    // }
}
