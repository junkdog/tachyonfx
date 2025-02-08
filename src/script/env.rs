use std::any::{type_name, Any};
use std::collections::BTreeMap;
use crate::script::ScriptError;

pub struct ScriptEnv {
    bound_variables: BTreeMap<String, Box<dyn Any>>,
}


impl ScriptEnv {
    pub(crate) fn new() -> Self {
        Self {
            bound_variables: BTreeMap::new(),
        }
    }

    pub fn bind<K, T>(self, name: K, value: T) -> Self
    where
        K: Into<String>,
        T: 'static,
    {
        let mut this = self;
        this.bound_variables.insert(name.into(), Box::new(value));
        this
    }

    pub(super) fn get<K, T>(&self, name: K) -> Result<&T, ScriptError>
    where
        K: AsRef<str>,
        T: 'static,
    {
        match self.bound_variables.get(name.as_ref()) {
            Some(v) => Ok(v),
            None => Err(ScriptError::UnknownArgument {
                name: name.as_ref().to_string(),
            }),
        }.and_then(|v| v.downcast_ref().ok_or_else(|| ScriptError::NoSuchVariable {
            position: 0, // todo: resolve position
            name: name.as_ref().to_string(),
            expected: type_name::<T>(),
        }))
    }
}