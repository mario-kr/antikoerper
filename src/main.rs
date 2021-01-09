#![deny(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces
)]

//! Antikoerper is a simple and lightweight data aggregation and visualization tool

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate clap;
extern crate toml;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate futures;
extern crate influxdb;
extern crate itertools;
extern crate regex;
extern crate serde_regex;
extern crate time;
extern crate tokio;
extern crate xdg;

use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::process;

use clap::{App, Arg};

mod app;
mod conf;
mod item;
mod output;

use output::AKOutput;

fn main() {
    let matches = App::new("Antikörper")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Neikos <neikos@neikos.email>")
        .about("Lightweight data aggregation and visualization tool.")
        .after_help("You can output logging information by using the RUST_LOG env var.")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("daemonize")
                .short("d")
                .long("daemonize")
                .multiple(false)
                .takes_value(false)
                .help("Starts antikoerper in daemon mode"),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .get_matches();

    trace!("Getting XDG Base directories");
    let xdg_dirs = xdg::BaseDirectories::with_prefix("antikoerper").unwrap();

    let level = match matches.occurrences_of("v") {
        0 => "error",
        1 => "warn",
        2 => "debug",
        3 | _ => "trace",
    };

    env_logger::Builder::from_env(
        env_logger::Env::default()
        .default_filter_or(level))
        .init();

    trace!("Matching for config value");
    let config_path = matches
        .value_of("config")
        .and_then(|s| Some(PathBuf::from(s)))
        .or_else(|| xdg_dirs.find_config_file("config.toml"));
    trace!("Value is: {:#?}", config_path);

    let config_path = match config_path {
        Some(x) => x,
        None => {
            println!("Could not find config file, make sure to give one with the --config option.");
            println!("The default is XDG_CONFIG_HOME/antikoerper/config.toml");
            println!("");
            println!(
                "Check out https://github.com/anti-koerper/antikoerper for details
on what should be in that file."
            );
            return;
        }
    };

    if matches.is_present("daemonize") {
        let mut child = process::Command::new(std::env::args().next().unwrap());
        let args = env::args()
            .skip(1)
            .filter(|a| a != "--daemonize" && a != "-d")
            .collect::<Vec<_>>();
        child
            .args(&args)
            .stdin(process::Stdio::null())
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null());
        match child.spawn() {
            Ok(_) => debug!("Successfully daemonized"),
            Err(e) => error!("Failed daemonizing the process {:#?}", e),
        }
        return;
    }

    info!("Config file used: {}", &config_path.display());

    let mut config_file = {
        let file = File::open(&config_path);
        match file {
            Ok(f) => f,
            Err(e) => {
                error!("{}", e);
                println!("Could not open file '{}': {}", config_path.display(), e);
                return;
            }
        }
    };

    let mut config = match conf::load(&mut config_file) {
        Ok(c) => c,
        Err(e) => {
            return error!(
                "Error at loading config file ({}): \n{}",
                config_path.display(),
                e
            )
        }
    };

    // run prepare() for every given output
    config.output = config
        .output
        .iter()
        .map(|op| {
            op.prepare(&config.items)
                .map_err(|e| {
                    error!("Error while preparing an output: {}", e);
                    error!("Abort start-up");
                    ::std::process::exit(1);
                })
                .unwrap()
        })
        .collect();

    app::start(config.clone());

    // run clean_up() for every given output
    config.output.iter().for_each(|op| {
        op.clean_up()
            .map_err(|e| error!("Error while cleaning up an output: {}", e))
            .unwrap_or(())
    });
}
