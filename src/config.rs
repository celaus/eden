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

extern crate toml;
extern crate serde;

use crate::error::ConfigError;
use std::io::Read;

#[derive(Deserialize)]
pub struct Settings {
    pub sensors: Sensors,
    pub server: Server,
    pub device: Agent,
    pub threads: Threads,
}

#[derive(Deserialize)]
pub struct Sensors {
    pub sampling_rate: u64,
    pub temperature_barometer_addr: String,
    pub temperature_barometer_name: String,
    pub timeout: u64,
}

#[derive(Deserialize)]
pub struct Server {
    pub endpoint: String,
    pub secret: String,
}

#[derive(Deserialize, Clone)]
pub struct Agent {
    pub name: String,
    pub role: String,
}

#[derive(Deserialize, Clone)]
pub struct Threads {
    pub send_pool: usize,
}

pub fn read_config<T: Read + Sized>(mut f: T) -> Result<Settings, ConfigError> {
    let mut buffer = String::new();
    f.read_to_string(&mut buffer).map_err(ConfigError::Io)?;
    toml::from_str(&buffer).map_err(ConfigError::Parse)
}
