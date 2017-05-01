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


extern crate hyper;
extern crate hyper_rustls;
extern crate serde;
extern crate serde_json;
extern crate scoped_pool;
use std::time::Duration;

use self::hyper::Url;
use self::hyper::client::{Client as HyperClient, IntoUrl};
use self::hyper::net::{HttpConnector, HttpsConnector};
use self::hyper_rustls::TlsClient;
use std::sync::mpsc::Receiver;
use self::hyper::header::{Authorization, Bearer, ContentType, ContentLength};
use self::hyper::mime::{Mime, TopLevel, SubLevel, Attr, Value};
use self::scoped_pool::Pool;
use std::sync::Arc;
use std::error::Error;
use SensorReading;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub meta: MetaData,
    pub data: Vec<Measurement>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurement {
    pub sensor: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaData {
    pub name: String,
}


pub trait SensorDataConsumer {
    fn attach(&self, data: Receiver<SensorReading>, batch_size: usize, max_timeout: Duration);
}

pub struct Client {
    endpoint: Url,
    jwt: String,
    sender_pool: Pool,
    client: Arc<HyperClient>,
}

impl Client {
    pub fn new<U: IntoUrl>(endpoint: U, pool_size: usize, token: String) -> Result<Client, String> {
        let u = try!(endpoint.into_url().map_err(|e| e.description().to_owned()));

        let pool = Pool::new(pool_size);
        let hyper_client = match u.scheme() {
            "http" => HyperClient::with_connector(HttpConnector {}),
            "https" => HyperClient::with_connector(HttpsConnector::new(TlsClient::new())),
            _ => return Err("Unknown URL scheme".to_string()),
        };
        let client = Arc::new(hyper_client);
        Ok(Client {
            endpoint: u,
            jwt: token,
            sender_pool: pool,
            client: client,
        })
    }

    pub fn send_bulk(&self, payload: Vec<Message>) {

        let body = serde_json::to_string(&payload).unwrap();
        debug!("Sending: {}", body);
        self.sender_pool.scoped(|scope| {
            let send_to = self.endpoint.clone();
            let token = self.jwt.clone();
            let ref client = self.client;

            scope.execute(move || {
                match client.post(send_to)
                    .header(ContentLength(body.len() as u64))
                    .header(ContentType(Mime(TopLevel::Application,
                                             SubLevel::Json,
                                             vec![(Attr::Charset, Value::Utf8)])))
                    .header(Authorization(Bearer { token: token }))
                    .body(&body)
                    .send() {
                    Ok(_) => (),//Ok(()),
                    Err(e) => {
                        warn!("Endpoint '{}' returned an error: {:?}", self.endpoint, e);
                    }
                }
            });
        });
    }
}

impl SensorDataConsumer for Client {
    fn attach(&self, data: Receiver<SensorReading>, batch_size: usize, max_timeout: Duration) {
        loop {
            let mut v: Vec<Message> = Vec::with_capacity(batch_size);

            loop {
                if let Ok(msg) = data.recv_timeout(max_timeout) {
                    v.push(match msg {
                        SensorReading::TemperaturePressure { sensor, t, p, ts } => {
                            Message {
                                meta: MetaData { name: sensor },
                                data: vec![Measurement {
                                               value: t as f64,
                                               unit: "celsius".to_string(),
                                               sensor: "temperature".to_string(),
                                           },
                                           Measurement {
                                               value: p as f64,
                                               unit: "kilopascal".to_string(),
                                               sensor: "barometer".to_string(),
                                           }],
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
                          v.len());
                    break; // send!
                }
            }
            if v.len() > 0 {
                self.send_bulk(v);
            }
        }
    }
}
