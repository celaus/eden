#![feature(plugin, custom_attribute, proc_macro)]
#![plugin(docopt_macros)]

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
extern crate time;

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
use client::{Client, SensorDataConsumer, EdenClientConfig, EdenServerEndpoint};

#[derive(Debug)]
pub enum SensorReading {
    TemperaturePressure {
        t: f32,
        p: f32,
        ts: i64
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

    return Ok(EdenClientConfig::new(secret, raw_addr, temperature_barometer_addr, sampling_rate));
}


fn main() {
    let logging_filename = "logging.yml";
    log4rs::init_file(logging_filename, Default::default()).unwrap();

    let mut f = File::open("./config.toml").unwrap();
    let o = read_config(&mut f).unwrap();
    let sampling_rate = o.sampling_rate;

    info!("Initializing devices");
    let i2c_dev = LinuxI2CDevice::new(o.temperature_barometer_addr.clone(), BMP085_I2C_ADDR).unwrap();
    let mut temperature_barometer = BMP085BarometerThermometer::new(i2c_dev, SamplingMode::Standard).unwrap();

    info!("Starting Eden");
    let client = Client::new(o).unwrap();

    let (tx, rx) = channel::<(EdenServerEndpoint, SensorReading)>();

    thread::spawn(move || {
        client.attach(rx);
    });

    mioco::start(move || {
            info!("Starting Event Loop");
            loop {
                let mut timer = mioco::timer::Timer::new();
                timer.set_timeout(sampling_rate);
                select!(
                r:timer => {
                    let now = time::now().to_timespec();
                    let temp = SensorReading::TemperaturePressure {
                        t: temperature_barometer.temperature_celsius().unwrap(),
                        p: temperature_barometer.pressure_kpa().unwrap(),
                        ts: now.sec * 1000,
                    };
                    tx.send((EdenServerEndpoint::Temperature, temp));
                });
            }
        })
        .unwrap();
}
