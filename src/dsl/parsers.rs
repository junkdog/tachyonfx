use crate::dsl::expressions::{Expr, FnCall, FnCallInfo, StyleMethod, Value};
use crate::fx::RepeatMode;
use crate::{CellFilter, Interpolation, Motion};
use anpa::combinators::{attempt, many, many_to_vec, middle, no_separator, or_diff, right, separator, succeed, times};
use anpa::core::parse;
use anpa::core::{ParserExt, StrParser};
use anpa::number::float;
use anpa::parsers::{item_if, item_while};
use anpa::whitespace::skip_whitespace;
use anpa::{defer_parser, greedy_or, or, right, skip, take, tuplify};
use ratatui::prelude::Style;
use ratatui::style::{Color, Modifier};
use crate::dsl::DslError;

pub(super) fn parse_expr(
    input: &str,
) -> Result<Expr, DslError> {
    let parsed = parse(or!(container_effect(), effect()), input);
    if let Some(expr) = parsed.result {
        Ok(expr)
    } else {
        Err(DslError::ParseError(format!("remaining input: {}", parsed.state)))
    }
}


fn trim<'a>(prefix: &str) -> impl StrParser<'a, ()> + use<'a, '_>{
    right!(
        skip_whitespace(),
        skip!(prefix),
        skip_whitespace()
    )
}

fn effect<'a>() -> impl StrParser<'a, Expr> {
    let name = right!(
        succeed(attempt(skip!("fx::"))),
        item_while(|c: char| c.is_ascii_alphabetic() || c.is_ascii_digit() || c == '_'),
    );

    let args = middle(trim("("), arguments(), trim(")"));
    let self_fns = many_to_vec(self_fn_call(), true, separator(trim(""), true));

    tuplify!(name, args, self_fns)
        .map(|(name, arguments, self_fns)| Expr::Fx {
            name: name.to_string(),
            arguments,
            self_fns
        }
    )
}

// parameterizing effect with CellFilter, e.g. effect.filter(CellFilter::All)
fn effect_cell_filter<'a>() -> impl StrParser<'a, Option<Expr>> {
    succeed(
        middle(
            trim(".filter("),
            cell_filter(),
            trim(")")
        )
    )
}

fn fx_name<'a>(s: &'static str) -> impl StrParser<'a, &'a str> {
    right!(
        succeed(attempt(skip!("fx::"))),
        take!(s),
    )
}

fn container_effect<'a>() -> impl StrParser<'a, Expr> {
    let effect_parser = or!(defer_parser!(container_effect()), effect());

    let name = or!(fx_name("sequence"), fx_name("parallel"));
    let args = middle(
        right!(trim("("), trim("&[")),
        many_to_vec(effect_parser, true, separator(trim(","), true)),
        right!(trim("]"), trim(")"))
    );

    tuplify!(
        name,
        args,
        many_to_vec(self_fn_call(), true, separator(trim(""), true))
    ).map(|(name, args, self_fns)| match name {
        "sequence" => Expr::Sequence { effects: args, self_fns },
        "parallel" => Expr::Parallel { effects: args, self_fns },
        _ => unreachable!()
    })
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
        arguments(),
        trim("]")
    ).map(Expr::ArrayRef)
}

fn array<'a>() -> impl StrParser<'a, Expr> {
    middle(
        trim("["),
        arguments(),
        trim("]")
    ).map(Expr::Array)
}

fn option<'a>() -> impl StrParser<'a, Expr> {
    or!(
        trim("None").map(|_| Expr::Literal(Value::None)),
        middle(
            trim("Some("),
            argument(),
            trim(")")
        ).map(Box::new).map(Expr::OptionSome)
    )
}

fn var<'a>() -> impl StrParser<'a, Expr> {
    many(item_if(|c: char| matches!(c, 'a'..='z' | '0'..='9' | '_')), false, no_separator())
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
    let fg_color = middle(cf("FgColor("), color_ctor(), trim(")"))
        .map(|color| Expr::CellFilter {
            filter_type: "FgColor",
            arguments: vec![color]
        });
    let bg_color = middle(cf("BgColor("), color_ctor(), trim(")"))
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
    //     right!(trim(","), parse_u32(), trim(")")),
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
            color_ctor(),
            repeat_mode(), // used by fx::repeat
            style(),
            array_ref(), // e.g. &[fx1, fx2, fx3]
            array(),     // e.g. [1, 2, 3]
            container_effect(),
            option(),
            cell_filter(),
            effect(),
            fn_call().map(Expr::FnCall),
            var(),
        )
    }
}

fn arguments<'a>() -> impl StrParser<'a, Vec<Expr>> {
    many_to_vec(argument(), true, separator(trim(","), false))
}

fn style<'a>() -> impl StrParser<'a, Expr> {
    let constructor = or!(
        right!(trim("Style::"), or!(
            skip!("new()"),
            skip!("default()")
        ))
    ).map(|_| Style::default());

    let style_chain = many_to_vec(style_method_call(), false, separator(trim(""), true));

    right!(
        constructor,
        or!(
            style_chain.map(Expr::Style),
            succeed(trim("")).map(|_| Expr::Style(vec![]))
        )
    )
}

fn fn_call<'a>() -> impl StrParser<'a, FnCallInfo> {
    let fn_name_char = item_if(|c: char| matches!(c, 'a'..='z' | '0'..='9' | '_'));
    let fn_name = many(fn_name_char, false, no_separator())
        .map(|s: &str| s.to_string());

    tuplify!(
        fn_name,
        middle(trim("("), arguments(), trim(")"))
    ).map(FnCallInfo::from)
}

fn self_fn_call<'a>() -> impl StrParser<'a, FnCallInfo> {
    right!(
        skip_whitespace(),
        skip!("."),
        fn_call()
    )
}


fn style_method_call<'a>() -> impl StrParser<'a, StyleMethod> {
    or!(
        right!(
            skip!("."),
            middle(trim("fg("), color_ctor(), trim(")"))
        ).map(StyleMethod::Fg),

        right!(
            skip!("."),
            middle(trim("bg("), color_ctor(), trim(")"))
        ).map(StyleMethod::Bg),

        right!(
            skip!("."),
            middle(trim("add_modifier("), modifier(), trim(")"))
        ).map(StyleMethod::AddModifier)
    )
}

fn modifier<'a>() -> impl StrParser<'a, Modifier> {
    right!(
        trim("Modifier::"),
        or!(
            skip!("BOLD").map(|_| Modifier::BOLD),
            skip!("DIM").map(|_| Modifier::DIM),
            skip!("ITALIC").map(|_| Modifier::ITALIC),
            skip!("UNDERLINED").map(|_| Modifier::UNDERLINED),
            skip!("SLOW_BLINK").map(|_| Modifier::SLOW_BLINK),
            skip!("RAPID_BLINK").map(|_| Modifier::RAPID_BLINK),
            skip!("REVERSED").map(|_| Modifier::REVERSED),
            skip!("HIDDEN").map(|_| Modifier::HIDDEN),
            skip!("CROSSED_OUT").map(|_| Modifier::CROSSED_OUT)
        )
    )
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
            parse_u32(),
            right!(trim(","), parse_u32()),
            right!(trim(","), parse_u32()),
            right!(trim(","), parse_u32()),
        ),
        trim(")")
    ).map(|(x, y, w, h)| Expr::Call {
        function: FnCall::RectNew,
        args: vec![x, y, w, h]
    });

    let raw = middle(
        right!(trim("Rect"), trim("{")),
        tuplify!(
            middle(trim("x:"), parse_u32(), trim(",")),
            middle(trim("y:"), parse_u32(), trim(",")),
            middle(trim("width:"), parse_u32(), trim(",")),
            right!(trim("height:"), parse_u32()),
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
            parse_u32(),
            right!(trim(","), parse_u32())
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
            middle(trim("horizontal:"), parse_u32(), trim(",")),
            right!(trim("vertical:"), parse_u32()),
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

fn color_ctor<'a>() -> impl StrParser<'a, Expr> {
    middle(
        trim("Color::from_u32("),
        parse_u32(),
        trim(")")
    ).map(|u32| Expr::FnCall(FnCallInfo {
        name: "Color::from_u32".to_string(),
        args: vec![u32]
    }))
}

fn color<'a>() -> impl StrParser<'a, Expr> {
    // rgb
    let rgb = middle(
        trim("Color::Rgb("),
        tuplify!(
            parse_u32(),
            right!(trim(","), parse_u32()),
            right!(trim(","), parse_u32()),
        ),
        trim(")")
    ).map(|(r, g, b)| Expr::Call {
        function: FnCall::ColorRgb,
        args: vec![r, g, b]
    });

    // indexed
    let indexed = middle(
        trim("Color::Indexed("),
        parse_u32(),
        trim(")")
    ).map(|idx| Expr::Call {
        function: FnCall::ColorIndexed,
        args: vec![idx]
    });


    // named colors
    let literal = right!(
        succeed(attempt(skip!("Color::"))),
        item_while(|c: char| c.is_ascii_alphabetic()),
    ).map_if(|s: &str| match s {
        "Reset"        => Some(Color::Reset),
        "Black"        => Some(Color::Black),
        "Red"          => Some(Color::Red),
        "Green"        => Some(Color::Green),
        "Yellow"       => Some(Color::Yellow),
        "Blue"         => Some(Color::Blue),
        "Magenta"      => Some(Color::Magenta),
        "Cyan"         => Some(Color::Cyan),
        "Gray"         => Some(Color::Gray),
        "DarkGray"     => Some(Color::DarkGray),
        "LightRed"     => Some(Color::LightRed),
        "LightGreen"   => Some(Color::LightGreen),
        "LightYellow"  => Some(Color::LightYellow),
        "LightBlue"    => Some(Color::LightBlue),
        "LightMagenta" => Some(Color::LightMagenta),
        "LightCyan"    => Some(Color::LightCyan),
        "White"        => Some(Color::White),
        _              => None
    }).map(|c| Expr::Literal(Value::Color(c)));

    or!(literal, rgb, indexed)
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
    use crate::dsl::expressions::{Expr, FnCall, FnCallInfo, StyleMethod, Value};
    use crate::fx::RepeatMode;
    use crate::{CellFilter, Duration, Interpolation, Motion};
    use anpa::core::{parse, AnpaResult, ParserExt};
    use ratatui::style::{Color, Modifier};

    fn assert_expr_eq(
        result: AnpaResult<&str, Expr>,
        expected: Expr
    ) {
        assert_eq!(result.state, "", "Expected parser to consume the entire input");
        assert!(result.result.is_some());
        assert_eq!(result.result.unwrap(), expected);
    }

    fn assert_parser_eq(
        result: AnpaResult<&str, Expr>,
        expected: Value
    ) {
        assert_eq!(result.state, "", "Expected parser to consume the entire input");
        assert_eq!(result.result, Some(Expr::Literal(expected)));
    }

    fn assert_cell_filter_eq(
        input: &str,
        expected: Expr,
    ) {
        let filter = parse(super::cell_filter(), input)
            .result
            .expect("Failed to parse cell filter");

        assert_eq!(filter, expected);
    }

    #[test]
    fn skip_trim() {
        let input = "  Hello  ";
        let result = parse(super::trim("Hello"), input).result;
        assert_eq!(result, Some(()))

    }

    #[test]
    fn test_color_from_u32() {
        let input = "Color::from_u32(0x1d2021)";
        assert_expr_eq(
            parse(super::color_ctor(), input),
            Expr::FnCall(FnCallInfo {
                name: "Color::from_u32".to_string(),
                args: vec![Expr::Literal(Value::U32(0x1d2021))]
            })
        );
    }

    #[test]
    fn test_color_variants() {
        [
            Color::Black,
            Color::Red,
            Color::Green,
            Color::Yellow,
            Color::Blue,
            Color::Magenta,
            Color::Cyan,
            Color::Gray,
            Color::DarkGray,
            Color::LightRed,
            Color::LightGreen,
            Color::LightYellow,
            Color::LightBlue,
            Color::LightMagenta,
            Color::LightCyan,
            Color::White,
        ].into_iter().for_each(|color| {
            let input = format!("Color::{}", color.to_string());
            assert_parser_eq(
                parse(super::color(), &input),
                Value::Color(color)
            );
        });

        let input = "Color::Indexed(3)";
        assert_expr_eq(
            parse(super::color(), input),
            call_expr(FnCall::ColorIndexed, &[literal(Value::U32(3))])
        );

        let input = "Color::Rgb(255, 127, 64)";
        assert_expr_eq(
            parse(super::color(), input),
            call_expr(FnCall::ColorRgb, &[
                literal(Value::U32(255)),
                literal(Value::U32(127)),
                literal(Value::U32(64))
            ])
        );
    }

    #[test]
    fn test_margin() {
        let input = "Margin::new(10, 20)";
        assert_expr_eq(
            parse(super::margin(), input),
            call_expr(FnCall::MarginNew, &[literal(Value::U32(10)), literal(Value::U32(20))])
        );

        let input = r#"Margin {
            horizontal: 10,
            vertical: 20
        }"#;
        assert_expr_eq(
            parse(super::margin(), input),
            call_expr(FnCall::MarginStruct, &[
                literal(Value::U32(10)),
                literal(Value::U32(20))
            ])
        );
    }

    #[test]
    fn test_optional() {
        let input = "None";
        assert_expr_eq(
            parse(super::option(), input),
            Expr::Literal(Value::None)
        );

        let input = "Some(10)";
        assert_expr_eq(
            parse(super::option(), input),
            Expr::OptionSome(Box::new(literal(Value::U32(10))))
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
        assert_expr_eq(
            parse(super::repeat_mode(), input),
            call_expr(FnCall::RepeatModeTimes, &[literal(Value::U32(10))])
        );

        let input = "RepeatMode::Duration(Duration::from_millis(1000))";
        assert_expr_eq(
            parse(super::repeat_mode(), input),
            call_expr(FnCall::RepeatModeDuration, &[
                call_expr(FnCall::DurationFromMillis, &[literal(Value::U32(1000))])
            ])
        );
    }

    #[test]
    fn test_basic_filters() {
        // Test All filter
        assert_cell_filter_eq("All", literal(Value::CellFilter(CellFilter::All)));

        // Test Text filter
        assert_cell_filter_eq("Text", literal(Value::CellFilter(CellFilter::Text)));
    }

    #[test]
    fn test_fn_call_and_self_fn_call() {
        let input = "foo(\"bar\")";
        assert_expr_eq(
            parse(super::fn_call().map(Expr::FnCall), input),
            Expr::FnCall(FnCallInfo {
                name: "foo".to_string(),
                args: vec![literal(Value::String("bar".into()))]
            })
        );

        // let input = ".bar(10)";
        // assert_expr_eq(
        //     parse(super::self_fn_call().map(Expr::SelfFnCall), input),
        //     Expr::SelfFnCall(FnCallInfo {
        //         name: "bar".to_string(),
        //         args: vec![literal(Value::U32(10))]
        //     })
        // );
    }

    #[test]
    fn test_style() {
        let input = "Style::default()";
        assert_expr_eq(
            parse(super::style(), input),
            Expr::Style(vec![])
        );

        let input = "Style::new()\
            .fg(Color::from_u32(0x1d2021))\
            .bg(Color::from_u32(0x1d2023))\
            .add_modifier(Modifier::BOLD)";

        assert_expr_eq(
            parse(super::style(), input),
            Expr::Style(vec![
                StyleMethod::Fg(Expr::FnCall(FnCallInfo {
                    name: "Color::from_u32".to_string(),
                    args: vec![Expr::Literal(Value::U32(0x1d2021))]
                })),
                StyleMethod::Bg(Expr::FnCall(FnCallInfo {
                    name: "Color::from_u32".to_string(),
                    args: vec![Expr::Literal(Value::U32(0x1d2023))]
                })),
                StyleMethod::AddModifier(Modifier::BOLD),
            ])
        );
    }

    #[test]
    fn test_sequence() {
        let input = r#"fx::sequence(
            &[fx::yolo("Hello"), fx::fubar("World")]
        )"#;
        assert_expr_eq(
            parse(super::container_effect(), input),
            Expr::Sequence { self_fns: vec![], effects: vec![
                Expr::Fx { name: "yolo".to_string(), self_fns: vec![], arguments: vec![
                    literal(Value::String("Hello".to_string()))
                ]},
                Expr::Fx { name: "fubar".to_string(), self_fns: vec![], arguments: vec![
                    literal(Value::String("World".to_string()))
                ]}
            ]}
        );
    }

    #[test]
    fn effect_with_cell_filter() {
        let input = r#"fx::yolo("Hello").filter(CellFilter::Text)"#;
        assert_expr_eq(
            parse(super::effect(), input),
            Expr::Fx {
                name: "yolo".to_string(),
                arguments: vec![literal(Value::String("Hello".to_string()))],
                self_fns: vec![
                    FnCallInfo::new("filter",
                        vec![literal(Value::CellFilter(CellFilter::Text))]
                    )],
            }
        );
    }

    #[test]
    fn test_parallel() {
        let input = r#"fx::parallel(&[
            fx::foo(),
            fx::bar()
        ])"#;
        assert_expr_eq(
            parse(super::container_effect(), input),
            Expr::Parallel{ self_fns: vec![], effects: vec![
                Expr::Fx { name: "foo".to_string(), self_fns: vec![], arguments: vec![] },
                Expr::Fx { name: "bar".to_string(), self_fns: vec![], arguments: vec![] }
            ]}
        );
    }

    #[test]
    fn test_color_filters() {
        // Test FgColor
        assert_cell_filter_eq(
            "FgColor(Color::from_u32(0xFF0000))",
            Expr::CellFilter {
                filter_type: "FgColor",
                arguments: vec![Expr::FnCall(FnCallInfo {
                    name: "Color::from_u32".to_string(),
                    args: vec![Expr::Literal(Value::U32(0xFF0000))]
                })]
            }
        );

        // Test BgColor
        assert_cell_filter_eq(
            "BgColor(Color::from_u32(0x00FF00))",
            Expr::CellFilter {
                filter_type: "BgColor",
                arguments: vec![Expr::FnCall(FnCallInfo {
                    name: "Color::from_u32".to_string(),
                    args: vec![Expr::Literal(Value::U32(0x00FF00))]
                })],
            }
        );
    }

    #[test]
    fn test_margin_filters() {
        // Test Inner margin
        assert_cell_filter_eq(
            "Inner(Margin::new(1, 2))",
            Expr::CellFilter {
                filter_type: "Inner",
                arguments: vec![call_expr(FnCall::MarginNew, &[literal(Value::U32(1)), literal(Value::U32(2))])]
            }
        );

        // Test Outer margin
        assert_cell_filter_eq(
            "Outer(Margin::new(3, 4))",
            Expr::CellFilter {
                filter_type: "Outer",
                arguments: vec![call_expr(FnCall::MarginNew, &[literal(Value::U32(3)), literal(Value::U32(4))])]
            }
        );
    }

    // #[test]
    fn test_layout_filter() {
        todo!("Layout filter is not yet implemented");
        // let layout = Layout::default()
        //     .direction(Direction::Vertical)
        //     .constraints([Constraint::Length(1), Constraint::Min(0)]);
        //
        // assert_cell_filter_eq(
        //     "Layout(Layout::vertical([Length(1), Min(0)]), 1)",
        //     Expr::CellFilter {
        //         filter_type: "Layout",
        //         arguments: vec![
        //             Expr::Literal(Value::Layout(layout)),
        //             Expr::Literal(Value::U32(1))
        //         ]
        //     }
        // );
    }

    #[test]
    fn test_compound_filters() {
        // Test AllOf
        assert_cell_filter_eq(
            "AllOf(vec![Text, Inner(Margin::new(1, 1))])",
            Expr::CellFilter {
                filter_type: "AllOf",
                arguments: vec![
                    Expr::Literal(Value::CellFilter(CellFilter::Text)),
                    Expr::CellFilter {
                        filter_type: "Inner",
                        arguments: vec![call_expr(FnCall::MarginNew, &[literal(Value::U32(1)), literal(Value::U32(1))])]
                    }
                ]
            }
        );

        // Test AnyOf
        assert_cell_filter_eq(
            "AnyOf(vec![Text, Outer(Margin::new(1, 1))])",
            Expr::CellFilter {
                filter_type: "AnyOf",
                arguments: vec![
                    literal(Value::CellFilter(CellFilter::Text)),
                    Expr::CellFilter {
                        filter_type: "Outer",
                        arguments: vec![call_expr(FnCall::MarginNew, &[literal(Value::U32(1)), literal(Value::U32(1))])]
                    }
                ]
            }
        );

        // Test NoneOf
        assert_cell_filter_eq(
            "NoneOf(vec![Text, Inner(Margin::new(1, 1))])",
            Expr::CellFilter {
                filter_type: "NoneOf",
                arguments: vec![
                    literal(Value::CellFilter(CellFilter::Text)),
                    Expr::CellFilter {
                        filter_type: "Inner",
                        arguments: vec![call_expr(FnCall::MarginNew, &[literal(Value::U32(1)), literal(Value::U32(1))])]
                    }
                ]
            }
        );
    }

    #[test]
    fn test_not_filter() {
        assert_cell_filter_eq(
            "CellFilter::Not(Box::new(CellFilter::Text))",
            Expr::CellFilter {
                filter_type: "Not",
                arguments: vec![Expr::Literal(Value::CellFilter(CellFilter::Text))]
            }
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
            Expr::CellFilter {
                filter_type: "AllOf",
                arguments: vec![
                    Expr::CellFilter {
                        filter_type: "Not",
                        arguments: vec![Expr::Literal(Value::CellFilter(CellFilter::Text))]
                    },
                    Expr::CellFilter {
                        filter_type: "AnyOf",
                        arguments: vec![
                            Expr::CellFilter {
                                filter_type: "Inner",
                                arguments: vec![call_expr(FnCall::MarginNew, &[literal(Value::U32(1)), literal(Value::U32(1))])]
                            },
                            Expr::CellFilter {
                                filter_type: "Outer",
                                arguments: vec![call_expr(FnCall::MarginNew, &[literal(Value::U32(2)), literal(Value::U32(2))])]
                            }
                        ]
                    }
                ]
            }
        );
    }

    #[test]
    fn test_rect() {
        let input = "Rect::new(10, 20, 30, 40)";
        assert_expr_eq(
            parse(super::rect(), input),
            call_expr(FnCall::RectNew, &[
                literal(Value::U32(10)),
                literal(Value::U32(20)),
                literal(Value::U32(30)),
                literal(Value::U32(40))
            ])
        );

        let input = r#"Rect {
            x: 10,
            y: 20,
            width: 30,
            height: 40
        }"#;
        assert_expr_eq(
            parse(super::rect(), input),
            call_expr(FnCall::RectStruct, &[
                literal(Value::U32(10)),
                literal(Value::U32(20)),
                literal(Value::U32(30)),
                literal(Value::U32(40))
            ])
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

    #[test]
    fn parse_array_ref() {
        let input = "&[\"Hello, World!\", 1337, 3.14, (1000, SineIn)]";
        let result = parse(super::array_ref(), input);
        let expected = Expr::ArrayRef(vec![
            Expr::Literal(Value::String("Hello, World!".to_string())),
            Expr::Literal(Value::U32(1337)),
            Expr::Literal(Value::F32(3.14)),
            call_expr(FnCall::EffectTimerNew, &[
                call_expr(FnCall::DurationFromMillis, &[literal(Value::U32(1000))]),
                literal(Value::Interpolation(Interpolation::SineIn))
            ])
        ]);

        assert_expr_eq(result, expected);
    }

    #[test]
    fn parse_array() {
        let input = "[\"Hello, World!\", 1337, 3.14, (1000, SineIn)]";
        let result = parse(super::array(), input);
        let expected = Expr::Array(vec![
            Expr::Literal(Value::String("Hello, World!".to_string())),
            Expr::Literal(Value::U32(1337)),
            Expr::Literal(Value::F32(3.14)),
            call_expr(FnCall::EffectTimerNew, &[
                call_expr(FnCall::DurationFromMillis, &[literal(Value::U32(1000))]),
                literal(Value::Interpolation(Interpolation::SineIn))
            ])
        ]);

        assert_expr_eq(result, expected);
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
        assert_expr_eq(
            parse(super::argument(), input),
            call_expr(FnCall::EffectTimerFromMs, &[
                literal(Value::U32(1000)),
                literal(Value::Interpolation(Interpolation::Linear))
            ])
        );

        let input = "Duration::from_millis(1000)";
        assert_expr_eq(
            parse(super::argument(), input),
            call_expr(FnCall::DurationFromMillis, &[literal(Value::U32(1000))])
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
                call_expr(FnCall::EffectTimerNew, &[
                    call_expr(FnCall::DurationFromMillis, &[literal(Value::U32(1000))]),
                    literal(Value::Interpolation(Interpolation::SineIn))
                ])
            ])
        );
    }

    #[test]
    fn parse_fx_statement() {
        let input = "coalesce(Duration::from_millis(220))";
        let result = parse(super::effect(), input).result;
        let expected = Expr::Fx {
            name: "coalesce".to_string(),
            arguments: vec![
                call_expr(FnCall::DurationFromMillis, &[literal(Value::U32(220))]),
            ],
            self_fns: vec![]
        };
        assert_eq!(result, Some(expected));

        let input = "fx::dissolve((Duration::from_millis(220), ElasticOut))";
        let result = parse(super::effect(), input).result;
        let expected = Expr::Fx {
            name: "dissolve".to_string(),
            self_fns: vec![],
            arguments: vec![
                call_expr(FnCall::EffectTimerNew, &[
                    call_expr(FnCall::DurationFromMillis, &[literal(Value::U32(220))]),
                    Expr::Literal(Value::Interpolation(Interpolation::ElasticOut))
                ])
            ]};
        assert_eq!(result, Some(expected));

        let input = "fx::ping_pong(fx::coalesce((500, CircOut)))";
        let result = parse(super::effect(), input).result;
        let expected = Expr::Fx {
            name: "ping_pong".to_string(),
            self_fns: vec![],
            arguments: vec![
                Expr::Fx {
                    name: "coalesce".to_string(),
                    self_fns: vec![],
                    arguments: vec![
                        call_expr(FnCall::EffectTimerNew, &[
                            call_expr(FnCall::DurationFromMillis, &[literal(Value::U32(500))]),
                            Expr::Literal(Value::Interpolation(Interpolation::CircOut))
                        ])
                    ]
                }
            ]
        };
        assert_eq!(result, Some(expected));
    }

    fn call_expr(function: FnCall, args: &[Expr]) -> Expr {
        Expr::Call { function, args: args.into() }
    }

    fn literal(value: Value) -> Expr {
        Expr::Literal(value)
    }
}
