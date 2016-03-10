#![deny(missing_docs,
        missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code,
        unstable_features,
        unused_import_braces, unused_qualifications)]

//! Antikoerper is a simple and lightweight data aggregation and visualization tool

extern crate rustc_serialize;
extern crate toml;
extern crate clap;
#[macro_use] extern crate log;
extern crate env_logger;

use clap::{Arg, App, SubCommand};

mod conf;
mod item;

fn main() {
    let matches = App::new("Antikörper")
                    .version(env!("CARGO_PKG_VERSION"))
                    .author("Neikos <neikos@neikos.email>")
                    .about("Lightweight data aggregation and visualization tool.")
                    .after_help("You can output logging information by using the RUST_LOG env var.")
                    .arg(Arg::with_name("config")
                         .short("c")
                         .long("config")
                         .value_name("FILE")
                         .help("Sets a custom config file")
                         .takes_value(true))
                    .arg(Arg::with_name("v")
                         .short("v")
                         .multiple(true)
                         .help("Sets the level of verbosity"))
                    .get_matches();

    let config = matches.value_of("config").unwrap_or("~/.config/");

    let level = match matches.occurrences_of("v") {
        0 => log::LogLevelFilter::Off,
        1 => log::LogLevelFilter::Warn,
        2 => log::LogLevelFilter::Debug,
        3 | _ => log::LogLevelFilter::Trace,
    };

    env_logger::LogBuilder::new().filter(None, level).init().unwrap();

    info!("Config file used: {}", config);

}
