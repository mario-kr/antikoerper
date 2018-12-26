
use std::collections::HashMap;
use std::time::{SystemTime, Instant, Duration};
use std::fs::File;
use std::process::Command;
use std::io::Read;
use std::f64;

use regex::Regex;
use futures::Future;
use futures::Stream;
use tokio::runtime::Runtime;
use tokio::timer::Interval;

use conf::Config;
use item::Item;
use item::ItemKind;
use item::DigestKind;
use output::OutputKind;
use output::AKOutput;

fn produce_value(i : &Item, shell: &String) -> Result<String, ()> {

    let mut raw_result = String::new();
    match i.kind {
        ItemKind::File{ref path} => {
            let mut f = match File::open(path) {
                Ok(f) => f,
                Err(e) => {
                    error!("Could not open file: {}\n{}", path.display(), e);
                    return Err(());
                }
            };
            match f.read_to_string(&mut raw_result) {
                Ok(_) => (),
                Err(e) => {
                    error!("Could read output from file: {},\n{}", path.display(), e);
                    return Err(());
                }
            }
        },
        ItemKind::Command{ref path, ref args} => {
            let mut output = Command::new(path);
            output.args(args);
            for (k,v) in i.env.iter() {
                output.env(k, v);
            }
            let output = match output.output() {
                Ok(f) => f,
                Err(e) => {
                    error!("Could not run command: {}\n{}", path.display(), e);
                    return Err(());
                }
            };
            raw_result = match String::from_utf8(output.stdout) {
                Ok(r) => r,
                Err(e) => {
                    error!("Could not read output from command: {}\n{}", path.display(), e);
                    return Err(());
                }
            }
        },
        ItemKind::Shell{ref script} => {
            let mut output = Command::new(&shell);
            output.arg("-c");
            output.arg(script);
            for (k,v) in i.env.iter() {
                output.env(k, v);
            }
            let output = match output.output() {
                Ok(f) => f,
                Err(e) => {
                    error!("Could not run shell script: {}\n{}", script, e);
                    return Err(());
                }
            };
            raw_result = match String::from_utf8(output.stdout) {
                Ok(r) => r,
                Err(e) => {
                    error!("Could not read output from shell script: {}\n{}", script, e);
                    return Err(());
                }
            }
        }
    }
    debug!("{}={}", i.key, raw_result);

    Ok(raw_result)
}

fn digest_value(i: &Item, raw: String) -> Result<(String, HashMap<String, f64>), ()> {

    let mut values : HashMap<String, f64> = HashMap::new();

    match i.digest {

        // no digest - use the fallback method
        DigestKind::Raw => {
            match raw.trim().parse::<f64>() {
                Ok(v) => values.insert(format!("{}.parsed", &i.key).into(), v),
                Err(_) => None
            };
        },

        // digest using regexes, and write the extracted values
        DigestKind::Regex { ref regex } => {

            if let Some(captures) = regex.captures(raw.trim()) {
                for cn in regex.capture_names() {
                    if let Some(named_group) = cn {
                        let value = match captures[named_group].parse::<f64>() {
                            Ok(f) => f,
                            Err(_) => f64::NAN
                        };
                        values.insert(format!("{}.{}", &i.key, &named_group).into(), value);
                    }
                }

                debug!("{}={}", clone.key, raw_result);

                // digest?
                match clone.digest {

                    // no digest - use the fallback method
                    DigestKind::Raw => {
                        match raw_result.trim().parse::<f64>() {
                            Ok(v) => {
                                for mut outp in outputs {
                                    match outp.write_value(
                                        &format!("{}.parsed", &clone.key),
                                        cur_time,
                                        v)
                                    {
                                        Ok(_) => (),
                                        Err(e) => error!("Failure writing to output: {}", e)
                                    };
                                }
                            },
                            Err(_) => {
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
    }
    return Ok((raw, values));
}

fn output_value(i: &Item, o: &Vec<OutputKind>, raw: String, values: HashMap<String, f64>)
    -> Result<(), ()>
{
    let cur_time = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n,
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    };

    for mut outp in o.clone() {

        if values.len() == 0 {
            let _ = outp.write_raw_value_as_fallback(&format!("{}.raw", &i.key), cur_time, &raw)
                .map_err(|e| error!("Failure writing to output: {}", e));

        } else {

            let _ = outp.write_raw_value(&format!("{}.raw", &i.key), cur_time, &raw)
                .map_err(|e| error!("Failure writing to output: {}", e));

            for (k, v) in values.iter() {
                let _ = outp.write_value(&k, cur_time, *v)
                    .map_err(|e| error!("Failure writing to output: {}", e));
            }
        }
    }
    Ok(())
}


pub fn start(conf: Config) {
    // We would deamonize here if necessary

    // panics when failing to construct Runtime
    let mut runtime = Runtime::new().unwrap();

    for item in conf.items.clone() {
        let shell = conf.general.shell.clone();
        let outputs = conf.output.clone();
        let i_clone = item.clone();
        let work_item = Interval::new(Instant::now(), Duration::from_secs(i_clone.interval as u64))
            .for_each(move |_| {
                produce_value(&i_clone, &shell)
                    .and_then(|raw_value| digest_value(&i_clone, raw_value))
                    .and_then(|(raw, extracted)| output_value(&i_clone, &outputs, raw, extracted))
                    .map_err(|_| tokio::timer::Error::shutdown())
            })
            .map_err(|_| ());
        runtime.spawn(work_item);
    }

    runtime.shutdown_on_idle().wait().unwrap();
}

