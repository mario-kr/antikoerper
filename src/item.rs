
use std::path::PathBuf;
use std::time::Duration;

/// The different kinds of items one can supervise
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd)]
pub enum ItemKind {
    /// Read the file at the given location, useful on Linux for the /sys dir for example
    Read(PathBuf),
    /// Path to an executable with a list of arguments to be given to the executable
    Command(PathBuf, Vec<String>),
}

/// A single item, knowing when it is supposed to run next, what should be done and its key.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Item {
    next_time: Duration,
    step: Duration,
    key: String,
    kind: ItemKind,
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
            kind: ItemKind::Read(PathBuf::from("/dev/null")),
        });
        heap.push(Item {
            next_time: Duration::from_secs(3),
            step: Duration::from_secs(5),
            key: String::from("tests.two"),
            kind: ItemKind::Read(PathBuf::from("/dev/null")),
        });

        if let Some(item) = heap.pop() {
            assert_eq!(item.key, "tests.two");
        } else {
            unreachable!();
        }
    }
}
