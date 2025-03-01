use crate::dsl::expressions::{Expr, FnCallInfo, Value};
use crate::dsl::DslError;
use crate::fx::RepeatMode;
use crate::{CellFilter, EffectTimer, Interpolation, Motion};
use anpa::combinators::*;
use anpa::core::{parse, ParserExt, StrParser};
use anpa::number::float;
use anpa::parsers::{item_if, item_while, until};
use anpa::whitespace::{skip_whitespace, whitespace};
use anpa::{defer_parser, greedy_or, or, right, skip, take, tuplify};
use compact_str::{format_compact, CompactString, ToCompactString};
use ratatui::layout::{Constraint, Direction, Margin, Rect};
use ratatui::prelude::Style;
use ratatui::style::{Color, Modifier};
use std::ops::Neg;

pub(super) fn parse_expr(
    input: &str,
) -> Result<Vec<Expr>, DslError> {
    let main_parser = or!(let_binding(), container_effect(), effect(), var());
    let parsed_expr = parse(many_to_vec(main_parser, true, no_separator()), input);

    if !parsed_expr.state.is_empty() {
        return Err(DslError::ParseError(format_compact!("unparsed input: {}", parsed_expr.state)));
    }

    match parsed_expr.result {
        Some(exprs) => Ok(exprs),
        None => Err(DslError::ParseError(format_compact!("unparsed input: {}", parsed_expr.state)))
    }
}

pub(super) fn argument<'a>() -> impl StrParser<'a, Expr> {
    // must defer to avoid recursive opaqueness
    defer_parser! {
        or!(
            container_effect(),
            effect(),
            // `parse_f32` must come last due to how float() is implemented,
            // as such we use greedy_or to ensure that `parse_f32` is not
            // picked over `parse_i32` or `parse_u32`.
            greedy_or!(parse_i32(), parse_u32(), parse_f32()),
            string_literal(),
            effect_timer(),
            duration(),
            motion(),
            modifier(),
            constraint(),
            direction(),
            layout(),
            rect(),
            offset(),
            margin(),
            color(),
            repeat_mode(), // used by fx::repeat
            style(),
            array_ref(), // e.g. &[fx1, fx2, fx3]
            array(),     // e.g. [1, 2, 3]
            option(),
            cell_filter(),
            var(),
            let_binding(),
        )
    }
}

fn arguments<'a>() -> impl StrParser<'a, Vec<Expr>> {
    many_to_vec(argument(), true, separator(trim(","), true))
}

fn trim_to<'a>(prefix: &str) -> impl StrParser<'a, ()> + use<'a, '_>{
    right!(skip_comments_and_whitespace(), skip!(prefix))
}

fn trim<'a>(prefix: &str) -> impl StrParser<'a, ()> + use<'a, '_>{
    right!(trim_to(prefix), skip_comments_and_whitespace())
}

fn effect<'a>() -> impl StrParser<'a, Expr> {
    let name = right!(
        trim(""),
        succeed(attempt(skip!("fx::"))),
        snake_case_str(),
    );

    let args = middle(trim("("), arguments(), trim(")"));

    tuplify!(name, args, chained_fn_calls())
        .map(|(name, arguments, self_fns)| Expr::Fx {
            name: name.to_compact_string(),
            arguments,
            self_fns
        }
    )
}

fn fx_name<'a>(s: &'static str) -> impl StrParser<'a, &'a str> {
    right!(
        trim(""),
        succeed(attempt(skip!("fx::"))),
        take!(s),
    )
}

fn container_effect<'a>() -> impl StrParser<'a, Expr> {
    let effect_parser = or!(
        defer_parser!(container_effect()), // nested sequence() or parallel()
        effect(),                          // regular effect
        var()                              // bound variable or let binding
    );

    let name = or!(fx_name("sequence"), fx_name("parallel"));
    let args = middle(
        right!(trim("("), trim("&[")),
        many_to_vec(effect_parser, true, separator(trim(","), true)),
        right!(trim("]"), trim(")"))
    );

    tuplify!(
        name,
        args,
        chained_fn_calls()
    ).map(|(name, effects, self_fns)| match name {
        "sequence" => Expr::Sequence { effects, self_fns },
        "parallel" => Expr::Parallel { effects, self_fns },
        _ => unreachable!("already confirmed name is either sequence or parallel")
    })
}

fn string_literal<'a>() -> impl StrParser<'a, Expr> {
    let unicode = right(skip!('u'), times(4, item_if(|c: char| c.is_ascii_hexdigit())));
    let escaped = right(skip!('\\'), or_diff(unicode, item_if(|c: char| "\"\\/bfnrt".contains(c))));
    let valid_char = item_if(|c: char| c != '"' && c != '\\' && !c.is_control());
    let not_end = or_diff(valid_char, escaped);

    middle(skip!('"'), many(not_end, true, no_separator()), skip!('"'))
        .map(|s: &str| s.replace("\\\"", "\""))
        .map(|s| s.to_compact_string())
        .map(IntoLiteral::into_literal)
}

fn skip_comments_and_whitespace<'a>() -> impl StrParser<'a, ()> {
    let line_comment = right!(skip!("//"), item_while(|c: char| c != '\n'));
    let block_comment = right!(skip!("/*"), until("*/"));

    let parsers = or!(
        line_comment.map(|_| ()),
        block_comment.map(|_| ()),
        not_empty(whitespace()).map(|_| ())
    );

    many(parsers, true, no_separator())
        .map(|_| ())
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
        trim("None").map(|_| Expr::Literal(Value::OptionNone)),
        middle(
            trim("Some("),
            argument(),
            trim(")")
        ).map(Box::new).map(Expr::OptionSome)
    )
}

fn var<'a>() -> impl StrParser<'a, Expr> {
    tuplify!(
        snake_case_str().map(|s: &str| s.to_compact_string()),
        chained_fn_calls()
    ).map(|(name, self_fns)| Expr::Var { name, self_fns })
}

fn let_binding<'a>() -> impl StrParser<'a, Expr> {
    tuplify!(
        right!(trim("let"), snake_case_str()),
        middle(trim("="), argument(), trim(";")),
    ).map(|(name, value)| Expr::LetBinding {
        name: name.to_compact_string(),
        let_expr: Box::new(value),
    })
}

fn cell_filter<'a>() -> impl StrParser<'a, Expr> {
    // cell id filter
    let cf = |s| right!(
        skip_whitespace(),
        succeed(attempt(skip!("CellFilter::"))),
        skip!(s)
    );

    // Basic filters
    let all = cf("All").map(|_| CellFilter::All.into_literal());
    let text = cf("Text").map(|_| CellFilter::Text.into_literal());

    // Layout filter
    let layout = middle(
        cf("Layout("),
        tuplify!(
            argument(),
            right!(trim(","), parse_u32())
        ),
        trim(")")
    ).map(|(layout, idx)| Expr::CellFilter {
        filter_type: "Layout",
        arguments: vec![layout, idx]
    });

    // Color filters
    let fg_color = middle(cf("FgColor("), argument(), trim(")"))
        .map(|color| Expr::CellFilter {
            filter_type: "FgColor",
            arguments: vec![color]
        });
    let bg_color = middle(cf("BgColor("), argument(), trim(")"))
        .map(|color| Expr::CellFilter {
            filter_type: "BgColor",
            arguments: vec![color]
        });

    // Margin-based filters
    let inner = middle(cf("Inner("), argument(), trim(")"))
        .map(|margin| Expr::CellFilter {
            filter_type: "Inner",
            arguments: vec![margin]
        });
    let outer = middle(cf("Outer("), argument(), trim(")"))
        .map(|margin| Expr::CellFilter {
            filter_type: "Outer",
            arguments: vec![margin]
        });

    // Compound filters
    let all_of = middle(
        right!(cf("AllOf("), trim("vec![")),
        arguments(),
        right!(trim("]"), trim(")"))
    ).map(|filters| Expr::CellFilter {
        filter_type: "AllOf",
        arguments: filters
    });

    let any_of = middle(
        right!(cf("AnyOf("), trim("vec![")),
        arguments(),
        right!(trim("]"), trim(")"))
    ).map(|filters| Expr::CellFilter {
        filter_type: "AnyOf",
        arguments: filters
    });

    let none_of = middle(
        right!(cf("NoneOf("), trim("vec![")),
        arguments(),
        right!(trim("]"), trim(")"))
    ).map(|filters| Expr::CellFilter {
        filter_type: "NoneOf",
        arguments: filters
    });

    let not = middle(
        right!(cf("Not("), trim("Box::new(")),
        argument(),
        right!(trim(")"), trim(")")),
    ).map(|filter| Expr::CellFilter {
        filter_type: "Not",
        arguments: vec![filter]
    });

    or!(
        fg_color,
        bg_color,
        inner,
        outer,
        layout,
        all_of,
        any_of,
        none_of,
        not,
        all,
        text,
    )
}

fn style<'a>() -> impl StrParser<'a, Expr> {
    let constructor = or!(
        right!(trim("Style::"), or!(
            skip!("new()"),
            skip!("default()")
        ))
    ).map(|_| Style::default());


    right!(
        constructor,
        or!(
            chained_fn_calls().map(Expr::Style),
            succeed(trim("")).map(|_| Expr::Style(vec![]))
        )
    )
}

fn snake_case_str<'a>() -> impl StrParser<'a, &'a str> {
    not_empty(item_while(|c: char| matches!(c, 'a'..='z' | '0'..='9' | '_')))
        .map_if(|s: &str| if s.starts_with(|c| matches!(c, 'a'..='z')) { Some(s) } else { None })
}

fn fn_call<'a>() -> impl StrParser<'a, FnCallInfo> {
    tuplify!(
        snake_case_str(),
        middle(trim("("), arguments(), trim(")"))
    ).map(FnCallInfo::from)
}

fn self_fn_call<'a>() -> impl StrParser<'a, FnCallInfo> {
    right!(trim_to("."), fn_call())
}

fn chained_fn_calls<'a>() -> impl StrParser<'a, Vec<FnCallInfo>> {
    many_to_vec(self_fn_call(), true, separator(trim(""), true))
}

fn modifier<'a>() -> impl StrParser<'a, Expr> {
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
    ).map(IntoLiteral::into_literal)
}

fn constraint<'a>() -> impl StrParser<'a, Expr> {
    right!(
        skip_whitespace(),
        succeed(attempt(skip!("Constraint::"))),
        tuplify!(
            item_while(|c: char| c.is_ascii_alphabetic()),
            middle(trim("("), arguments(), trim(")")),
        )
    ).map_if(|(name, args)| match name {
        "Min"        => Some(fn_call_expr("Constraint::Min", args)),
        "Max"        => Some(fn_call_expr("Constraint::Max", args)),
        "Length"     => Some(fn_call_expr("Constraint::Length", args)),
        "Percentage" => Some(fn_call_expr("Constraint::Percentage", args)),
        "Fill"       => Some(fn_call_expr("Constraint::Fill", args)),
        "Ratio"      => Some(fn_call_expr("Constraint::Ratio", args)),
        _            => None,
    })
}

fn offset<'a>() -> impl StrParser<'a, Expr> {
    middle(
        right!(trim("Offset"), trim("{")),
        tuplify!(
            middle(trim("x:"), argument(), trim(",")),
            right!(trim("y:"), argument()),
        ),
        trim("}"),
    ).map(move |(x, y)| fn_call_expr("Offset", vec![x, y]))
}

fn direction<'a>() -> impl StrParser<'a, Expr> {
    right!(
        succeed(skip!("Direction::")),
        or!(
            skip!("Horizontal").map(|_| Direction::Horizontal),
            skip!("Vertical").map(|_| Direction::Vertical),
        )
    ).map(IntoLiteral::into_literal)
}

fn layout<'a>() -> impl StrParser<'a, Expr> {
    let new = middle(
        trim("Layout::new("),
        arguments(),
        trim(")")
    ).map(|args| fn_call_expr("Layout::new", args));

    let horizontal = middle(
        trim("Layout::horizontal("),
        arguments(),
        trim(")")
    ).map(|args| fn_call_expr("Layout::horizontal", args));

    let vertical = middle(
        trim("Layout::vertical("),
        arguments(),
        trim(")")
    ).map(|args| fn_call_expr("Layout::vertical", args));


    tuplify!(
        or!(new, horizontal, vertical),
        chained_fn_calls()
    ).map(move |(ctor, self_fns)| Expr::Layout { expr: Box::new(ctor), self_fns })
}

fn parse_u32<'a>() -> impl StrParser<'a, Expr> {
    let plain = many(item_if(|c: char| c.is_ascii_digit()), false, no_separator())
        .map_if(|s: &str| s.parse().ok());

    let hexadecimal = right!(
        skip!("0x"),
        item_while(|c: char| c.is_ascii_hexdigit())
    ).map_if(|s: &str| u32::from_str_radix(s, 16).ok());

    or!(
        right!(skip_whitespace(), or!(hexadecimal, plain))
            .map(IntoLiteral::into_literal),
        var()
    )
}

fn parse_i32<'a>() -> impl StrParser<'a, Expr> {
    let plain = many(item_if(|c: char| c.is_ascii_digit()), false, no_separator())
        .map_if(|s: &str| s.parse::<i32>().map(|v| v.neg()).ok());

    or!(
        right!(skip_whitespace(), trim("-"), plain)
            .map(IntoLiteral::into_literal),
        var()
    )
}

fn parse_f32<'a>() -> impl StrParser<'a, Expr> {
    or!(
        float().map(Value::F32).map(Expr::Literal),
        var(),
    )
}

fn repeat_mode<'a>() -> impl StrParser<'a, Expr> {
    let forever = skip!("RepeatMode::Forever")
        .map(|_| RepeatMode::Forever.into_literal());

    let times = middle(
        trim("RepeatMode::Times("),
        argument(),
        trim(")")
    ).map(|times| fn_call_expr("RepeatMode::Times", vec![times]));

    let duration = middle(
        trim("RepeatMode::Duration("),
        argument(),
        trim(")")
    ).map(|duration| fn_call_expr("RepeatMode::Duration", vec![duration]));

    or!(forever, times, duration)
}

fn effect_timer<'a>() -> impl StrParser<'a, Expr> {
    let duration_or_var = or!(duration(), var());

    // raw int: ms with linear interpolation
    let from_u32 = parse_u32()
        .map(|ms| fn_call_expr(
            "EffectTimer::from_ms",
            vec![ms, Interpolation::Linear.into_literal()]
        ));

    let into_duration = or!(
        duration_or_var,
        parse_u32().map(|ms| fn_call_expr("Duration::from_millis", vec![ms])),
    );

    // tuple: (u32, interpolation)
    let from_tuple = tuplify!(
        right!(trim("("), into_duration),
        middle(trim(","), or!(interpolation(), var()), trim(")")),
    ).map(|(duration, interpolation)| {
        fn_call_expr("EffectTimer::new", vec![duration, interpolation])
    });

    // ctor: EffectTimer::new(duration, interpolation)
    let from_new = middle(
        trim("EffectTimer::new("),
        tuplify!(
            duration_or_var,
            right!(trim(","), interpolation()),
        ),
        trim(")")
    ).map(|(duration, interpolation)| {
        fn_call_expr("EffectTimer::new", vec![duration, interpolation])
    });

    // from ms: EffectTimer::from_ms(u32, Interpolation)
    let from_ms = tuplify!(
        right!(trim("EffectTimer::from_ms("), parse_u32()),
        middle(trim(","), or!(interpolation(), var()), trim(")")),
    ).map(|(ms, interpolation)| fn_call_expr("EffectTimer::from_ms", vec![ms, interpolation]));

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
            argument(),
            right!(trim(","), argument()),
            right!(trim(","), argument()),
            right!(trim(","), argument()),
        ),
        trim(")")
    );

    let raw = middle(
        right!(trim("Rect"), trim("{")),
        tuplify!(
            middle(trim("x:"), argument(), trim(",")),
            middle(trim("y:"), argument(), trim(",")),
            middle(trim("width:"), argument(), trim(",")),
            right!(trim("height:"), argument()),
        ),
        trim("}")
    );

    tuplify!(
        or!(new, raw),
        chained_fn_calls()
    ).map(|((x, y, w, h), self_fns)| {
        fn_call_chained_expr("Rect::new", vec![x, y, w, h], self_fns)
    })
}

fn margin<'a>() -> impl StrParser<'a, Expr> {
    // ctor: Margin::new(u32, u32)
    let new = middle(
        trim("Margin::new("),
        tuplify!(
            argument(),
            right!(trim(","), argument())
        ),
        trim(")")
    ).map(|(x, y)| fn_call_expr("Margin::new", vec![x, y]));

    // ctor: Margin::new(u32)
    let construct = middle(
        right!(trim("Margin"), trim("{")),
        tuplify!(
            middle(trim("horizontal:"), argument(), trim(",")),
            right!(trim("vertical:"), argument()),
        ),
        trim("}")
    ).map(|(horizontal, vertical)| fn_call_expr("Margin::new", vec![horizontal, vertical]));

    or!(new, construct)
}

fn duration<'a>() -> impl StrParser<'a, Expr> {
    // ctor from_millis
    let from_millis = middle(
        trim("Duration::from_millis("),
        argument(),
        trim(")"),
    ).map(|ms| fn_call_expr("Duration::from_millis", vec![ms]));

    // ctor from_secs_f32
    let from_secs = middle(
        trim("Duration::from_secs_f32("),
        argument(),
        trim(")"),
    ).map(|secs| fn_call_expr("Duration::from_secs_f32", vec![secs]));

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
    }).map(IntoLiteral::into_literal);

    or!(literal, var())
}

fn color<'a>() -> impl StrParser<'a, Expr> {
    // from_u32
    let from_u32 = middle(
        trim("Color::from_u32("),
        argument(),
        trim(")")
    ).map(|u32| fn_call_expr("Color::from_u32", vec![u32]));

    // rgb
    let rgb = middle(
        trim("Color::Rgb("),
        arguments(),
        trim(")")
    ).map(|rgb| fn_call_expr("Color::Rgb", rgb));

    // indexed
    let indexed = middle(
        trim("Color::Indexed("),
        argument(),
        trim(")")
    ).map(|idx| fn_call_expr("Color::Indexed", vec![idx]));

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
    }).map(IntoLiteral::into_literal);

    or!(from_u32, literal, rgb, indexed)
}

fn interpolation<'a>() -> impl StrParser<'a, Expr> {
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
    }).map(IntoLiteral::into_literal)
}

fn fn_call_expr(name: &str, args: Vec<Expr>) -> Expr {
    Expr::FnCall { call: FnCallInfo::new(name, args), self_fns: Vec::default() }
}

fn fn_call_chained_expr(name: &str, args: Vec<Expr>, self_fns: Vec<FnCallInfo>) -> Expr {
    Expr::FnCall { call: FnCallInfo::new(name, args), self_fns: self_fns.into() }
}

trait IntoLiteral {
    fn into_literal(self) -> Expr;
}

macro_rules! impl_into_literal {
    // single type implementation
    ($type:ty => $variant:ident) => {
        impl IntoLiteral for $type {
            fn into_literal(self) -> Expr {
                Expr::Literal(Value::$variant(self))
            }
        }
    };

    // multiple types implementation
    ($($type:ty => $variant:ident),+ $(,)?) => {
        $(
            impl_into_literal!($type => $variant);
        )+
    };
}

impl_into_literal! {
    CellFilter      => CellFilter,
    Color           => Color,
    Constraint      => Constraint,
    Direction       => Direction,
    Style           => Style,
    CompactString   => String,
    i32             => I32,
    u32             => U32,
    f32             => F32,
    crate::Duration => Duration,
    EffectTimer     => Timer,
    Modifier        => Modifier,
    Motion          => Motion,
    Rect            => Rect,
    Margin          => Margin,
    RepeatMode      => RepeatMode,
    Interpolation   => Interpolation,
}


#[cfg(test)]
mod tests {
    use crate::dsl::expressions::{Expr, FnCallInfo, Value};
    use crate::dsl::parsers::fn_call_expr;
    use crate::fx::RepeatMode;
    use crate::{CellFilter, Interpolation, Motion};
    use anpa::core::{parse, AnpaResult, ParserExt};
    use compact_str::ToCompactString;
    use ratatui::layout::Direction;
    use ratatui::style::{Color, Modifier};

    fn assert_expr_eq(
        result: AnpaResult<&str, Expr>,
        expected: Expr
    ) {
        assert_eq!(result.state, "", "Expected parser to consume the entire input");
        assert!(result.result.is_some());

        assert_eq!(
            format!("{:#?}", result.result.unwrap()),
            format!("{:#?}", expected),
        );
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
            parse(super::color(), input),
            fn_call_expr("Color::from_u32", vec![Expr::Literal(Value::U32(0x1d2021))])
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
            let input = format!("Color::{}", color.to_compact_string());
            assert_parser_eq(
                parse(super::color(), &input),
                Value::Color(color)
            );
        });

        let input = "Color::Indexed(3)";
        assert_expr_eq(
            parse(super::color(), input),
            fn_call_expr("Color::Indexed", vec![Expr::Literal(Value::U32(3))])
        );

        let input = "Color::Rgb(255, 127, 64)";
        assert_expr_eq(
            parse(super::color(), input),
            fn_call_expr("Color::Rgb", vec![
                Expr::Literal(Value::U32(255)),
                Expr::Literal(Value::U32(127)),
                Expr::Literal(Value::U32(64))
            ])
        );
    }

    #[test]
    fn test_margin() {
        let input = "Margin::new(10, 20)";
        assert_expr_eq(
            parse(super::margin(), input),
            fn_call_expr("Margin::new", vec![literal(Value::U32(10)), literal(Value::U32(20))])
        );

        let input = r#"Margin {
            horizontal: 10,
            vertical: 20
        }"#;
        assert_expr_eq(
            parse(super::margin(), input),
            fn_call_expr("Margin::new", vec![literal(Value::U32(10)), literal(Value::U32(20))])
        );
    }

    #[test]
    fn test_optional() {
        let input = "None";
        assert_expr_eq(
            parse(super::option(), input),
            Expr::Literal(Value::OptionNone)
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
            fn_call_expr("RepeatMode::Times", vec![literal(Value::U32(10))])
        );

        let input = "RepeatMode::Duration(Duration::from_millis(1000))";
        assert_expr_eq(
            parse(super::repeat_mode(), input),
            fn_call_expr("RepeatMode::Duration", vec![
                fn_call_expr("Duration::from_millis", vec![literal(Value::U32(1000))])
            ])
        );
    }

    #[test]
    fn test_modifiers() {
        [
            ("Modifier::BOLD", Modifier::BOLD),
            ("Modifier::DIM", Modifier::DIM),
            ("Modifier::ITALIC", Modifier::ITALIC),
            ("Modifier::UNDERLINED", Modifier::UNDERLINED),
            ("Modifier::SLOW_BLINK", Modifier::SLOW_BLINK),
            ("Modifier::RAPID_BLINK", Modifier::RAPID_BLINK),
            ("Modifier::REVERSED", Modifier::REVERSED),
            ("Modifier::HIDDEN", Modifier::HIDDEN),
            ("Modifier::CROSSED_OUT", Modifier::CROSSED_OUT),
        ].into_iter().for_each(|(input, expected)| {
            assert_expr_eq(
                parse(super::modifier(), input),
                Expr::Literal(Value::Modifier(expected))
            );
        });
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
        let result = parse(super::fn_call()
            .map(|call| Expr::FnCall { call, self_fns: Vec::default() }), input);

        assert_expr_eq(
            result,
            fn_call_expr("foo", vec![literal(Value::String("bar".into()))])
        );
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
                FnCallInfo::new("fg", vec![
                    fn_call_expr("Color::from_u32", vec![literal(Value::U32(0x1d2021))])
                ]),
                FnCallInfo::new("bg", vec![
                    fn_call_expr("Color::from_u32", vec![literal(Value::U32(0x1d2023))])
                ]),
                FnCallInfo::new("add_modifier", vec![Expr::Literal(Value::Modifier(Modifier::BOLD))])
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
                Expr::Fx { name: "yolo".to_compact_string(), self_fns: vec![], arguments: vec![
                    literal(Value::String("Hello".to_compact_string()))
                ]},
                Expr::Fx { name: "fubar".to_compact_string(), self_fns: vec![], arguments: vec![
                    literal(Value::String("World".to_compact_string()))
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
                name: "yolo".to_compact_string(),
                arguments: vec![literal(Value::String("Hello".to_compact_string()))],
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
                Expr::Fx { name: "foo".to_compact_string(), self_fns: vec![], arguments: vec![] },
                Expr::Fx { name: "bar".to_compact_string(), self_fns: vec![], arguments: vec![] }
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
                arguments: vec![
                    fn_call_expr("Color::from_u32", vec![Expr::Literal(Value::U32(0xFF0000))])
                ]
            }
        );

        // Test BgColor
        assert_cell_filter_eq(
            "BgColor(Color::from_u32(0x00FF00))",
            Expr::CellFilter {
                filter_type: "BgColor",
                arguments: vec![
                    fn_call_expr("Color::from_u32", vec![literal(Value::U32(0x00FF00))])
                ],
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
                arguments: vec![
                    fn_call_expr(
                        "Margin::new",
                        vec![literal(Value::U32(1)), literal(Value::U32(2))]
                    )
                ]
            }
        );

        // Test Outer margin
        assert_cell_filter_eq(
            "Outer(Margin::new(3, 4))",
            Expr::CellFilter {
                filter_type: "Outer",
                arguments: vec![
                    fn_call_expr(
                        "Margin::new",
                        vec![literal(Value::U32(3)), literal(Value::U32(4))]
                    )
                ]
            }
        );
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
                        arguments: vec![
                            fn_call_expr("Margin::new",
                                vec![literal(Value::U32(1)), literal(Value::U32(1))]
                            )
                        ]
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
                        arguments: vec![fn_call_expr(
                            "Margin::new",
                            vec![literal(Value::U32(1)), literal(Value::U32(1))]
                        )]
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
                        arguments: vec![
                            fn_call_expr("Margin::new",
                                vec![literal(Value::U32(1)), literal(Value::U32(1))]
                            )
                        ]
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
                                arguments: vec![
                                    fn_call_expr("Margin::new",
                                        vec![literal(Value::U32(1)), literal(Value::U32(1))]
                                    )
                                ]
                            },
                            Expr::CellFilter {
                                filter_type: "Outer",
                                arguments: vec![
                                    fn_call_expr("Margin::new",
                                        vec![literal(Value::U32(2)), literal(Value::U32(2))]
                                    )
                                ]
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
            fn_call_expr("Rect::new", vec![
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
            fn_call_expr("Rect::new", vec![
                literal(Value::U32(10)),
                literal(Value::U32(20)),
                literal(Value::U32(30)),
                literal(Value::U32(40))
            ])
        );
    }

    #[test]
    fn comments_are_discarded_by_parser() {
        let result = parse(super::argument(), r#"
            // This is a comment
            Color::from_u32(0x1d2021) // yolo
            // also a comment
        "#);
        assert_eq!(result.state, "", "Expected parser to consume the entire input");
        assert_eq!(
            result.result,
            Some(fn_call_expr("Color::from_u32", vec![literal(Value::U32(0x1d2021))]))
        );

        let result = parse(super::argument(), r#"
            // header comment
            /*
             * multi-line comment;
             * spanning two (4?) lines
             */
            /* foo */ /*bar*/ Duration::from_millis( /* 1st */ 1000 /* ms */) // yolo
            // trailing comment
        "#);

        assert_eq!(result.state, "", "Expected parser to consume the entire input");
        assert_eq!(
            result.result,
            Some(fn_call_expr("Duration::from_millis", vec![literal(Value::U32(1000))]))
        );
    }

    #[test]
    fn test_constraints() {
        [
            ("Constraint::Min(10)",        fn_call_expr("Constraint::Min",
                vec![literal(Value::U32(10))])),
            ("Constraint::Max(20)",        fn_call_expr("Constraint::Max",
                vec![literal(Value::U32(20))])),
            ("Constraint::Length(30)",     fn_call_expr("Constraint::Length",
                vec![literal(Value::U32(30))])),
            ("Constraint::Percentage(40)", fn_call_expr("Constraint::Percentage",
                vec![literal(Value::U32(40))])),
            ("Constraint::Fill(50)",       fn_call_expr("Constraint::Fill",
                vec![literal(Value::U32(50))])),
            ("Constraint::Ratio(60, 61)",  fn_call_expr("Constraint::Ratio",
                vec![literal(Value::U32(60)), literal(Value::U32(61))])),
        ].into_iter().for_each(|(input, expected)| {
            assert_expr_eq(
                parse(super::constraint(), input),
                expected
            );
        });
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
        let expected = fn_call_expr("Duration::from_millis", vec![literal(Value::U32(1000))]);
        assert_eq!(result, expected);

        let input = "Duration::from_secs_f32(0.5)";
        let result = parse(super::duration(), input).result.unwrap();
        let expected = fn_call_expr("Duration::from_secs_f32", vec![literal(Value::F32(0.5))]);
        assert_eq!(result, expected);

        let input = r#"Duration::from_millis(
                           321
                       )"#;
        let result = parse(super::duration(), input).result.unwrap();
        let expected = fn_call_expr("Duration::from_millis", vec![literal(Value::U32(321))]);
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
        assert_eq!(result, Some(Expr::Var { name: "my_var".to_compact_string(), self_fns: vec![] }));
    }

    #[test]
    fn parse_array_ref() {
        let input = "&[\"Hello, World!\", 1337, 3.14, (1000, SineIn)]";
        let result = parse(super::array_ref(), input);
        let expected = Expr::ArrayRef(vec![
            Expr::Literal(Value::String("Hello, World!".to_compact_string())),
            Expr::Literal(Value::U32(1337)),
            Expr::Literal(Value::F32(3.14)),
            fn_call_expr("EffectTimer::new", vec![
                fn_call_expr("Duration::from_millis", vec![literal(Value::U32(1000))]),
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
            Expr::Literal(Value::String("Hello, World!".to_compact_string())),
            Expr::Literal(Value::U32(1337)),
            Expr::Literal(Value::F32(3.14)),
            fn_call_expr("EffectTimer::new", vec![
                fn_call_expr("Duration::from_millis", vec![literal(Value::U32(1000))]),
                literal(Value::Interpolation(Interpolation::SineIn))
            ])
        ]);

        assert_expr_eq(result, expected);
    }

    #[test]
    fn parse_effect_timer() {
        let input = "EffectTimer::from_ms(1000, Interpolation::Linear)";
        let result = parse(super::effect_timer(), input).result.unwrap();
        let expected = fn_call_expr("EffectTimer::from_ms", vec![
            literal(Value::U32(1000)),
            literal(Value::Interpolation(Interpolation::Linear))
        ]);
        assert_eq!(result, expected);

        let input = "EffectTimer::new(Duration::from_millis(1000), Linear)";
        let result = parse(super::effect_timer(), input).result.unwrap();
        let expected = fn_call_expr("EffectTimer::new", vec![
            fn_call_expr("Duration::from_millis", vec![literal(Value::U32(1000))]),
            literal(Value::Interpolation(Interpolation::Linear))
        ]);
        assert_eq!(result, expected);

        let input = "EffectTimer::new(Duration::from_secs_f32(0.5), Linear)";
        let result = parse(super::effect_timer(), input).result.unwrap();
        let expected = fn_call_expr("EffectTimer::new", vec![
            fn_call_expr("Duration::from_secs_f32", vec![literal(Value::F32(0.5))]),
            literal(Value::Interpolation(Interpolation::Linear))
        ]);
        assert_eq!(result, expected);

        let input = "(1337, Reverse)";
        let result = parse(super::effect_timer(), input).result.unwrap();
        let expected = fn_call_expr("EffectTimer::new", vec![
            fn_call_expr("Duration::from_millis", vec![literal(Value::U32(1337))]),
            literal(Value::Interpolation(Interpolation::Reverse))
        ]);
        assert_eq!(result, expected);

        let input = "(Duration::from_millis(1337), Reverse)";
        let result = parse(super::effect_timer(), input).result.unwrap();
        let expected = fn_call_expr("EffectTimer::new", vec![
            fn_call_expr("Duration::from_millis", vec![literal(Value::U32(1337))]),
            literal(Value::Interpolation(Interpolation::Reverse))
        ]);
        assert_eq!(result, expected);

        let input = "1234";
        let result = parse(super::effect_timer(), input).result.unwrap();
        let expected = fn_call_expr("EffectTimer::from_ms", vec![
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
            Value::String("Hello, World!".to_compact_string())
        );

        // let input = r#""Hello, \"World!\"""#;
        let input = "\"Hello, \\\"World!\\\"\"";
        assert_parser_eq(
            parse(super::string_literal(), input),
            Value::String("Hello, \"World!\"".to_compact_string())
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
            Value::String("Hello, World!".to_compact_string())
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
            fn_call_expr("EffectTimer::from_ms", vec![
                literal(Value::U32(1000)),
                literal(Value::Interpolation(Interpolation::Linear))
            ])
        );

        let input = "Duration::from_millis(1000)";
        assert_expr_eq(
            parse(super::argument(), input),
            fn_call_expr("Duration::from_millis", vec![literal(Value::U32(1000))])
        );
    }

    #[test]
    fn parse_arguments() {
        let input = "\"Hello, World!\", 1337, 3.14, (1000, SineIn)";
        assert_eq!(
            parse(super::arguments(), input).result,
            Some(vec![
                Expr::Literal(Value::String("Hello, World!".to_compact_string())),
                Expr::Literal(Value::U32(1337)),
                Expr::Literal(Value::F32(3.14)),
                fn_call_expr("EffectTimer::new", vec![
                    fn_call_expr("Duration::from_millis", vec![literal(Value::U32(1000))]),
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
            name: "coalesce".to_compact_string(),
            arguments: vec![
                fn_call_expr("Duration::from_millis", vec![literal(Value::U32(220))]),
            ],
            self_fns: vec![]
        };
        assert_eq!(result, Some(expected));

        let input = "fx::dissolve((Duration::from_millis(220), ElasticOut))";
        let result = parse(super::effect(), input).result;
        let expected = Expr::Fx {
            name: "dissolve".to_compact_string(),
            self_fns: vec![],
            arguments: vec![
                fn_call_expr("EffectTimer::new", vec![
                    fn_call_expr("Duration::from_millis", vec![literal(Value::U32(220))]),
                    Expr::Literal(Value::Interpolation(Interpolation::ElasticOut))
                ])
            ]};
        assert_eq!(result, Some(expected));

        let input = "fx::ping_pong(fx::coalesce((500, CircOut)))";
        let result = parse(super::effect(), input).result;
        let expected = Expr::Fx {
            name: "ping_pong".to_compact_string(),
            self_fns: vec![],
            arguments: vec![
                Expr::Fx {
                    name: "coalesce".to_compact_string(),
                    self_fns: vec![],
                    arguments: vec![
                        fn_call_expr("EffectTimer::new", vec![
                            fn_call_expr("Duration::from_millis", vec![literal(Value::U32(500))]),
                            Expr::Literal(Value::Interpolation(Interpolation::CircOut))
                        ])
                    ]
                }
            ]
        };
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_layout_parser() {
        // Test Layout::new
        let input = "Layout::new(Direction::Horizontal, [Length(1), Percentage(50)])";
        assert_expr_eq(
            parse(super::layout(), input),
            Expr::Layout {
                expr: Box::new(fn_call_expr("Layout::new", vec![
                    literal(Value::Direction(Direction::Horizontal)),
                    Expr::Array(vec![
                        fn_call_expr("Constraint::Length", vec![literal(Value::U32(1))]),
                        fn_call_expr("Constraint::Percentage", vec![literal(Value::U32(50))])
                    ])
                ])),
                self_fns: vec![]
            }
        );

        // Test Layout::horizontal with constraints
        let input = "Layout::horizontal([Length(1), Percentage(50)])";
        assert_expr_eq(
            parse(super::layout(), input),
            Expr::Layout {
                expr: Box::new(fn_call_expr("Layout::horizontal", vec![
                    Expr::Array(vec![
                        fn_call_expr("Constraint::Length", vec![literal(Value::U32(1))]),
                        fn_call_expr("Constraint::Percentage", vec![literal(Value::U32(50))])
                    ])
                ])),
                self_fns: vec![]
            }
        );

        // Test Layout::vertical with method chaining
        let input = "Layout::vertical([Min(5)]).margin(1).spacing(2)";
        assert_expr_eq(
            parse(super::layout(), input),
            Expr::Layout {
                expr: Box::new(fn_call_expr("Layout::vertical", vec![
                    Expr::Array(vec![
                        fn_call_expr("Constraint::Min", vec![literal(Value::U32(5))])
                    ])
                ])),
                self_fns: vec![
                    FnCallInfo::new("margin", vec![literal(Value::U32(1))]),
                    FnCallInfo::new("spacing", vec![literal(Value::U32(2))])
                ]
            }
        );

        // Test complex nested layout
        let input = r#"Layout::horizontal([
            Constraint::Length(1),
            Constraint::Percentage(80),
            Constraint::Ratio(1, 3)
        ]).spacing(1).direction(Direction::Vertical)"#;

        assert_expr_eq(
            parse(super::layout(), input),
            Expr::Layout {
                expr: Box::new(fn_call_expr("Layout::horizontal", vec![
                    Expr::Array(vec![
                        fn_call_expr("Constraint::Length", vec![literal(Value::U32(1))]),
                        fn_call_expr("Constraint::Percentage", vec![literal(Value::U32(80))]),
                        fn_call_expr("Constraint::Ratio", vec![
                            literal(Value::U32(1)),
                            literal(Value::U32(3))
                        ])
                    ])
                ])),
                self_fns: vec![
                    FnCallInfo::new("spacing", vec![literal(Value::U32(1))]),
                    FnCallInfo::new("direction", vec![
                        literal(Value::Direction(Direction::Vertical))
                    ])
                ]
            }
        );

        // Test with variables
        let input = "Layout::horizontal(constraints).margin(spacing)";
        assert_expr_eq(
            parse(super::layout(), input),
            Expr::Layout {
                expr: Box::new(fn_call_expr("Layout::horizontal", vec![
                    Expr::Var { name: "constraints".to_compact_string(), self_fns: vec![] }
                ])),
                self_fns: vec![
                    FnCallInfo::new("margin", vec![Expr::Var {
                        name: "spacing".to_compact_string(),
                        self_fns: vec![]
                    }])
                ]
            }
        );
    }

    #[test]
    fn test_layout_parser_edge_cases() {
        // Test empty constraint array
        let input = "Layout::vertical([])";
        assert_expr_eq(
            parse(super::layout(), input),
            Expr::Layout {
                expr: Box::new(fn_call_expr("Layout::vertical", vec![
                    Expr::Array(vec![])
                ])),
                self_fns: vec![]
            }
        );

        // Test multiple method chains with complex arguments
        let input = "Layout::new(Direction::Horizontal).constraints([Length(1)]).spacing(2).margin(3)";
        assert_expr_eq(
            parse(super::layout(), input),
            Expr::Layout {
                expr: Box::new(fn_call_expr("Layout::new", vec![
                    literal(Value::Direction(Direction::Horizontal))
                ])),
                self_fns: vec![
                    FnCallInfo::new("constraints", vec![
                        Expr::Array(vec![
                            fn_call_expr("Constraint::Length", vec![literal(Value::U32(1))])
                        ])
                    ]),
                    FnCallInfo::new("spacing", vec![literal(Value::U32(2))]),
                    FnCallInfo::new("margin", vec![literal(Value::U32(3))])
                ]
            }
        );

        // Test whitespace handling
        let input = r#"Layout::horizontal( [ Length ( 1 ) ] )
            .margin ( 2 )
            .spacing ( 3 )"#;

        assert_expr_eq(
            parse(super::layout(), input),
            Expr::Layout {
                expr: Box::new(fn_call_expr("Layout::horizontal", vec![
                    Expr::Array(vec![
                        fn_call_expr("Constraint::Length", vec![literal(Value::U32(1))])
                    ])
                ])),
                self_fns: vec![
                    FnCallInfo::new("margin", vec![literal(Value::U32(2))]),
                    FnCallInfo::new("spacing", vec![literal(Value::U32(3))])
                ]
            }
        );
    }

    #[test]
    fn parse_simple_let() {
        let input = "let x = 42;";
        let result = parse(super::let_binding(), input).result.unwrap();

        match result {
            Expr::LetBinding { name, let_expr: value } => {
                assert_eq!(name, "x");
                assert!(matches!(*value, Expr::Literal(Value::U32(42))));
            }
            _ => panic!("Expected LetBinding"),
        }
    }

    #[test]
    fn parse_let_with_complex_expr() {
        let input = "let duration = Duration::from_millis(500);";
        let result = parse(super::let_binding(), input).result.unwrap();

        match result {
            Expr::LetBinding { name, .. } => {
                assert_eq!(name, "duration");
            }
            _ => panic!("Expected LetBinding"),
        }
    }

    #[test]
    fn parse_let_with_spacing() {
        let input = "let    my_var   =    42   ;";
        let result = parse(super::let_binding(), input).result.unwrap();

        match result {
            Expr::LetBinding { name, let_expr } => {
                assert_eq!(name, "my_var");
                assert!(matches!(*let_expr, Expr::Literal(Value::U32(42))));
            }
            _ => panic!("Expected LetBinding"),
        }
    }

    #[test]
    fn parse_let_with_effect() {
        let input = "let fade = fx::fade_to(Color::Red, 1000);";
        let result = parse(super::let_binding(), input).result.unwrap();

        match result {
            Expr::LetBinding { name, .. } => {
                assert_eq!(name, "fade");
            }
            _ => panic!("Expected LetBinding"),
        }
    }

    fn literal(value: Value) -> Expr {
        Expr::Literal(value)
    }
}
