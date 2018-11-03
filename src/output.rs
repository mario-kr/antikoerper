extern crate xdg;

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::string::String;
use std::time::Duration;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OutputErrorKind {
    PrepareError(String),
    WriteError(String),
    CleanupError(String),
}

#[derive(Debug)]
pub struct OutputError {
    kind: OutputErrorKind,
    cause: Option<Box<::std::error::Error>>,
}

impl ::std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match self.kind {
            OutputErrorKind::PrepareError(ref s) => write!(f, "failed to prepare output {}", s),
            OutputErrorKind::WriteError(ref s) => write!(f, "failed writing values to output {}", s),
            OutputErrorKind::CleanupError(ref s) => write!(f, "cleanup of output {} returned an error", s)
        }
    }
}

pub trait AKOutput {
    fn prepare(&mut self) -> Result<(), OutputError>;
    fn write_value(&mut self, key: &String, time: Duration, value: f64) -> Result<(), OutputError>;
    fn write_raw_value(&mut self, key: &String, time: Duration, value: &String) -> Result<(), OutputError>;
    fn write_raw_value_as_fallback(&mut self, key: &String, time: Duration, value: &String) -> Result<(), OutputError>;
    fn clean_up(&mut self) -> Result<(), OutputError>;
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
pub struct FileOutput {
    #[serde(default = "file_base_path_default")]
    pub base_path: PathBuf,
    #[serde(default = "file_always_raw_default")]
    pub always_write_raw: bool,
}

fn file_base_path_default() -> PathBuf {
    xdg::BaseDirectories::with_prefix("antikoerper")
        .unwrap()
        .create_data_directory(&PathBuf::new())
        .unwrap_or_else(|e| {
            println!("Error: {}", e);
            ::std::process::exit(1)
        })
}

fn file_always_raw_default() -> bool {
    false
}

impl AKOutput for FileOutput {

    fn prepare(&mut self) -> Result<(), OutputError> {
        // TODO: crate base_path if necessary
        // TODO: check if base_path is writable
        Ok(())
    }

    fn write_value(&mut self, key: &String, time: Duration, value: f64) -> Result<(), OutputError> {
        self.base_path.push(key);
        match OpenOptions::new().write(true).append(true).create(true).open(&self.base_path)
            .and_then(|mut file| {
                file.write(&format!("{} {}\n", time.as_secs(), value).as_bytes()[..])
            })
        {
            Ok(_) => {
                self.base_path.pop();
                Ok(())
            },
            Err(e) => {
                self.base_path.pop();
                Err(OutputError {
                    kind: OutputErrorKind::WriteError(String::from("FileOutput")),
                    cause: Some(Box::new(e))
                })
            }
        }
    }

    fn write_raw_value_as_fallback(&mut self, key: &String, time: Duration, value: &String) -> Result<(), OutputError> {
        self.base_path.push(key);
        match OpenOptions::new().write(true).append(true).create(true).open(&self.base_path)
            .and_then(|mut file| {
                file.write(&format!("{} {}\n", time.as_secs(), value.trim()).as_bytes()[..])
            })
        {
            Ok(_) => {
                self.base_path.pop();
                Ok(())
            },
            Err(e) => {
                self.base_path.pop();
                Err(OutputError {
                    kind: OutputErrorKind::WriteError(String::from("FileOutput")),
                    cause: Some(Box::new(e))
                })
            }
        }
    }

    fn write_raw_value(&mut self, key: &String, time: Duration, value: &String) -> Result<(), OutputError> {
        if self.always_write_raw {
            self.write_raw_value_as_fallback(key, time, value)
        } else {
            Ok(())
        }
    }

    fn clean_up(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OutputKind {
    File{
        #[serde(flatten)]
        fo : FileOutput
    },
    // more in the future
}

impl AKOutput for OutputKind {

    fn prepare(&mut self) -> Result<(), OutputError> {
        match self {
            OutputKind::File{ fo } => fo.prepare(),
        }
    }

    fn write_value(&mut self, key: &String, time: Duration, value: f64) -> Result<(), OutputError> {
        match self {
            OutputKind::File{ fo } => fo.write_value(key, time, value),
        }
    }

    fn write_raw_value_as_fallback(&mut self, key: &String, time: Duration, value: &String) -> Result<(), OutputError> {
        match self {
            OutputKind::File{ fo } => fo.write_raw_value_as_fallback(key, time, value),
        }
    }

    fn write_raw_value(&mut self, key: &String, time: Duration, value: &String) -> Result<(), OutputError> {
        match self {
            OutputKind::File{ fo } => fo.write_raw_value(key, time, value),
        }
    }

    fn clean_up(&mut self) -> Result<(), OutputError> {
        match self {
            OutputKind::File{ fo } => fo.clean_up(),
        }
    }
}
