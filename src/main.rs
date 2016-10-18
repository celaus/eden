#![feature(custom_derive, plugin, associated_consts,  custom_attribute)]
#![plugin(docopt_macros)]

extern crate chrono;
extern crate toml;
extern crate rustc_serialize;
#[macro_use]
extern crate mioco;
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

mod client;
use client::{Client, SensorDataConsumer, EdenClientConfig};

#[derive(Debug)]
pub enum SensorReading {
    TemperaturePressure {
        t: f32,
        p: f32,
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
    return Ok(EdenClientConfig::new(secret, raw_addr));
}


fn main() {
    let logging_filename = "logging.yml";
    log4rs::init_file(logging_filename, Default::default()).unwrap();

    let mut f = File::open("./config.toml").unwrap();
    let o = read_config(&mut f).unwrap();

    info!("Initializing devices");
    let i2c_dev = LinuxI2CDevice::new("/dev/i2c-1", BMP085_I2C_ADDR).unwrap();
    let mut temperature_barometer = BMP085BarometerThermometer::new(i2c_dev, SamplingMode::Standard).unwrap();

    info!("Starting Eden");
    let client = Client::new(o).unwrap();

    let (tx, rx) = channel::<SensorReading>();

    let guard = thread::spawn(move || {
        client.attach(rx);
    });

    mioco::start(move || {
            info!("Starting Event Loop");
            loop {
                let mut timer = mioco::timer::Timer::new();
                timer.set_timeout(1000);
                select!(
                r:timer => {
                    let temp = SensorReading::TemperaturePressure {
                        t: temperature_barometer.temperature_celsius().unwrap(),
                        p: temperature_barometer.pressure_kpa().unwrap()
                    };
                    tx.send(temp);
                });
            }
        })
        .unwrap();
}
