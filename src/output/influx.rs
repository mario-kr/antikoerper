use std::sync::Arc;
use std::time::Duration;

use futures::future::Future;
use tokio;

use influent::create_client;
use influent::client::{http::HttpClient, Client, Credentials, ClientError, Precision};
use influent::measurement::{Measurement, Value};

use item::Item;

use output::AKOutput;
use output::error::*;

#[derive(Clone, Deserialize)]
pub struct InfluxOutput {
    #[serde(default = "influx_database_default")]
    pub database: String,
    pub username: String,
    pub password: String,
    #[serde(default = "influx_hosts_default")]
    pub hosts: Vec<String>,
    #[serde(default = "influx_raw_as_fallback_default")]
    pub use_raw_as_fallback: bool,
    #[serde(default = "influx_always_raw_default")]
    pub always_write_raw: bool,
    #[serde(skip, default = "influx_dummy_client_deser")]
    pub client: Option<Arc<HttpClient>>,
}

fn influx_database_default() -> String {
    String::from("antikoerper")
}

fn influx_hosts_default() -> Vec<String> {
    vec![String::from("http://localhost:8086")]
}

fn influx_raw_as_fallback_default() -> bool {
    false
}

fn influx_always_raw_default() -> bool {
    false
}

fn influx_dummy_client_deser() -> Option<Arc<HttpClient>> {
    None
}

impl std::fmt::Debug for InfluxOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "InfluxOutput {{ database: {}, username: {}, password: {}, hosts: {:?}, use_raw_as_fallback: {}, always_write_raw: {} }}",
               self.database,
               self.username,
               self.password,
               self.hosts,
               self.use_raw_as_fallback,
               self.always_write_raw)
    }
}

impl PartialEq for InfluxOutput {
    fn eq(&self, other: &InfluxOutput) -> bool {
        self.database == other.database &&
            self.username == other.username &&
            self.password == other.password &&
            self.hosts == other.hosts &&
            self.use_raw_as_fallback == other.use_raw_as_fallback &&
            self.always_write_raw == other.always_write_raw
    }
}

impl Eq for InfluxOutput {}

impl From<ClientError> for OutputError {
    fn from(_e: ClientError) -> OutputError {
        OutputError {
            kind: OutputErrorKind::WriteError(String::from("InfluxOutput")),
            // influent::client::ClientError does not implement Error
            cause: None,
        }
    }
}

impl AKOutput for InfluxOutput {

    fn prepare(&self, _items: &Vec<Item>) -> Result<Self, OutputError> {
        trace!("running prepare for InfluxOutput");
        Ok(Self {
            database : self.database.clone(),
            username : self.username.clone(),
            password : self.password.clone(),
            hosts : self.hosts.clone(),
            use_raw_as_fallback : self.use_raw_as_fallback,
            always_write_raw : self.always_write_raw,
            client : Some(Arc::new(create_client(
                    Credentials {
                        username: self.username.clone(),
                        password: self.password.clone(),
                        database: self.database.clone(),
                    },
                    self.hosts.clone()
                    )))
        })
    }

    fn write_value(&self, key: &String, time: Duration, value: f64) -> Result<(), OutputError> {
        let mut m = Measurement::new(key);
        // Duration.as_nanos() is currently nightly only
        m.set_timestamp(time.as_secs() as i64 * 1000000000 + time.subsec_nanos() as i64);
        m.add_field("value", Value::Float(value));
        if let Some(ref client) = self.client {
            tokio::spawn(client
                       .write_one(m, Some(Precision::Nanoseconds))
                       .map_err(|e| println!("{:?}", e))
                      );
        } else {
            return Err(OutputError {
                kind: OutputErrorKind::WriteError("InfluxOutput.write_value: client is null".into()),
                cause: None
            });
        }
        Ok(())
    }

    fn write_raw_value_as_fallback(&self, key: &String, time: Duration, value: &str) -> Result<(), OutputError> {
        if self.use_raw_as_fallback {
            let mut m = Measurement::new(key);
            // Duration.as_nanos() is currently nightly only
            m.set_timestamp(time.as_secs() as i64 * 1000000000 + time.subsec_nanos() as i64);
            m.add_field("value", Value::String(value));
            if let Some(ref client) = self.client {
                tokio::spawn(client
                           .write_one(m, Some(Precision::Nanoseconds))
                           .map_err(|e| println!("{:?}", e))
                          );
            } else {
                return Err(OutputError {
                    kind: OutputErrorKind::WriteError("InfluxOutput.write_raw_value_as_fallback: client is null".into()),
                    cause: None
                });
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    fn write_raw_value(&self, key: &String, time: Duration, value: &str) -> Result<(), OutputError> {
        if self.always_write_raw {
            let mut m = Measurement::new(key);
            // Duration.as_nanos() is currently nightly only
            m.set_timestamp(time.as_secs() as i64 * 1000000000 + time.subsec_nanos() as i64);
            m.add_field("value", Value::String(value));
            if let Some(ref client) = self.client {
                tokio::spawn(client
                           .write_one(m, Some(Precision::Nanoseconds))
                           .map_err(|e| println!("{:?}", e))
                          );
            } else {
                return Err(OutputError {
                    kind: OutputErrorKind::WriteError("InfluxOutput.write_raw_value: client is null".into()),
                    cause: None
                });
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    fn clean_up(&self) -> Result<(), OutputError> {
        Ok(())
    }
}
