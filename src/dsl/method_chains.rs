use crate::dsl::environment::DslEnv;
use crate::dsl::expressions::FnCallInfo;
use crate::dsl::{Arguments, DslError, EffectDsl};
use crate::Effect;
use ratatui::layout::{Layout, Rect};
use ratatui::style::Style;
use crate::fx::IntoTemporaryEffect;

/// A trait for types that support method chaining in the tachyonfx DSL.
///
/// This trait enables types to handle method chains in DSL expressions by providing
/// a mechanism to fold a sequence of function calls into a final result. It's primarily
/// used for applying sequential modifications to objects like Effects, Layouts, and Styles.
///
/// # Implementation Notes
///
/// Implementors only need to provide the `apply_fn` method, which handles individual
/// function applications. The default `fold_fns` implementation will handle iterating
/// over multiple chained methods.
/// ```
pub(super) trait ChainableMethods where Self: Sized {
    fn fold_fns<'dsl>(
        self,
        self_fns: Vec<FnCallInfo>,
        context: &'dsl EffectDsl,
        vars: &'dsl DslEnv,
    ) -> Result<Self, DslError> {
        self_fns.into_iter().try_fold(self, |this, f| {
            let name = f.name;
            let mut args = Arguments::new(f.args.into(), context, vars, f.span);
            let result = Self::apply_fn(this, name.as_str(), &mut args);
            if args.remaining_arg_count() > 0 {
                Err(DslError::TooManyArguments {
                    name,
                    count: args.remaining_arg_count(),
                    location: args.span()
                })
            } else {
                result
            }
        })
    }

    fn apply_fn(
        object: Self,
        name: &str,
        args: &mut Arguments<'_>
    ) -> Result<Self, DslError>;
}

impl ChainableMethods for Effect {
    fn apply_fn(
        effect: Self,
        name: &str,
        args: &mut Arguments<'_>
    ) -> Result<Self, DslError> {
        Ok(match name {
            "clone"                  => effect.clone(),
            "reversed"               => effect.reversed(),
            "with_area"              => effect.with_area(args.rect()?),
            "with_color_space"       => effect.with_color_space(args.color_space()?),
            "with_duration"          => effect.with_duration(args.duration()?),
            "with_filter" | "filter" => effect.with_filter(args.cell_filter()?),
            _                        => Err(DslError::UnknownFunction {
                name: name.into(),
                location: args.span()
            })?,
        })
    }
}

impl ChainableMethods for Layout {
    fn apply_fn(
        layout: Self,
        name: &str,
        args: &mut Arguments<'_>
    ) -> Result<Self, DslError> {
        Ok(match name {
            "clone"             => layout.clone(),
            "constraints"       => layout.constraints(args.array(Arguments::constraint)?),
            "margin"            => layout.margin(args.read_u16()?),
            "horizontal_margin" => layout.horizontal_margin(args.read_u16()?),
            "vertical_margin"   => layout.vertical_margin(args.read_u16()?),
            "spacing"           => layout.spacing(args.read_u16()?),
            _                   => Err(DslError::UnknownFunction {
                name: name.into(),
                location: args.span()
            })?,
        })
    }
}

impl ChainableMethods for Style {
    fn apply_fn(
        style: Self,
        name: &str,
        args: &mut Arguments<'_>
    ) -> Result<Self, DslError> {
        Ok(match name {
            "clone"           => style,
            "fg"              => style.fg(args.color()?),
            "bg"              => style.bg(args.color()?),
            "add_modifier"    => style.add_modifier(args.modifier()?),
            "remove_modifier" => style.remove_modifier(args.modifier()?),
            _                 => Err(DslError::UnknownFunction {
                name: name.into(),
                location: args.span()
            })?,
        })
    }
}

impl ChainableMethods for Rect {
    fn apply_fn(
        rect: Self,
        name: &str,
        args: &mut Arguments<'_>
    ) -> Result<Self, DslError> {
        Ok(match name {
            "clone"        => rect,
            "clamp"        => rect.clamp(args.rect()?),
            "inner"        => rect.inner(args.margin()?),
            "intersection" => rect.intersection(args.rect()?),
            "union"        => rect.union(args.rect()?),
            "offset"       => rect.offset(args.offset()?),
            _              => Err(DslError::UnknownFunction {
                name: name.into(),
                location: args.span()
            })?,
        })
    }
}
