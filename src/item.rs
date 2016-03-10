
use std::path::PathBuf;
use std::time::Duration;
use std::error::Error;

use toml;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ItemErrorKind {
    MissingValueSection,
    MissingStepSection,
    ValueArrayInvalid,
    ValueTableMissingKey,
    InvalidValueType,
    InvalidShellType,
    InvalidPathType,
    MultipleSources,
    MissingKey,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ItemError {
    key: String,
    kind: ItemErrorKind,
}

impl ItemError {
    fn as_str(&self) -> &str {
        match self.kind {
            ItemErrorKind::MissingValueSection  => "missing 'command', 'shell' or 'file' key",
            ItemErrorKind::MissingStepSection   => "missing 'step' key",
            ItemErrorKind::ValueArrayInvalid    => "specified an empty array as command",
            ItemErrorKind::ValueTableMissingKey => "specified a table with missing path and/or args",
            ItemErrorKind::InvalidValueType     => "invalid value type, you may only use tables, strings and arrays",
            ItemErrorKind::InvalidShellType     => "invalid value type, you may only use a string",
            ItemErrorKind::InvalidPathType      => "invalid value type, you may only use a string",
            ItemErrorKind::MultipleSources      => "multiple sources given, you may only use command or file or shell",
            ItemErrorKind::MissingKey           => "missing key field",
        }
    }
}

impl ItemError {
    fn new(key: String ,k: ItemErrorKind) -> ItemError {
        ItemError {
            key: key,
            kind: k,
        }
    }
    fn is_missing_value_section(&self) -> bool {
        match self.kind {
            ItemErrorKind::MissingValueSection => true,
            _ => false
        }
    }
}

impl Error for ItemError {
    fn description(&self) -> &str {
        self.as_str()
    }
}

impl ::std::fmt::Display for ItemError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        write!(f, "{}: {}", self.key, self.as_str())
    }
}

/// The different kinds of items one can supervise
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd)]
pub enum ItemKind {
    /// Read the file at the given location, useful on Linux for the /sys dir for example
    File(PathBuf),
    /// Path to an executable with a list of arguments to be given to the executable
    Command(PathBuf, Vec<String>),
    /// A string to be executed in a shell context
    Shell(String),
}

/// A single item, knowing when it is supposed to run next, what should be done and its key.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Item {
    next_time: Duration,
    step: Duration,
    key: String,
    kind: ItemKind,
}

impl Item {
    pub fn from_toml(table: &toml::Table) -> Result<Item, ItemError> {

        let key = match table.get("key") {
            Some(&toml::Value::String(ref s)) => s.clone(),
            _ => return Err(ItemError::new(String::from(""), ItemErrorKind::MissingKey))
        };

        let command = table.get("command")
            .ok_or(ItemError::new(key.clone(), ItemErrorKind::MissingValueSection))
            .and_then(|v| {
                let path : PathBuf;
                let args : Vec<String>;
                if let &toml::Value::Table(ref v) = v {
                    if let (Some(&toml::Value::String(ref s)), Some(&toml::Value::Array(ref a)))
                                                                 = (v.get("path"), v.get("args")) {
                        path = PathBuf::from(&s);
                        args = {
                            if a.iter().map(|x| x.as_str()).all(|x| x.is_some()) {
                                a.iter().map(|x| x.as_str()).map(|x| x.unwrap().into()).collect()
                            } else {
                                return Err(ItemError::new(key.clone(), ItemErrorKind::ValueArrayInvalid));
                            }
                        };

                        Ok(ItemKind::Command(path, args))
                    } else {
                        return Err(ItemError::new(key.clone(), ItemErrorKind::ValueTableMissingKey));
                    }
                } else if let &toml::Value::Array(ref a) = v {
                    if a.len() < 1 {
                        return Err(ItemError::new(key.clone(), ItemErrorKind::ValueArrayInvalid));
                    }
                    let mut iter = a.iter().map(|x| x.as_str());
                    if !iter.all(|x| x.is_some()) {
                        return Err(ItemError::new(key.clone(), ItemErrorKind::ValueArrayInvalid));
                    }
                    let mut strings = iter.map(|x| x.unwrap().into()).collect::<Vec<String>>();
                    path = PathBuf::from(strings.pop().unwrap());
                    args = strings;
                    Ok(ItemKind::Command(path, args))
                } else if let &toml::Value::String(ref s) = v {
                    path = PathBuf::from(s);
                    args = Vec::new();
                    Ok(ItemKind::Command(path, args))
                } else {
                    return Err(ItemError::new(key.clone(), ItemErrorKind::InvalidValueType));
                }
            });

        let shell = table.get("shell")
            .ok_or(ItemError::new(key.clone(), ItemErrorKind::MissingValueSection))
            .and_then(|v| {
                if let &toml::Value::String(ref s) = v {
                    Ok(ItemKind::Shell(s.clone()))
                } else {
                    Err(ItemError::new(key.clone(), ItemErrorKind::InvalidShellType))
                }
            });

        let path = table.get("path")
            .ok_or(ItemError::new(key.clone(), ItemErrorKind::MissingValueSection))
            .and_then(|v| {
                if let &toml::Value::String(ref s) = v {
                    Ok(ItemKind::File(PathBuf::from(s)))
                } else {
                    Err(ItemError::new(key.clone(), ItemErrorKind::InvalidPathType))
                }
            });

        let sources = vec![command, shell, path];

        {
            if sources.iter().all(|x| x.is_err()) {
                return Err(ItemError::new(key.clone(), ItemErrorKind::MissingValueSection));
            }

            if sources.iter().filter(|x| x.is_ok()).count() > 2 {
                return Err(ItemError::new(key.clone(), ItemErrorKind::MultipleSources));
            }
        }

        let kind = try!(sources.into_iter().filter(|x| x.is_ok()).next().unwrap());

        let time = match table.get("step") {
            Some(&toml::Value::Integer(x)) => x,
            _ => {
                return Err(ItemError {
                    key: key.clone(),
                    kind: ItemErrorKind::MissingStepSection,
                });
            }
        };


        Ok(Item {
            next_time: Duration::new(0, 0),
            step: Duration::from_secs(time as u64),
            key: key,
            kind: kind,
        })
    }
}

impl PartialOrd for Item {
    fn partial_cmp(&self, other: &Self) -> Option<::std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Item {
    fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
        if self.next_time < other.next_time {
            return ::std::cmp::Ordering::Greater
        } else {
            return ::std::cmp::Ordering::Less
        }
        return ::std::cmp::Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;
    use std::collections::BinaryHeap;

    use item::{Item,ItemKind};

    #[test]
    fn items_ordered_by_smallest_time_first() {
        let mut heap = BinaryHeap::new();
        heap.push(Item {
            next_time: Duration::from_secs(5),
            step: Duration::from_secs(5),
            key: String::from("tests.one"),
            kind: ItemKind::File(PathBuf::from("/dev/null")),
        });
        heap.push(Item {
            next_time: Duration::from_secs(3),
            step: Duration::from_secs(5),
            key: String::from("tests.two"),
            kind: ItemKind::File(PathBuf::from("/dev/null")),
        });

        if let Some(item) = heap.pop() {
            assert_eq!(item.key, "tests.two");
        } else {
            unreachable!();
        }
    }
}
