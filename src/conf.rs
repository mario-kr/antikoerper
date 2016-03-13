
use std::collections::BinaryHeap;
use std::io::Read;

use toml;
use item::Item;

/// The Configuration of Antikoerper
#[derive(Debug, Clone)]
pub struct Config {
    pub items: BinaryHeap<Item>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ConfigErrorKind {
    IoError,
    TomlError,
    MissingItems,
    ErrorItems,
}

#[derive(Debug)]
pub struct ConfigError {
    kind: ConfigErrorKind,
    cause: Option<Box<::std::error::Error>>,
}

impl ::std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match self.kind {
            ConfigErrorKind::IoError
                | ConfigErrorKind::TomlError => self.cause.as_ref().unwrap().fmt(f),
            ConfigErrorKind::MissingItems => write!(f, "no items section"),
            ConfigErrorKind::ErrorItems => write!(f, "some items have errors"),
        }
    }
}

impl From<::std::io::Error> for ConfigError {
    fn from(e: ::std::io::Error) -> Self {
        ConfigError {
            kind: ConfigErrorKind::IoError,
            cause: Some(Box::new(e)),
        }
    }
}

impl From<toml::ParserError> for ConfigError {
    fn from(e: toml::ParserError) -> Self {
        ConfigError {
            kind: ConfigErrorKind::TomlError,
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

    println!("{:#?}", parsed);

    let items = match parsed.get("items") {
        Some(&toml::Value::Array(ref t)) => t,
        _ => return Err(ConfigError {
            kind: ConfigErrorKind::MissingItems,
            cause: None
        })
    }.iter().filter_map(|v| {
        if let toml::Value::Table(ref v) = *v {
            Some(Item::from_toml(v))
        } else {
            None
        }
    }).collect::<Vec<_>>();

    for err in items.iter().filter(|x| x.is_err()) {
        if let Err(ref x) = *err {
            println!("{}", x);
        }
    }

    if items.iter().filter(|x| x.is_err()).count() > 0 {
        return Err(ConfigError {
            kind: ConfigErrorKind::ErrorItems,
            cause: Some(Box::new(items.iter().filter(|x| x.is_err()).next().unwrap().clone().err().unwrap()))
        });
    }

    Ok(Config {
        items: BinaryHeap::from(items.iter().cloned().map(|x| x.unwrap()).collect::<Vec<_>>())
    })
}

#[cfg(test)]
mod tests {
    use conf;

    #[test]
    fn load() {
        let data = "[[items]]
         key = \"os.uptime\"
         step = 60
         shell = \"cat /proc/uptime | cut -d\' \' -f1\"

         [[items]]
         key = \"os.loadavg\"
         step = 1
         shell = \"cat /proc/loadavg | cut -d\' \' -f1\"
";

        let config = conf::load(&mut data.as_bytes()).unwrap();
        assert_eq!(config.items.len(), 2);

    }
}
