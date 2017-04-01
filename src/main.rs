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
extern crate clap;
extern crate chrono;
#[macro_use]
extern crate mioco;
#[macro_use]
extern crate log;
extern crate log4rs;
extern crate bmp085;
extern crate i2cdev;

mod client;
mod error;
mod config;
mod auth;

use bmp085::*;
use i2cdev::linux::*;
use i2cdev::sensors::{Barometer, Thermometer};

use std::fs::File;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;
use chrono::UTC;
use config::{Settings, read_config};

use client::{Client, SensorDataConsumer};
use clap::{Arg, App};

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
        .version("0.2.0")
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

    log4rs::init_file(logging_filename, Default::default()).unwrap();
    let mut f = File::open(config_filename).unwrap();
    let settings: Settings = read_config(&mut f).unwrap();

    info!("Initializing devices");
    let i2c_dev = LinuxI2CDevice::new(settings.sensors.temperature_barometer_addr.clone(),
                                      BMP085_I2C_ADDR)
        .unwrap();
    let mut temperature_barometer =
        BMP085BarometerThermometer::new(i2c_dev, SamplingMode::Standard).unwrap();

    info!("Starting Eden");
    let client = Client::new(&settings.server.endpoint,
                             4usize,
                             settings.server.secret.clone(),
                             settings.device.name.clone())
        .unwrap();

    let (tx, rx) = channel::<SensorReading>();
    let timeout = settings.sensors.timeout;
    let dispatcher = thread::spawn(move || {
        client.attach(rx, 100, Duration::from_secs(timeout));
    });

    mioco::start(move || {
            info!("Starting Event Loop");
            loop {
                let mut timer = mioco::timer::Timer::new();
                timer.set_timeout(settings.sensors.sampling_rate.clone());
                select!(
                r:timer => {
                    let now = UTC::now();
                    let now_ms = now.timestamp() * 1000 + (now.timestamp_subsec_millis() as i64);
                    let temp = SensorReading::TemperaturePressure {
                        sensor: settings.sensors.temperature_barometer_name.clone(),
                        t: temperature_barometer.temperature_celsius().unwrap(),
                        p: temperature_barometer.pressure_kpa().unwrap(),
                        ts: now_ms
                    };
                // fire and forget
                    let _ = tx.send(temp);
                });
            }
        })
        .unwrap();
    let _ = dispatcher.join();
}
