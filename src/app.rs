
use std::thread;
use std::time::Duration;
use std::fs::File;
use std::process::Command;
use std::io::Read;

use conf::Config;
use time::get_time;
use item::ItemKind;

pub fn start(mut conf: Config) {
    // We would deamonize here if necessary

    loop {
        loop {
            if let Some(c) = conf.items.peek() {
                if c.next_time > get_time().sec {
                    break;
                }
            } else {
                break;
            }


            let mut item = conf.items.pop().unwrap();
            let clone = item.clone();
            item.next_time = get_time().sec + item.interval;
            conf.items.push(item);

            thread::spawn(move || {
                let mut result = String::new();
                match clone.kind {
                    ItemKind::File(ref path) => {
                        let mut f = File::open(path).unwrap();
                        f.read_to_string(&mut result).unwrap();
                    }
                    ItemKind::Command(ref path, ref args) => {
                        let output = Command::new(path).args(args).output().unwrap();
                        result = String::from_utf8(output.stdout).unwrap();
                    }
                    ItemKind::Shell(ref command) => {
                        let output = Command::new("/usr/bin/sh").arg("-c").arg(command)
                            .output().unwrap();
                        result = String::from_utf8(output.stdout).unwrap();
                    }
                }
                println!("{}={}", clone.key, result);
            });
        }
        if let Some(c) = conf.items.peek() {
            thread::sleep(Duration::from_secs((c.next_time - get_time().sec) as u64));
        }
    }
}

