extern crate ease;

extern crate serde_json;

use std::collections::HashMap;
use self::ease::{Url, Request};
use std::sync::mpsc::Receiver;
use std::str::FromStr;

use SensorReading;
use std::error::Error;
use std::io;


pub struct EdenClientConfig {
    server_address: String,
    secret: String,
}

pub enum EdenServerEndpoint {
    Temperature,
}

impl EdenClientConfig {
    pub fn new(secret: String, address: String) -> EdenClientConfig {
        EdenClientConfig {
            server_address: address,
            secret: secret,
        }
    }
}

#[derive(Deserialize, Debug)]
struct PostResponse {
    args: HashMap<String, String>,
    data: Option<String>,
    files: Option<HashMap<String, String>>,
    form: Option<HashMap<String, String>>,
    headers: HashMap<String, String>,
    json: Option<String>,
    origin: String,
    url: String,
}

pub trait SensorDataConsumer {
    fn attach(&self, data: Receiver<SensorReading>);
}

pub struct Client {
    config: EdenClientConfig,
    parsed_address: Url,
}

impl Client {
    pub fn new(config: EdenClientConfig) -> Result<Client, String> {
        let u = try!(Url::parse(&config.server_address).map_err(|e| e.description().to_owned()));
        Ok(Client {
            config: config,
            parsed_address: u,
        })
    }

    pub fn send(&self,
                endpoint: EdenServerEndpoint,
                payload: SensorReading)
                -> Result<(), io::Error> {
        let path = match endpoint {
            EdenServerEndpoint::Temperature => "temperature".to_string(),
        };
        println!("{:#?}", Request::new(self.parsed_address.clone()).post());
        return Ok(());
        // .and_then(|res| res.from_json::<PostResponse>()));
    }
}

impl SensorDataConsumer for Client {
    fn attach(&self, data: Receiver<SensorReading>) {
        while let Ok(msg) = data.recv() {
            info!("Sending {:?}", msg);
            self.send(EdenServerEndpoint::Temperature, msg);
        }
    }
}
