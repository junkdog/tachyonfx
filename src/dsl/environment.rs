use crate::dsl::expressions::Expr;
use crate::dsl::DslError;
use compact_str::{CompactString, ToCompactString};
use std::any::{type_name, Any};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;

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

    pub(super) fn bind_local<K>(
        &self, name: K,
        expr: Expr,
    ) where K: Into<CompactString> {
        let name = name.into();
        let expr = Expr::LetBinding {
            name: name.clone(),
            let_expr: Box::new(expr),
        };
        self.locals.borrow_mut().insert(name, Box::new(expr));
    }

    pub(super) fn bound_var<K, T>(&self, name: K) -> Result<T, DslError>
    where
        K: AsRef<str>,
        T: Clone + 'static,
    {
        self.globals.get(name.as_ref())
            .ok_or_else(|| DslError::UnknownArgument { name: name.as_ref().into() })
            .and_then(|v| v.downcast_ref().map(|v: &T| v.clone()).ok_or_else(||
                DslError::NoSuchVariable {
                    name: name.as_ref().to_compact_string(),
                    expected: type_name::<T>()
                }
            ))
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