# SPIDAP

[![crates.io](https://img.shields.io/crates/v/spidap.svg)](https://crates.io/crates/spidap)
[![docs.rs](https://docs.rs/spidap/badge.svg)](https://docs.rs/spidap)
![CI](https://github.com/adamgreig/spidap/workflows/CI/badge.svg)

SPIDAP allows you to use CMSIS-DAP compatible probes in JTAG mode to access
SPI flash memories directly, using the JTAG signals to emulate SPI.

The probe must be connected directly to the SPI flash:

| JTAG signal | SPI Flash |
|-------------|-----------|
| JTMS        | CS
| JTCK        | CLK
| JTDI        | SDI
| JTDO        | SDO

This crate uses [jtagdap] to handle CMSIS-DAP and JTAG, and [spi-flash-rs] to
handle the SPI flash itself. For programming ECP5 FPGAs over JTAG using
CMSIS-DAP probes, check out [ecpdap], which uses the same libraries.

[jtagdap]: https://github.com/adamgreig/jtagdap
[spi-flash-rs]: https://github.com/adamgreig/spi-flash-rs
[ecpdap]: https://github.com/adamgreig/ecpdap

## Pre-built Binaries

Pre-built binaries are available for Windows and Linux on the [Releases] page.
You must have [libusb] installed or available on your system, and you must
have permissions or drivers set up to access your CMSIS-DAP probe.

[Releases]: https://github.com/adamgreig/spidap/releases
[libusb]: https://libusb.info

## Building

* You must have a working Rust compiler installed.
  Visit [rustup.rs](https://rustup.rs) to install Rust.
* [libusb] is recommended to use the higher-speed CMSIS-DAPv2 protocol, where
  supported by your probe.
* You may need to set up drivers or permissions to access the USB device.

To build and install for your user, without checking out the repository:

```
cargo install spidap
```

Or, building locally after checking out this repository:

```
cargo build --release
```

You can either run the spidap executable directly from `target/release/spidap`,
or you can install it for your user using `cargo install --path .`.

## Usage

Run `spidap help` for detailed usage. Commonly used commands:

* `spidap probes`: List all detected CMSIS-DAP probes
* `spidap id`: Read the flash manufacturer and product IDs
* `spidap scan`: Read the flash SFDP metadata and status registers
* `spidap write data.bit`: Write `data.bit` to flash memory.

## Licence

spidap is licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
