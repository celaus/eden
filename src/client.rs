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

use reqwest::IntoUrl;

use std::time::Duration;
use std::sync::mpsc::Receiver;

use scoped_pool::Pool;
use std::sync::Arc;
use std::error::Error;
use crate::SensorReading;

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
    endpoint: reqwest::Url,
    jwt: String,
    sender_pool: Pool,
    client: Arc<reqwest::Client>,
}

impl Client {
    pub fn new<U: IntoUrl>(endpoint: U, pool_size: usize, token: String) -> Result<Client, String> {
        let u = endpoint.into_url().map_err(|e| e.description().to_owned())?;

        let pool = Pool::new(pool_size);
        let http_client = reqwest::Client::new();
        let client = Arc::new(http_client);
        Ok(Client {
            endpoint: u,
            jwt: token,
            sender_pool: pool,
            client: client,
        })
    }

    pub fn send_bulk(&self, payload: Vec<Message>) {

        self.sender_pool.scoped(|scope| {
            let send_to = self.endpoint.clone();
            let token = self.jwt.clone();
            let ref client = self.client;

            scope.execute(move || {
                match client.post(send_to)
                    //.header("ContentLength", body.len() as u64)
                    //.header("ContentType", "application/json;charset=utf-8")
                    .header("Authorization", format!("Bearer {}", token))
                    .json(&payload)
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
