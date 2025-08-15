#![allow(clippy::std_instead_of_alloc, clippy::std_instead_of_core)]

use std::{
    any::{type_name, Any},
    cell::RefCell,
    collections::BTreeMap,
    fmt,
};

use compact_str::{CompactString, ToCompactString};

use crate::dsl::{
    arguments::FromDslExpr,
    expressions::{Expr, ExprSpan},
    Arguments, DslError, EffectDsl,
};

pub(super) struct DslEnv {
    globals: BTreeMap<CompactString, Box<dyn Any>>,
    locals: RefCell<BTreeMap<CompactString, Box<Expr>>>,
}

impl DslEnv {
    pub(super) fn new() -> Self {
        Self {
            globals: BTreeMap::new(),
            locals: RefCell::default(),
        }
    }

    pub fn bind<K, T>(self, name: K, value: T) -> Self
    where
        K: Into<CompactString>,
        T: 'static,
    {
        let mut this = self;
        this.globals.insert(name.into(), Box::new(value));
        this
    }

    pub(super) fn bind_local<K>(&self, name: K, expr: Expr)
    where
        K: Into<CompactString>,
    {
        let name = name.into();
        let span = expr.span();

        let expr = Expr::LetBinding { name: name.clone(), let_expr: Box::new(expr), span };
        self.locals
            .borrow_mut()
            .insert(name, Box::new(expr));
    }

    pub(super) fn bound_var<'dsl, T: Clone + FromDslExpr + 'static>(
        &'dsl self,
        dsl: &'dsl EffectDsl,
        name: impl Into<CompactString>,
        use_site: ExprSpan,
    ) -> Result<T, DslError> {
        let name = name.into();
        if let Some(expr) = self.let_expr(name.as_str()) {
            let mut args = Arguments::new([expr].into(), dsl, self, use_site);
            Ok(match FromDslExpr::from_expr(&mut args) {
                Ok(v) => Ok(v),
                Err(DslError::WrongArgumentType { expected, actual, .. }) => {
                    // avoid reporting the span of the declaration of the variable,
                    // we want the use site
                    Err(DslError::WrongArgumentType { expected, actual, location: use_site })
                },
                e => e,
            })?
        } else {
            self.bound_global(name.as_str(), use_site)
        }
    }

    pub(super) fn bound_global<K, T>(&self, name: K, span: ExprSpan) -> Result<T, DslError>
    where
        K: AsRef<str>,
        T: Clone + 'static,
    {
        self.globals
            .get(name.as_ref())
            .ok_or_else(|| DslError::UnknownArgument { name: name.as_ref().into(), location: span })
            .and_then(|v| {
                v.downcast_ref()
                    .cloned()
                    .ok_or_else(|| DslError::NoSuchVariable {
                        name: name.as_ref().to_compact_string(),
                        expected: type_name::<T>(),
                        location: span,
                    })
            })
    }

    pub(super) fn let_expr(&self, name: impl AsRef<str>) -> Option<Expr> {
        if let Some(value) = self.locals.borrow().get(name.as_ref()) {
            if let Expr::LetBinding { let_expr, .. } = *value.clone() {
                Some(*let_expr)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl fmt::Debug for DslEnv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DslEnv")
            .field("globals", &self.globals)
            .field("let_exprs", &self.locals)
            .finish()
    }
}
