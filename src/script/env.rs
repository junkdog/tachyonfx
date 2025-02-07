use std::any::Any;
use std::collections::BTreeMap;

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

    pub fn get<K, T>(&self, name: K) -> Option<&T>
    where
        K: AsRef<str>,
        T: 'static,
    {
        self.bound_variables
            .get(name.as_ref())
            .and_then(|v| v.downcast_ref())
    }
}