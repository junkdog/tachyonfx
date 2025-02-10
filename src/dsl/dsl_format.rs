use crate::fx::RepeatMode;

pub trait DslFormat {
    fn dsl_format(&self) -> String;
}

impl DslFormat for RepeatMode {
    fn dsl_format(&self) -> String {
        match self {
            RepeatMode::Forever =>
                "RepeatMode::Forever".to_string(),
            RepeatMode::Times(n) =>
                format!("RepeatMode::Times({})", n),
            RepeatMode::Duration(d) =>
                format!("RepeatMode::Duration(Duration::from_millis({}))", d.as_millis()),
        }
    }
}

