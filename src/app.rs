
use std::thread;
use std::time::Duration;
use std::fs::{File, OpenOptions};
use std::process::Command;
use std::io::{Read, Write};
use std::collections::HashMap;
use std::f64;

use conf::Config;
use time::get_time;
use item::ItemKind;
use item::DigestKind;

pub fn start(mut conf: Config) {
    // We would deamonize here if necessary

    loop {
        loop {
            let cur_time = get_time().sec;
            conf.items.sort_unstable();
            if let Some(c) = conf.items.iter().peekable().peek() {
                if c.next_time > cur_time {
                    break;
                }
            } else {
                break;
            }


            let mut item = conf.items.pop().unwrap();
            let clone = item.clone();
            item.next_time = cur_time + item.interval;
            conf.items.push(item);

            let mut shell = String::new();

            let mut output_folder = conf.general.output.clone();

            if let ItemKind::Shell{script: _} = clone.kind {
                shell = conf.general.shell.clone();
            }

            thread::spawn(move || {
                let mut raw_result = String::new();
                match clone.kind {
                    ItemKind::File{ref path} => {
                        let mut f = match File::open(path) {
                            Ok(f) => f,
                            Err(e) => return error!("Could not open file: {}\n{}", path.display(), e),
                        };
                        match f.read_to_string(&mut raw_result) {
                            Ok(_) => (),
                            Err(e) => return error!("Could read output from file: {},\n{}", path.display(), e),
                        }
                    }
                    ItemKind::Command{ref path, ref args} => {
                        let mut output = Command::new(path);
                        output.args(args);
                        for (k,v) in clone.env.iter() {
                            output.env(k, v);
                        }
                        let output = match output.output() {
                            Ok(f) => f,
                            Err(e) => return error!("Could not run command: {}\n{}", path.display(), e)
                        };
                        raw_result = match String::from_utf8(output.stdout) {
                            Ok(r) => r,
                            Err(e) => return error!("Could not read output from command: {}\n{}", path.display(), e)
                        }
                    }
                    ItemKind::Shell{ref script} => {
                        let mut output = Command::new(&shell);
                        output.arg("-c");
                        output.arg(script);
                        for (k,v) in clone.env.iter() {
                            output.env(k, v);
                        }
                        let output = match output.output() {
                            Ok(f) => f,
                            Err(e) => return error!("Could not run shell script: {}\n{}", script, e)
                        };
                        raw_result = match String::from_utf8(output.stdout) {
                            Ok(r) => r,
                            Err(e) => return error!("Could not read output from shell script: {}\n{}", script, e)
                        }
                    }
                }

                debug!("{}={}", clone.key, raw_result);

                // digest the raw result if necessary
                let mut result : HashMap<String, f64> = HashMap::new();
                match clone.digest {
                    DigestKind::Raw => {
                    },
                    DigestKind::Regex { regex } => {
                        if let Some(captures) = regex.captures(raw_result.trim()) {
                            for cn in regex.capture_names() {
                                if let Some(named_group) = cn {
                                    let value = match captures[named_group].parse::<f64>() {
                                        Ok(f) => f,
                                        Err(_) => f64::NAN
                                    };
                                    result.insert(named_group.to_string(), value);
                                }
                            }
                        } else {
                            return error!("Provided regex did not match the output: {}\n{}", regex, raw_result);
                        }
                    },
                }

                // handle the raw_result extra
                output_folder.push(format!("{}.raw", &clone.key));
                match OpenOptions::new().write(true).append(true).create(true).open(&output_folder)
                    .and_then(|mut file| {
                        file.write(&format!("{} {}\n", cur_time, raw_result.trim()).as_bytes()[..])
                    })
                {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Error creating file {}, {}", output_folder.display(), e)
                    }
                }

                // iterate the results of digesting the raw_result, and put them in separate files
                for (k, v) in result {
                    output_folder.set_file_name(format!("{}.{}", &clone.key, k));
                    match OpenOptions::new().write(true).append(true).create(true).open(&output_folder)
                        .and_then(|mut file| {
                            file.write(&format!("{} {}\n", cur_time, v).as_bytes()[..])
                        })
                    {
                        Ok(_) => (),
                        Err(e) => {
                            error!("Error creating file {}, {}", output_folder.display(), e)
                        }
                    }
                }
            });
        }
            conf.items.sort_unstable();
            if let Some(c) = conf.items.iter().peekable().peek() {
                thread::sleep(Duration::from_secs((c.next_time - get_time().sec) as u64));
            }
        }
    }

