# Eden client: A sensor agent for your Raspberry Pi

[![Build Status](https://travis-ci.org/celaus/eden.svg?branch=master)](https://travis-ci.org/celaus/eden)

Eden is an IoT use case for monitoring temperature data off a Raspberry Pi.

# Run

## Requirements

A [BMP085 or BMP180] temperature sensor and barometer, connected via i2c to a Linux device 🤔

## Using Docker

This will pull and run a the Docker image created from the [Dockerfile](Dockerfile) in this repository. This will require a ARMv6 compatible CPU architecture (or OS), i.e. a Raspberry Pi v1, 2, 3, Zero or comparable.

`docker run -d --device /dev/i2c-1 clma/eden:arm`

## Build

```
git clone https://github.com/celaus/eden
cd eden
cargo build --release
cd /target/release/
./eden -c ../../config.toml -l ../../logging.yml
```
Bear in mind that this requires permission to access the i2c device(s).

## Download ARMv6 Binaries

If you are not too paranoid, you can also download prebuilt binaries directly.

[ARM](https://x5ff.xyz:8080/builds/eden-arm-latest.tgz)

[x86](https://x5ff.xyz:8080/builds/eden-x86-latest.tgz)



# License
[Apache 2.0](LICENSE)
