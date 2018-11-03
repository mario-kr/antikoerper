
use std::thread;
use std::time::{SystemTime, Duration};
use std::fs::File;
use std::process::Command;
use std::io::Read;
use std::f64;

use regex::Regex;

use conf::Config;
use time::get_time;
use item::ItemKind;
use item::DigestKind;
use output::AKOutput;

pub fn start(mut conf: Config) {
    // We would deamonize here if necessary

    loop {
        loop {
            let cur_time = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                Ok(n) => n,
                Err(_) => panic!("SystemTime before UNIX EPOCH!"),
            };
            conf.items.sort_unstable();
            if let Some(c) = conf.items.iter().peekable().peek() {
                if c.next_time > cur_time.as_secs() as i64 {
                    break;
                }
            } else {
                break;
            }


            let mut item = conf.items.remove(0);
            let clone = item.clone();
            item.next_time = cur_time.as_secs() as i64 + item.interval;
            conf.items.push(item);

            let outputs = conf.output.clone();

            let mut shell = String::new();

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

                // digest?
                match clone.digest {

                    // no digest - use the fallback method
                    DigestKind::Raw => {
                        for mut outp in outputs {
                            match outp.write_raw_value_as_fallback(
                                &format!("{}.raw", &clone.key),
                                cur_time,
                                &raw_result)
                            {
                                Ok(_) => (),
                                Err(e) => error!("Failure writing to output: {}", e)
                            };
                        }
                    },

                    // digest using regexes, and write the extracted values
                    DigestKind::Regex { regex } => {
                        // if an output is configured that way, the raw_result will always be written
                        for mut outp in outputs.clone() {
                            match outp.write_raw_value(
                                &format!("{}.raw", &clone.key),
                                cur_time,
                                &raw_result)
                            {
                                Ok(_) => (),
                                Err(e) => error!("Failure writing to output: {}", e)
                            };
                        }

                        if let Some(captures) = regex.captures(raw_result.trim()) {
                            for cn in regex.capture_names() {
                                if let Some(named_group) = cn {
                                    let value = match captures[named_group].parse::<f64>() {
                                        Ok(f) => f,
                                        Err(_) => f64::NAN
                                    };
                                    for mut outp in outputs.clone() {
                                        match outp.write_value(
                                            &format!("{}.{}", &clone.key, &named_group),
                                            cur_time,
                                            value)
                                        {
                                            Ok(_) => (),
                                            Err(e) => error!("Failure writing to output: {}", e)
                                        };
                                    }
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

                        Ok(Default::default())
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

