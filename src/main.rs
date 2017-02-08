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


#![feature(plugin, custom_attribute)]

extern crate clap;
extern crate chrono;
extern crate toml;
#[macro_use]
extern crate mioco;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate log4rs;
extern crate bmp085;
extern crate i2cdev;

use bmp085::*;
use i2cdev::linux::*;
use i2cdev::sensors::{Barometer, Thermometer};

use std::io;
use std::io::prelude::*;
use std::fs::File;
use toml::Value;
use std::sync::mpsc::channel;
use std::thread;
use chrono::*;

mod client;
mod error;
use client::{Client, SensorDataConsumer, EdenClientConfig, EdenServerEndpoint};
use clap::{Arg, App};

#[derive(Debug)]
pub enum SensorReading {
    TemperaturePressure {
        t: f32,
        p: f32,
        ts: i64,
    },
}

fn read_config<T: Read + Sized>(mut f: T) -> Result<EdenClientConfig, io::Error> {
    let mut buffer = String::new();
    try!(f.read_to_string(&mut buffer));
    let root: Value = buffer.parse().unwrap();
    let secret = root.lookup("keys.secret")
        .unwrap_or(&Value::String("asdf".to_owned()))
        .as_str()
        .unwrap()
        .to_owned();

    let raw_addr = root.lookup("settings.server_address")
        .unwrap_or(&Value::String("http://localhost:6200/".to_owned()))
        .as_str()
        .unwrap()
        .to_owned();

    let temperature_barometer_addr = root.lookup("sensors.temperature_barometer_addr")
        .unwrap_or(&Value::String("/dev/i2c-1".to_owned()))
        .as_str()
        .unwrap()
        .to_owned();

    let sampling_rate = root.lookup("sensors.sampling_rate")
        .unwrap_or(&Value::Integer(1000))
        .as_integer()
        .unwrap() as u64;

    Ok(EdenClientConfig::new(secret, raw_addr, temperature_barometer_addr, sampling_rate))
}


fn main() {
    let matches = App::new("Eden Client")
        .version("0.1.0")
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
    let o = read_config(&mut f).unwrap();

    let sampling_rate = o.sampling_rate;

    info!("Initializing devices");
    let i2c_dev = LinuxI2CDevice::new(o.temperature_barometer_addr.clone(), BMP085_I2C_ADDR)
        .unwrap();
    let mut temperature_barometer =
        BMP085BarometerThermometer::new(i2c_dev, SamplingMode::Standard).unwrap();

    info!("Starting Eden");
    let client = Client::new(o, EdenServerEndpoint::Temperature).unwrap();

    let (tx, rx) = channel::<SensorReading>();

    let dispatcher = thread::spawn(move || {
        client.attach(rx, 100);
    });

    mioco::start(move || {
            info!("Starting Event Loop");
            loop {
                let mut timer = mioco::timer::Timer::new();
                timer.set_timeout(sampling_rate);
                select!(
                r:timer => {
                    let now = UTC::now();
                    let now_ms = now.timestamp() * 1000 + (now.timestamp_subsec_millis() as i64);
                    let temp = SensorReading::TemperaturePressure {
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
