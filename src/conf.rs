use rustc_serialize::Decodable;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use toml::{Decoder, Parser};

pub type ConfigResult = Result<Configuration, ConfigError>;

#[derive(Debug)]
enum ConfigErrorKind {
    FSError,
    InvalidConfigFile,
    MissingAntikoerperSection,
}

#[derive(Debug)]
pub struct ConfigError {
    kind: ConfigErrorKind,
    cause: Option<Box<Error>>
}

impl ConfigError {
    fn new(k: ConfigErrorKind, c: Option<Box<Error>>) -> ConfigError {
        ConfigError {
            kind: k,
            cause: c,
        }
    }
}

impl Error for ConfigError {
    fn description(&self) -> &str {
        match self.kind {
            ConfigErrorKind::FSError                    => "Filesystem error",
            ConfigErrorKind::InvalidConfigFile          => "Could not parse config file",
            ConfigErrorKind::MissingAntikoerperSection  => "Config file misses the Antikörper section",
        }
    }
}

impl ::std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct Configuration {
    server_name: String,
}

impl Configuration {
    pub fn from_file(file: &mut File) -> ConfigResult {
        let mut input = String::new();
        let fs = file.read_to_string(&mut input);
        match fs {
            Err(e) => {
                return Err(ConfigError::new(ConfigErrorKind::FSError, Some(Box::new(e))));
            },
            _ => {}
        };
        let toml = match Parser::new(&input).parse() {
            None => {
                return Err(ConfigError::new(ConfigErrorKind::InvalidConfigFile, None));
            },
            Some(mut toml) => {
                match toml.remove("antikoerper") {
                    Some(t) => t,
                    None => {
                        return Err(
                            ConfigError::new(ConfigErrorKind::MissingAntikoerperSection, None)
                            );
                    }
                }
            }
        };
        let mut decoder = Decoder::new(toml);
        match Decodable::decode(&mut decoder) {
            Err(e) => {
                return Err(ConfigError::new(ConfigErrorKind::InvalidConfigFile, Some(Box::new(e))));
            },
            Ok(c) => Ok(c)
        }
    }
}
