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


extern crate simple_jwt;
extern crate hyper;
extern crate serde;
extern crate serde_json;
extern crate threadpool;
use std::time::Duration;

use self::hyper::Url;
use self::hyper::client::{Client as HyperClient, IntoUrl};
use std::sync::mpsc::Receiver;
use self::hyper::header::{Authorization, Bearer, ContentType, ContentLength};
use self::simple_jwt::{encode, Claim, Algorithm};
use self::hyper::mime::{Mime, TopLevel, SubLevel, Attr, Value};
use self::threadpool::ThreadPool;
use std::sync::Arc;
use std::error::Error;
use SensorReading;


#[derive(Debug)]
pub enum EdenServerEndpoint {
    Temperature,
}


pub trait SensorDataConsumer {
    fn attach(&self, data: Receiver<SensorReading>, batch_size: usize);
}

pub struct Client {
    parsed_address: Url,
    jwt: String,
    sender_pool: ThreadPool,
    endpoint: EdenServerEndpoint,
    client: Arc<HyperClient>,
}

impl Client {
    pub fn new<U: IntoUrl>(address: U, pool_size: usize, secret: String, endpoint: EdenServerEndpoint) -> Result<Client, String> {
        let u = try!(address.into_url().map_err(|e| e.description().to_owned()));

        let mut claim = Claim::default();
        claim.set_iss("pi");
        claim.set_payload_field("role", "sensor");
        let token = encode(&claim, &secret, Algorithm::HS256).unwrap();

        let pool = ThreadPool::new(pool_size);
        let client = Arc::new(HyperClient::new());
        Ok(Client {
            parsed_address: u,
            jwt: token,
            sender_pool: pool,
            endpoint: endpoint,
            client: client
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
        let client = self.client.clone();
        self.sender_pool.execute(move || {
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
                        SensorReading::TemperaturePressure { name, t, p, ts } => {
                            Message {
                                name: name,
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
    pub name: String,
    pub temp: f32,
    pub pressure: f32,
    pub timestamp: i64,
}
