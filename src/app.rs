
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

use conf::Config;
use item::Item;
use item::ItemKind;
use item::DigestKind;
use output::OutputKind;
use output::AKOutput;

/// Does whatever is specified in the configured item to get information/output
/// returns Result::Err if this fails in any way
fn produce_value(i : &Item, shell: &String) -> Result<String, ()> {

    fn run_cmd_capture_output(cmd: &PathBuf, args: &Vec<String>, env: &BTreeMap<String, String>)
        -> Result<String, ()>
    {
        let mut command = Command::new(cmd);
        command.args(args);
        for (k, v) in env.iter() {
            command.env(k, v);
        }
        let output = command.output()
            .map_err(|e| {
                error!("Could not run command: {}\n{}", cmd.display(), e);
            })?;
        Ok(String::from_utf8(output.stdout)
            .map_err(|e| {
                error!("Could not read output from command: {}\n{}", cmd.display(), e);
            })?)
    }

    let mut raw_result = String::new();
    match i.kind {
        ItemKind::File{ref path} => {
            let mut f = File::open(path)
                .map_err(|e| {
                    error!("Could not open file: {}\n{}", path.display(), e);
                })?;
            f.read_to_string(&mut raw_result)
                .map_err(|e| {
                    error!("Could not read output from file: {},\n{}", path.display(), e);
                })?;
        },
        ItemKind::Command{ref path, ref args} => {
            raw_result = run_cmd_capture_output(path, args, &i.env)?;
        },
        ItemKind::Shell{ref script} => {
            raw_result = run_cmd_capture_output(
                &PathBuf::from(shell),
                &vec![String::from("-c"), script.clone()],
                &i.env
            )?;
        }
    }
    debug!("{}={}", i.key, raw_result);

    Ok(raw_result)
}

/// Digest a previously acquired raw value as specified in the item.
/// This cannot fail; in the worst case, the HashMap remains empty, and
/// only the raw, unparsed value may later be written to the outputs.
fn digest_value(i: &Item, raw: String) -> (String, HashMap<String, f64>) {

    let mut values : HashMap<String, f64> = HashMap::new();
    let raw = raw.trim();

    match i.digest {

        // no digest - use the fallback method
        // errors when trying to parse a float from the given raw value are logged as debug
        // having not-parsable output is a valid use-case for antikoerper
        DigestKind::Raw => {
            let _ = raw.parse::<f64>()
                .map(|v| values.insert(format!("{}.parsed", &i.key).into(), v))
                .map_err(|_| debug!("Value could not be parsed as f64: {}", raw));
        },

        // digest using regexes, and write the extracted values
        DigestKind::Regex { ref regex } => {

            if let Some(captures) = regex.captures(raw) {
                for cn in regex.capture_names() {
                    if let Some(named_group) = cn {
                        let value = captures[named_group]
                            .parse::<f64>()
                            .unwrap_or(f64::NAN);
                        values.insert(format!("{}.{}", &i.key, &named_group).into(), value);
                    }
                }
            } else {
                error!("Provided regex did not match the output: {}\n{}", regex, raw);
            }
        }
    }
    (raw.to_string(), values)
}

/// Outputs the given values (raw as well as parsed ones) to the configured
/// outputs.
/// Every failure is logged, but the function is never aborted with an early
/// return, as there might be several outputs, of which one might work.
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

/// Create and starts the tokio runtime, adding one forever repeating task for
/// every configured item.
pub fn start(conf: Config) {

    // panics when failing to construct Runtime
    let mut runtime = Runtime::new().unwrap();

    for item in conf.items.clone() {
        let shell = conf.general.shell.clone();
        let outputs = conf.output.clone();
        let i_clone = item.clone();
        let work_item = Interval::new(Instant::now(), Duration::from_secs(i_clone.interval as u64))
            .for_each(move |_| {
                produce_value(&i_clone, &shell)
                    .and_then(|raw_value| Ok(digest_value(&i_clone, raw_value)))
                    .and_then(|(raw, v)| output_value(&i_clone, &outputs, raw, v))
                    .map_err(|_| tokio::timer::Error::shutdown())
            })
            .map_err(|_| ());
        runtime.spawn(work_item);
    }

    runtime.shutdown_on_idle().wait().unwrap();
}

