
use std::collections::BinaryHeap;

use toml;
use item::Item;

/// The Configuration of Antikoerper
pub struct Config {
    items: BinaryHeap<Item>,
}

