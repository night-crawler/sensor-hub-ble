#!/usr/bin/env bash
set -e

#cargo clean
cargo objcopy --release --bin ble_bas_peripheral -- -O ihex ./target/blink.hex

adafruit-nrfutil \
  dfu genpkg \
  --dev-type 0x0052 \
  --application ./target/blink.hex \
  "target/dfu.zip"

adafruit-nrfutil --verbose dfu serial \
  -pkg ./target/dfu.zip \
  -p /dev/ttyACM0 \
  -b 115200 \
  --singlebank
