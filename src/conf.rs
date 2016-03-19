
use std::collections::BinaryHeap;
use std::io::Read;
use std::path::PathBuf;

use toml;
use item::Item;

/// The Configuration of Antikoerper
#[derive(Debug, Clone)]
pub struct Config {
    pub items: BinaryHeap<Item>,
    pub general: General,
    pub output: PathBuf,
}

#[derive(Debug, Clone)]
pub struct General {
    pub shell: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum ConfigErrorKind {
    IoError,
    TomlError,
    MissingItems,
    ErrorItems,
    DuplicateItem(String),
    MismatchedShellType,
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
            ConfigErrorKind::DuplicateItem(ref s) => write!(f, "duplicate key: {}", s),
            ConfigErrorKind::MismatchedShellType => write!(f, "general.shell has to be a string")
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

pub fn load(r: &mut Read, o: PathBuf) -> Result<Config, ConfigError> {
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

    debug!("{:#?}", parsed);

    let general = match parsed.get("general") {
        Some(&toml::Value::Table(ref v)) => {
            General {
                shell: match v.get("shell") {
                    Some(&toml::Value::String(ref s)) => s.clone(),
                    Some(_) => return Err(ConfigError {
                        kind: ConfigErrorKind::MismatchedShellType,
                        cause: None,
                    }),
                    _ => String::from("/usr/bin/shell"),
                },
            }
        }
        _ => {
            General {
                shell: String::from("/usr/bin/shell"),
            }
        }
    };

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

    let mut it = items.iter().map(|x| x.as_ref().unwrap().key.clone()).collect::<Vec<_>>();
    it.sort();
    let mut it = it.windows(2).map(|x| if x[0] == x[1] { Some(x[0].clone()) } else { None })
        .filter_map(|x| x);

    if let Some(n) = it.next() {
        return Err(ConfigError {
            kind: ConfigErrorKind::DuplicateItem(n),
            cause: None
        })
    }


    Ok(Config {
        items: BinaryHeap::from(items.iter().cloned().map(|x| x.unwrap()).collect::<Vec<_>>()),
        general: general,
        output: o,
    })
}

#[cfg(test)]
mod tests {
    use conf;

    #[test]
    fn load() {
        let data = "[[items]]
         key = \"os.uptime\"
         interval = 60
         shell = \"cat /proc/uptime | cut -d\' \' -f1\"

         [[items]]
         key = \"os.loadavg\"
         interval = 1
         shell = \"cat /proc/loadavg | cut -d\' \' -f1\"
";

        let config = conf::load(&mut data.as_bytes(), PathBuf::from("")).unwrap();
        assert_eq!(config.items.len(), 2);
    }

    #[test]
    fn no_duplicates() {
        let data = "[[items]]
         key = \"os.uptime\"
         interval = 60
         shell = \"cat /proc/uptime | cut -d\' \' -f1\"

         [[items]]
         key = \"os.uptime\"
         interval = 1
         shell = \"cat /proc/loadavg | cut -d\' \' -f1\"
";

        let config = conf::load(&mut data.as_bytes(), PathBuf::from(""));
        assert!(config.is_err());
        match config {
            Err(conf::ConfigError{ kind: conf::ConfigErrorKind::DuplicateItem(n), ..}) => {
                assert_eq!(n, "os.uptime");
            },
            _ => {
                panic!("Wrong Error!")
            }
        }
    }
}
