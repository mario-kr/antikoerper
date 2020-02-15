
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::time::{SystemTime, Instant, Duration};
use std::fs::File;
use std::path::PathBuf;
use std::process::Command;
use std::io::Read;
use std::f64;

use futures::Future;
use futures::Stream;
use tokio::runtime::Runtime;
use tokio::timer::Interval;

use crate::conf::Config;
use crate::item::{ItemKind, DigestKind};
use crate::output::AKOutput;


/// Create and starts the tokio runtime, adding one forever repeating task for
/// every configured item.
pub fn start(conf: Config) {

    // panics when failing to construct Runtime
    let mut runtime = Runtime::new().unwrap();

    for item in conf.items {
        // copy shell and outputs, as those are needed in every thread
        // item is only one per thread, and can be moved
        let shell = conf.general.shell.clone();
        let outputs = conf.output.clone();
        let work_item = Interval::new(Instant::now(), Duration::from_secs(item.interval as u64))
            .for_each(move |_| {

                /// runs a command with the specified args and env, and returns
                /// its stdout as a String
                fn run_cmd_capture_output(cmd: &PathBuf, args: &Vec<String>, env: &BTreeMap<String, String>)
                    -> Result<String, tokio::timer::Error>
                    {
                        let mut command = Command::new(cmd);
                        command.args(args);
                        for (k, v) in env.iter() {
                            command.env(k, v);
                        }
                        let output = command.output()
                            .map_err(|e| {
                                error!("Could not run command: {}\n{}", cmd.display(), e);
                                tokio::timer::Error::shutdown()
                            })?;
                        Ok(String::from_utf8(output.stdout)
                           .map_err(|e| {
                               error!("Could not read output from command: {}\n{}", cmd.display(), e);
                               tokio::timer::Error::shutdown()
                           })?)
                    }

                //
                // generate/query for data as configured
                //
                let mut raw_result = String::new();
                match item.kind {
                    ItemKind::File{ref path} => {
                        let mut f = File::open(path)
                            .map_err(|e| {
                                error!("Could not open file: {}\n{}", path.display(), e);
                                tokio::timer::Error::shutdown()
                            })?;
                        f.read_to_string(&mut raw_result)
                            .map_err(|e| {
                                error!("Could not read output from file: {},\n{}", path.display(), e);
                                tokio::timer::Error::shutdown()
                            })?;
                    },
                    ItemKind::Command{ref path, ref args} => {
                        raw_result = run_cmd_capture_output(path, args, &item.env)?;
                    },
                    ItemKind::Shell{ref script} => {
                        raw_result = run_cmd_capture_output(
                            &PathBuf::from(&shell),
                            &vec![String::from("-c"), script.clone()],
                            &item.env
                            )?;
                    }
                }
                let raw_result = raw_result.trim();
                debug!("{}={}", item.key, raw_result);

                //
                // digest raw data as configured
                //
                let mut values : HashMap<String, f64> = HashMap::new();

                match item.digest {

                    // no digest - use the fallback method
                    // errors when trying to parse a float from the given raw value are logged as info
                    // having not-parsable output is a valid use-case for antikoerper
                    DigestKind::Raw => {
                        let _ = raw_result.parse::<f64>()
                            .map(|v| values.insert(format!("{}.parsed", &item.key).into(), v))
                            .map_err(|_| info!("Value could not be parsed as f64: {}", raw_result));
                    },

                    // digest using regexes, and write the extracted values
                    DigestKind::Regex { ref regex } => {

                        if let Some(captures) = regex.captures(raw_result) {
                            for cn in regex.capture_names() {
                                if let Some(named_group) = cn {
                                    let value = captures[named_group]
                                        .parse::<f64>()
                                        .unwrap_or(f64::NAN);
                                    values.insert(format!("{}.{}", &item.key, &named_group).into(), value);
                                }
                            }
                        } else {
                            warn!("Provided regex did not match the output: {}\n{}", regex, raw_result);
                        }
                    }
                }

                //
                // write data to all configured outputs
                //
                let cur_time = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                    Ok(n) => n,
                    Err(_) => panic!("SystemTime before UNIX EPOCH!"),
                };

                for outp in &outputs {
                    if values.len() == 0 {
                        let _ = outp.write_raw_value_as_fallback(&format!("{}.raw", &item.key), cur_time, raw_result)
                            .map_err(|e| error!("Failure writing to output: {}", e));

                    } else {
                        let _ = outp.write_raw_value(&format!("{}.raw", &item.key), cur_time, raw_result)
                            .map_err(|e| error!("Failure writing to output: {}", e));

                        for (k, v) in values.iter() {
                            let _ = outp.write_value(&k, cur_time, *v)
                                .map_err(|e| error!("Failure writing to output: {}", e));
                        }
                    }
                }
                Ok(())
            })
        .map_err(|_| ());
        runtime.spawn(work_item);
    }

    runtime.shutdown_on_idle().wait().unwrap();
}

