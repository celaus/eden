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

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;

use tokio::prelude::*;
use tokio::timer::Interval;

use std::time::{Duration, Instant};


mod client;
mod error;
mod config;
mod auth;

use bmp085::*;
use i2cdev::linux::LinuxI2CDevice;
use bmp085::sensors::{Barometer, Thermometer};

use std::fs::File;
use std::sync::mpsc::channel;
use std::thread;
use chrono::Utc;
use config::{Settings, read_config};

use client::{Client, SensorDataConsumer};
use clap::{Arg, App};
use auth::get_token;

#[derive(Debug)]
pub enum SensorReading {
    TemperaturePressure {
        sensor: String,
        t: f32,
        p: f32,
        ts: i64,
    },
}

fn main() {
    let matches = App::new("Eden Client")
        .version("0.4.0")
        .author("Claus Matzinger. <claus.matzinger+kb@gmail.com>")
        .about("Reads sensor input and sends it to an Eden Server :)")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .help("Sets a custom config file [default: config.toml]")
            .value_name("config.toml")
            .takes_value(true))
        .arg(Arg::with_name("logging")
            .short("l")
            .long("logging-conf")
            .value_name("logging.yml")
            .takes_value(true)
            .help("Sets the logging configuration [default: logging.yml]"))
        .get_matches();

    let config_filename = matches.value_of("config").unwrap_or("config.toml");
    let logging_filename = matches.value_of("logging").unwrap_or("logging.yml");
    info!("Using configuration file '{}' and logging config '{}'",
          config_filename,
          logging_filename);

    log4rs::init_file(logging_filename, Default::default()).expect("Could not initialize log4rs.");
    let mut f = File::open(config_filename).expect("Could not open config file.");
    let settings: Settings = read_config(&mut f).expect("Could not read config file.");

    info!("Initializing devices");
    let i2c_dev = LinuxI2CDevice::new(settings.sensors.temperature_barometer_addr.clone(),
                                      BMP085_I2C_ADDR)
        .expect("Could not open i2c device.");
    let mut temperature_barometer = BMP085BarometerThermometer::new(i2c_dev,
                                                                    SamplingMode::Standard)
        .expect("Could not initialize sensor driver");

    info!("Starting Eden");
    let device_info = settings.device.clone();
    let token = get_token(device_info.name,
                          device_info.role,
                          settings.server.secret.clone())
        .expect("Could not create token.");

    let client = Client::new(&settings.server.endpoint, settings.threads.send_pool, token)
        .expect("Could not create client.");

    let (tx, rx) = channel::<SensorReading>();
    let timeout = settings.sensors.timeout;
    let dispatcher = thread::spawn(move || {
        client.attach(rx, 100, Duration::from_secs(timeout));
    });


    info!("Starting Event Loop");

    let task = Interval::new(Instant::now(), Duration::from_secs(10))
        .for_each(|_instant| {
            let now = Utc::now();
            let now_ms = now.timestamp() * 1000 + (now.timestamp_subsec_millis() as i64);
            let temp = SensorReading::TemperaturePressure {
                sensor: settings.sensors.temperature_barometer_name.clone(),
                t: temperature_barometer.temperature_celsius().expect("Could not get temperature."),
                p: temperature_barometer.pressure_kpa().expect("Could not get pressure"),
                ts: now_ms
            };
            // fire and forget
            let _ = tx.send(temp);
            Ok(())
        })
        .map_err(|e| panic!("delay errored; err={:?}", e));

    tokio::run(task);
}
