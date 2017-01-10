// Copyright 2016 Claus Matzinger
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.


extern crate ease;
extern crate simple_jwt;
extern crate hyper;
extern crate serde_json;
extern crate threadpool;
use std::time::Duration;


use self::ease::{Url, Request};
use std::sync::mpsc::Receiver;
use self::hyper::header::{Authorization, Bearer, ContentType, ContentLength};
use self::simple_jwt::{encode, Claim, Algorithm};
use self::hyper::mime::{Mime, TopLevel, SubLevel, Attr, Value};
use self::threadpool::ThreadPool;

use std::error::Error;
use SensorReading;

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
    pub fn new(secret: String,
               address: String,
               temperature_barometer_addr: String,
               sampling_rate: u64)
               -> EdenClientConfig {
        EdenClientConfig {
            server_address: address,
            secret: secret,
            temperature_barometer_addr: temperature_barometer_addr,
            sampling_rate: sampling_rate,
        }
    }
}

pub trait SensorDataConsumer {
    fn attach(&self, data: Receiver<SensorReading>, batch_size: usize);
}

pub struct Client {
    parsed_address: Url,
    jwt: String,
    sender_pool: ThreadPool,
    endpoint: EdenServerEndpoint,
}

impl Client {
    pub fn new(config: EdenClientConfig, endpoint: EdenServerEndpoint) -> Result<Client, String> {
        let u = try!(Url::parse(&config.server_address).map_err(|e| e.description().to_owned()));

        let mut claim = Claim::default();
        claim.set_iss("pi");
        claim.set_payload_field("role", "sensor");
        let token = encode(&claim, &config.secret, Algorithm::HS256).unwrap();

        let pool = ThreadPool::new(4);

        Ok(Client {
            parsed_address: u,
            jwt: token,
            sender_pool: pool,
            endpoint: endpoint,
        })
    }

    pub fn send_bulk(&self, payload: Vec<Message>) {
        let path = match self.endpoint {
            EdenServerEndpoint::Temperature => "temperature".to_string(),
        };
        let body = serde_json::to_string(&payload).unwrap();
        info!("Sending: {}", body);

        let url = self.parsed_address.clone().join(&path).unwrap();
        let token = self.jwt.clone();
        let send_to = url.clone();

        self.sender_pool.execute(move || {
            match Request::new(send_to)
                .header(ContentLength(body.len() as u64))
                .header(ContentType(Mime(TopLevel::Application,
                                         SubLevel::Json,
                                         vec![(Attr::Charset, Value::Utf8)])))
                .header(Authorization(Bearer { token: token }))
                .body(body)
                .post() {
                Ok(_) => (),//Ok(()),
                Err(e) => {
                    warn!("Endpoint '{}' returned an error: {:?}", url, e);
                    // Err(EdenServerError { description: e.description().to_owned() })
                }
            }
        });
    }
}

impl SensorDataConsumer for Client {
    fn attach(&self, data: Receiver<SensorReading>, batch_size: usize) {
        let mut current_batch = 0;

        let max_timeout = Duration::from_secs(90);

        loop {
            let mut v: Vec<Message> = Vec::with_capacity(batch_size);

            loop {
                if let Ok(msg) = data.recv_timeout(max_timeout) {
                    v.push(match msg {
                        SensorReading::TemperaturePressure { t, p, ts } => {
                            Message {
                                temp: t,
                                pressure: p,
                                timestamp: ts,
                            }
                        }
                    });
                    if v.len() == batch_size {
                        break;
                    }
                } else {
                    info!("No data received for {:?}. Current queue size: {}",
                          max_timeout,
                          current_batch);
                    break;
                }
            }
            if v.len() > 0 {
                self.send_bulk(v);
                current_batch = 0;
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub temp: f32,
    pub pressure: f32,
    pub timestamp: i64,
}
