extern crate hyper;
extern crate rustc_serialize;
extern crate toml;

use std::fs::File;

mod conf;

fn main() {
    // First step, read config file
    let conf = conf::Configuration::from_file(&mut {
        match File::open("antikoerper.conf") {
            Ok(f) => f,
            Err(e) => {
                println!("Could not open config file: {}", e);
                std::process::exit(1);
            }
        }
    });
    let conf = match conf {
        Err(e) => {
            println!("Could not create config: {}", e);
            std::process::exit(1);
        }
        Ok(c) => c
    };

    println!("Starting Antikörper....");
    println!("{:#?}", conf);

}
