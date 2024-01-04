# Sensor HUB BLE

Embassy commit hash: `6627824c02f5efc092c1a1fd77914fdde9d15813`

## Features

- [x] BME280 (temperature, humidity, pressure)
- [x] LIS2DH12 (accelerometer)
- [x] VEML6040 (rgb, white, cct, lux)
- [x] nRF ADC for analog sensors
- [x] Sensor reading exposed via BLE
- [x] E-Paper display
- [x] Display force update (WIP) by buttons
- [x] Sensors are not polled unless there's a connection and there's enough light
- [x] SPI/I2C expander (allows to attach additional sensors/devices and drive them over BLE)
- [x] PoC BLE data [collector](https://github.com/night-crawler/sensor-hub-ble-collector/)
- [ ] Additional ADC (driver not implemented yet)
- [ ] Pairing & Encryption (now all sensor reading are world-readable/writable)

## Assets

[Easy EDA Project](https://github.com/night-crawler/sensor-hub-ble/files/13175419/Sensor.Hub.Board.zip)
[Schematics](https://github.com/night-crawler/sensor-hub-ble/files/13175421/SCH_Sensor.Hub.Schematics_2023-10-26.pdf)

![image](https://github.com/night-crawler/sensor-hub-ble/assets/1235203/3d844fa4-711c-420f-a658-379bbeeb739b)
![image](https://github.com/night-crawler/sensor-hub-ble/assets/1235203/1b0716de-e52c-4a31-af76-9b71af7a0f70)

## MS88SF3

[board](https://www.minew.com/uploads/MS88SF3_V1.1-nRF52840-Datasheet.pdf)

## BLE

[Service UUIDs](https://bitbucket.org/bluetooth-SIG/public/src/main/assigned_numbers/uuids/service_uuids.yaml)
[Characteristic UUIDs](https://bitbucket.org/bluetooth-SIG/public/src/main/assigned_numbers/uuids/characteristic_uuids.yaml)

# Credits

- [Embassy](https://github.com/embassy-rs/embassy)
- [BME280](https://github.com/VersBinarii/bme280-rs)
- [LIS2DH12](https://github.com/tkeksa/lis2dh12)
- [VEML6040](https://github.com/eldruin/veml6040-rs)

```bash
probe-rs-cli erase --chip nrf52840
probe-rs-cli download --chip nrf52840 --format hex s140_nrf52_7.3.0_softdevice.hex
```

## Board Errata

1. Consult the nRF52840 pinout and use recommended !low-frequency pins for i2c/spi.
2. There must be a separate power switch for i2c expander 
   (now it's on the same power rail as the SPI expander).
3. E-Paper display resistor `R.WS1` (`rese` pull-down resistor) must be 2 Ohm, not 4.7K. 
   It leads to display going black. Can be fixed with a jumper wire/short across the current one.
4. Capacitive sensors (and probably all other) must be easy replaceable and attachable
   (who knows what solvent was used to clean up the board? was it safe for the sensor?).
5. MS88SF3 has an internal LDO + buck converter that must be used in the next revision. 
   Current LDO has 4mA quiescent current which is insane for a battery-powered device.
6. MS88SF3 should not be placed in the middle of th board because it reduces the range of the BLE signal.
7. The second light sensor (VEML6040) was supposed to be a UV Sensor. 
   Now it must be removed from the board to resolve i2c address conflicts. 
8. Onboard USB must be soldered. Charging must be implemented on the board.
