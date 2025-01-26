# Template Project for embassy-rs on the pi pico w

This repository should be used as an entrypoint to using [embassy-rs](https://github.com/embassy-rs/embassy) 
on the raspberry pi pico w (rp2040).

The file `src/main.rs` contains setup for embassy, with support for usb logging and wireless (Wi-Fi/BLE).
As an initial example, the LED is toggled and Wi-Fi devices are scanned and printed.

## Run
Execute the main file using
```shell
cargo run
```

To NOT flash the cyw43 firmware use
```shell
cargo run --features skip-cyw43-firmware
```