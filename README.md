## MS88SF3

[board](https://www.minew.com/uploads/MS88SF3_V1.1-nRF52840-Datasheet.pdf)

## BLE

[Service UUIDs](https://bitbucket.org/bluetooth-SIG/public/src/main/assigned_numbers/uuids/service_uuids.yaml)
[Characteristic UUIDs](https://bitbucket.org/bluetooth-SIG/public/src/main/assigned_numbers/uuids/characteristic_uuids.yaml)

# Credits
- [bme280](https://github.com/VersBinarii/bme280-rs)
- [lis2dh12](https://github.com/tkeksa/lis2dh12)

```bash
probe-rs-cli erase --chip nrf52840
probe-rs-cli download --chip nrf52840 --format hex s140_nrf52_7.3.0_softdevice.hex
``