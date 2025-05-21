use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum CompileError {
    General {
        message: String,
        source: Box<dyn std::error::Error>,
    },
    InvalidFieldName {
        field_name: String,
    },
    MissingSub0 {
        obj_num: u32,
    },
    MissingSub1 {
        obj_num: u32,
    },
    MissingSub {
        obj_num: u32,
        sub_num: u32,
    },
    ParseInt {
        message: String,
        source: std::num::ParseIntError,
    },
    ParseFloat {
        message: String,
        source: std::num::ParseFloatError,
    },
    #[snafu(display("Error parsing toml: {}. Toml error: {}", message, source.to_string()))]
    ParseToml {
        message: String,
        source: toml::de::Error,
    },
    DefaultValueTooLong {
        message: String,
    },
    DefaultValueTypeMismatch {
        message: String,
    },
    NotRunViaCargo,
    Io {
        source: std::io::Error,
    },
}
