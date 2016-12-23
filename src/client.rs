extern crate ease;
extern crate simple_jwt;
extern crate hyper;


extern crate serde_json;

use std::collections::HashMap;
use self::ease::{Url, Request};
use std::sync::mpsc::Receiver;
use std::str::FromStr;
use self::hyper::header::{Authorization, Bearer};
use self::simple_jwt::{encode, Claim, Algorithm};

use SensorReading;
use std::error::Error;
use std::io;

#[derive(Debug)]
pub struct EdenClientConfig {
    pub server_address: String,
    secret: String,
    pub temperature_barometer_addr: String,
    pub sampling_rate: u64,
}

#[derive(Debug)]
pub enum EdenServerEndpoint {
    Temperature,
}

impl EdenClientConfig {
    pub fn new(secret: String, address: String, temperature_barometer_addr: String, sampling_rate: u64) -> EdenClientConfig {
        EdenClientConfig {
            server_address: address,
            secret: secret,
            temperature_barometer_addr: temperature_barometer_addr,
            sampling_rate: sampling_rate
        }
    }
}

pub trait SensorDataConsumer {
    fn attach(&self, data: Receiver<(EdenServerEndpoint, SensorReading)>);
}

pub struct Client {
    parsed_address: Url,
    jwt: String,
}

impl Client {
    pub fn new(config: EdenClientConfig) -> Result<Client, String> {
        let u = try!(Url::parse(&config.server_address).map_err(|e| e.description().to_owned()));

        let mut claim = Claim::default();
        claim.set_iss("pi");
        claim.set_payload_field("role", "sensor");
        let token = encode(&claim, &config.secret, Algorithm::HS256).unwrap();

        Ok(Client {
            parsed_address: u,
            jwt: token,
        })
    }

    pub fn send(&self,
                endpoint: EdenServerEndpoint,
                payload: Message)
                -> Result<(), io::Error> {
        let path = match endpoint {
            EdenServerEndpoint::Temperature => "temperature".to_string(),
        };
        let body = serde_json::to_string(&payload).unwrap();
        info!("Sending: {}", body);
        let url = self.parsed_address.clone().join(&path).unwrap();
        info!("{:#?}",
                 Request::new(url)
                     .header(Authorization(Bearer { token: self.jwt.clone() }))
                     .body(body)
                     .post().unwrap());
        return Ok(());
    }
}

impl SensorDataConsumer for Client {
    fn attach(&self, data: Receiver<(EdenServerEndpoint, SensorReading)>) {
        while let Ok(msg) = data.recv() {
            info!("Sending {:?}", msg);
            let reading = match msg.1 {
                SensorReading::TemperaturePressure{ t: t, p: p, ts: ts} => Message { temp: t, pressure: p, timestamp: ts }
            };
            self.send(msg.0, reading);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub temp: f32,
    pub pressure: f32,
    pub timestamp: i64
}
