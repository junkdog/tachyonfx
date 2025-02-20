use compact_str::CompactString;
use ratatui::layout::{Layout, Rect};
use ratatui::style::Style;
use crate::dsl::{Arguments, DslError, EffectDsl};
use crate::dsl::environment::DslEnv;
use crate::dsl::expressions::FnCallInfo;
use crate::Effect;

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
            let mut args = Arguments::new(f.args.into(), context, vars);
            Self::apply_fn(this, name, &mut args)
        })
    }

    fn apply_fn(
        object: Self,
        name: CompactString,
        args: &mut Arguments<'_>
    ) -> Result<Self, DslError>;
}

impl ChainableMethods for Effect {
    fn apply_fn(
        effect: Self,
        name: CompactString,
        args: &mut Arguments<'_>
    ) -> Result<Self, DslError> {
        Ok(match name.as_str() {
            "with_area"              => effect.with_area(args.rect()?),
            "with_filter" | "filter" => effect.with_filter(args.cell_filter()?),
            _                        => Err(DslError::UnknownFunction { name })?,
        })
    }
}

impl ChainableMethods for Layout {
    fn apply_fn(
        layout: Self,
        name: CompactString,
        args: &mut Arguments<'_>
    ) -> Result<Self, DslError> {
        Ok(match name.as_str() {
            "constraints"       => layout.constraints(args.array(Arguments::constraint)?),
            "margin"            => layout.margin(args.read_u16()?),
            "horizontal_margin" => layout.horizontal_margin(args.read_u16()?),
            "vertical_margin"   => layout.vertical_margin(args.read_u16()?),
            "spacing"           => layout.spacing(args.read_u16()?),
            _                   => Err(DslError::UnknownFunction { name })?,
        })
    }
}

impl ChainableMethods for Style {
    fn apply_fn(
        style: Self,
        name: CompactString,
        args: &mut Arguments<'_>
    ) -> Result<Self, DslError> {
        Ok(match name.as_str() {
            "fg"           => style.fg(args.color()?),
            "bg"           => style.bg(args.color()?),
            "add_modifier" => style.add_modifier(args.modifier()?),
            _              => Err(DslError::UnknownFunction { name })?,
        })
    }
}

impl ChainableMethods for Rect {
    fn apply_fn(
        rect: Self,
        name: CompactString,
        args: &mut Arguments<'_>
    ) -> Result<Self, DslError> {
        Ok(match name.as_str() {
            "clamp"        => rect.clamp(args.rect()?),
            "inner"        => rect.inner(args.margin()?),
            "interesction" => rect.intersection(args.rect()?),
            "union"        => rect.union(args.rect()?),
            _              => Err(DslError::UnknownFunction { name })?,
        })
    }
}
