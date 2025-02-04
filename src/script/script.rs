use std::any::Any;
use std::collections::BTreeMap;
use crate::Effect;
use crate::script::parser::{FxArg};


pub struct ScriptDeserializer {
    name: &'static str,
    deserialize: Box<dyn Fn(&Self, &[FxArg]) -> Option<Effect>>,
}


#[derive(Default)]
pub struct ScriptContext {
    deserializers: Vec<ScriptDeserializer>,
}

pub struct ScriptEnv {
    bound_variables: BTreeMap<&'static str, Box<dyn Any>>,
}


impl ScriptEnv {
    pub(crate) fn new() -> Self {
        Self {
            bound_variables: BTreeMap::new(),
        }
    }

    pub fn bind_variable<T: 'static>(self, name: &'static str, value: T) -> Self {
        let mut this = self;
        this.bound_variables.insert(name, Box::new(value));
        this
    }

    pub fn get_variable<T: 'static>(&self, name: &'static str) -> Option<&T> {
        self.bound_variables.get(name).and_then(|v| v.downcast_ref())
    }
}

impl ScriptDeserializer {
    pub fn new(
        name: &'static str,
        deserialize: impl Fn(&Self, &[FxArg]) -> Option<Effect> + 'static
    ) -> Self {
        Self {
            name,
            deserialize: Box::new(deserialize),
        }
    }
}

impl ScriptContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn execute(
        &self,
        env: &mut ScriptEnv,
        input: &str,
    ) -> Option<Effect> {
        unimplemented!("parse input")
    }

    pub fn add_deserializer(&mut self, deserializer: ScriptDeserializer) {
        self.deserializers.push(deserializer);
    }

    fn deserialize(&self, input: FxArg) -> Option<Effect> {
        match input {
            FxArg::Fx { name, parameters } => self.deserializers
                .iter()
                .find(|d| d.name == name)
                .map(|d| (d.deserialize)(d, &parameters))
                .flatten(),
            _ => None
        }
    }

    pub fn register_deserializer(
        &mut self,
        name: &'static str,
        deserialize: impl Fn(&Self, &[FxArg]) -> Option<Effect> + 'static
    ) {
        self.add_deserializer(ScriptDeserializer::new(name, deserialize));
    }
}