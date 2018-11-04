extern crate xdg;

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::string::String;
use std::time::Duration;

use item::Item;

use output::AKOutput;
use output::error::*;

#[derive(Debug, Clone, Eq, PartialEq, Default, Deserialize)]
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

impl Default for FileOutput {
    fn default() -> FileOutput {
        FileOutput {
            base_path: file_base_path_default(),
            always_write_raw: file_always_raw_default()
        }
    }
}

impl AKOutput for FileOutput {

    fn prepare(&mut self, _items: &Vec<Item>) -> Result<(), OutputError> {
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
