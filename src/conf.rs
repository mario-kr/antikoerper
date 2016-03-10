
use std::collections::BinaryHeap;
use std::io::Read;

use toml;
use item::Item;

/// The Configuration of Antikoerper
pub struct Config {
    items: BinaryHeap<Item>,
}


pub fn load(r: &mut Read) -> Config {
    Config {
        items: BinaryHeap::new()
    }
}
