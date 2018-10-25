extern crate xdg;

use std::io::Read;
use std::path::PathBuf;

use itertools::Itertools;

use item::Item;
use item::ItemError;
use item::ItemErrorKind;

#[derive(Debug, Clone, Eq, PartialEq)]
enum ConfigErrorKind {
    IoError,
    TomlError,
    ErrorItems,
    DuplicateItem(String),
    Utf8Error,
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
            ConfigErrorKind::ErrorItems => write!(f, "some items have errors"),
            ConfigErrorKind::DuplicateItem(ref s) => write!(f, "duplicate key: {}", s),
            ConfigErrorKind::Utf8Error => write!(f, "utf8 error")
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

impl From<::toml::de::Error> for ConfigError {
    fn from(e: ::toml::de::Error) -> Self {
        ConfigError {
            kind: ConfigErrorKind::TomlError,
            cause: Some(Box::new(e)),
        }
    }
}

impl From<::std::string::FromUtf8Error> for ConfigError {
    fn from(e: ::std::string::FromUtf8Error) -> Self {
        ConfigError {
            kind: ConfigErrorKind::Utf8Error,
            cause: Some(Box::new(e)),
        }
    }
}

impl From<ItemError> for ConfigError {
    fn from(e: ItemError) -> Self {
        ConfigError {
            kind: ConfigErrorKind::ErrorItems,
            cause: Some(Box::new(e)),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub general: General,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct General {
    #[serde(default = "shell_default")]
    pub shell  : String,

    #[serde(default = "output_default")]
    pub output : PathBuf,
}

fn shell_default() -> String {
    String::from("/usr/bin/sh")
}

fn output_default() -> PathBuf {
    xdg::BaseDirectories::with_prefix("antikoerper")
        .unwrap()
        .create_data_directory(&PathBuf::new())
        .unwrap_or_else(|e| {
            println!("Error: {}", e);
            ::std::process::exit(1)
        })
}

pub fn load(r: &mut Read, o: PathBuf) -> Result<Config, ConfigError> {
    let content = {
        let mut buffer = String::new();
        r.read_to_string(&mut buffer)?;
        buffer
    };

    let mut data: Config = ::toml::de::from_str(&content)
        .map_err(ConfigError::from)?;

    debug!("{:#?}", data);

    data.general.output = xdg::BaseDirectories::with_prefix("antikoerper")
        .unwrap()
        .create_data_directory(if o == PathBuf::new() {
            data.general.output
        } else {
             // using the one provided with commandline argument
             o
        })
        .map_err(|e| {
            println!("Error while checking/creating path");
            println!("Error: {}", e);
            ConfigError { kind: ConfigErrorKind::IoError, cause: None }
        })?;

    trace!("Output path is: {:#?}", data.general.output);

    if let Some(err) = data.items
        .iter()
        .map(|x| x.key.clone())
        .sorted()
        .windows(2)
        .filter_map(|x| if x[0] == x[1] {
            Some(x[0].clone())
        } else {
            None
        })
        .next()
        .map(|n| Err(ConfigError { kind: ConfigErrorKind::DuplicateItem(n), cause: None }))
    {
        return err
    }

    if let Some(err) = data.items
        .iter()
        .filter_map(|i| if i.interval == 0 {
            Some(Err(ItemError::new(i.key.clone(), ItemErrorKind::InvalidInterval)))
        } else {
            None
        }).next()
    {
        return err.map_err(ConfigError::from)
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    extern crate xdg;

    use std::path::PathBuf;

    use conf;

    #[test]
    fn load() {
        let data = "[general]
         output = \"/tmp/test\"
         [[items]]
         key = \"os.uptime\"
         interval = 60
         command = \"cat /proc/uptime | cut -d\' \' -f1\"

         [[items]]
         key = \"os.loadavg\"
         interval = 1
         command = \"cat /proc/loadavg | cut -d\' \' -f1\"
";

        let config = conf::load(&mut data.as_bytes(), PathBuf::from("")).unwrap();
        assert_eq!(config.items.len(), 2);
    }

    #[test]
    fn no_duplicates() {
        let data = "[general]
         output = \"/tmp/test\"
         [[items]]
         key = \"os.uptime\"
         interval = 60
         command = \"cat /proc/uptime | cut -d\' \' -f1\"

         [[items]]
         key = \"os.uptime\"
         interval = 1
         command = \"cat /proc/loadavg | cut -d\' \' -f1\"
";

        let config = conf::load(&mut data.as_bytes(), PathBuf::from(""));
        assert!(config.is_err());
        match config {
            Err(conf::ConfigError{ kind: conf::ConfigErrorKind::DuplicateItem(n), ..}) => {
                assert_eq!(n, "os.uptime");
            },
            _ => {
                panic!("Wrong Error!: {:?}", config)
            }
        }
    }

    #[test]
    fn output_dir() {
        let data = "[general]
        output = \"/tmp/test\"
        [[items]]
        key = \"os.battery\"
        interval = 60
        command = \"acpi\"
        ";
        // Testcase 1: output-dir supplied by config file only
        let config = conf::load(&mut data.as_bytes(), PathBuf::new()).unwrap();
        assert_eq!(config.general.output, PathBuf::from("/tmp/test"));

        // Testcase 2: output-dir supplied by config, but also with commandline-argument
        // argument should override config
        let config = conf::load(&mut data.as_bytes(), PathBuf::from("/tmp/cmd_test")).unwrap();
        assert_eq!(config.general.output, PathBuf::from("/tmp/cmd_test"));

        // Testcase 3: Not output given, default should be used
        let data = "[general]
        [[items]]
        key = \"os.battery\"
        interval = 60
        command = \"acpi\"
        ";
        let config = conf::load(&mut data.as_bytes(), PathBuf::new()).unwrap();
        let xdg_default_dir = match xdg::BaseDirectories::with_prefix("antikoerper").unwrap()
            .create_data_directory(&PathBuf::new()) {
                Ok(s) => s,
                Err(e) => {
                    println!("Error: {}", e);
                    return;
                }
            };
        assert_eq!(config.general.output, xdg_default_dir,
                   "Expected {:?} to be {:?}", config.general.output, xdg_default_dir);
    }
}
