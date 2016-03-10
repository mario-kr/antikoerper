
use std::collections::BinaryHeap;
use std::io::Read;

use toml;
use item::Item;

/// The Configuration of Antikoerper
#[derive(Debug, Clone)]
pub struct Config {
    items: BinaryHeap<Item>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ConfigErrorKind {
    IoError,
    TomlError,
}

#[derive(Debug)]
pub struct ConfigError {
    kind: ConfigErrorKind,
    cause: Option<Box<::std::error::Error>>
}

impl ::std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match self.kind {
            ConfigErrorKind::IoError => self.cause.as_ref().unwrap().fmt(f),
            ConfigErrorKind::TomlError => self.cause.as_ref().unwrap().fmt(f),
        }
    }
}

impl<T: ::std::error::Error + 'static> From<T> for ConfigError {
    fn from(e: T) -> Self {
        ConfigError {
            kind: ConfigErrorKind::IoError,
            cause: Some(Box::new(e)),
        }
    }
}

pub fn load(r: &mut Read) -> Result<Config, ConfigError> {
    let content = {
        let mut buffer = String::new();
        try!(r.read_to_string(&mut buffer));
        buffer
    };


    let mut parser = toml::Parser::new(&content);
    let parsed = if let Some(t) = parser.parse() {
        t
    } else {
        return Err(ConfigError::from(parser.errors[0].clone()));
    };

    Ok(Config {
        items: BinaryHeap::new()
    })
}

#[cfg(test)]
mod tests {
    use conf;

    #[test]
    fn load() {
        let data = "[items.os.uptime]
         shell = 'cat /proc/uptime | cut -d\' \' -f1'

         [items.os.usage]
         shell = 'cat /proc/loadavg | cut -d\' \' -f1'";

        let config = conf::load(&mut data.as_bytes());
        assert_eq!(config.items.len(), 2);

    }
}
