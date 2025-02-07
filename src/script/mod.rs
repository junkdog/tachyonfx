mod parser;
mod script;
mod args;
mod env;

pub enum ScriptError {

    UnknownFxName(String),
}

pub enum ArgsError {
    MissingArg(String),
    WrongArgType(String),
}