use core::mem;
use defmt::info;

use embassy_nrf::saadc::{AnyInput, ChannelConfig, Oversample, Resistor, Resolution, Saadc};
use embassy_nrf::{peripherals, saadc};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use nrf_softdevice::ble::Connection;
use rclite::Arc;

use crate::common::ble::conv::ConvExt;
use crate::common::ble::{ADC_EVENT_PROCESSOR, SERVER};
use crate::common::device::device_manager::{Irqs, SaadcPins};
use crate::notify_all;

#[embassy_executor::task]
pub(crate) async fn read_saadc_task(saadc_pins: Arc<Mutex<ThreadModeRawMutex, SaadcPins<7>>>) {
    let server = SERVER.get();
    loop {
        let _token = ADC_EVENT_PROCESSOR.wait_for_condition().await;

        let measurements = {
            let saadc_pins = saadc_pins.lock().await;
            let mut adc = init_adc(&saadc_pins.pins, &saadc_pins.adc);
            adc.calibrate().await;
            let mut measurements = [0i16; 7];
            adc.sample(&mut measurements).await;
            measurements
        };

        let voltages = compute_voltages(&measurements, 3.6);
        let serialized_voltages = serialize_voltages(voltages);

        info!(
            "SAADC\n\traw: {:?}\n\tvoltages: {:?}\n\tserialized: {:?}",
            measurements, voltages, serialized_voltages
        );

        notify_all!(
            ADC_EVENT_PROCESSOR,
            server.adc,
            voltage0 = &serialized_voltages[0],
            voltage1 = &serialized_voltages[1],
            voltage2 = &serialized_voltages[2],
            voltage3 = &serialized_voltages[3],
            voltage4 = &serialized_voltages[4],
            voltage5 = &serialized_voltages[5],
            voltage6 = &serialized_voltages[6],
            voltage7 = &serialized_voltages[7]
        );

        Timer::after(ADC_EVENT_PROCESSOR.get_timeout_duration()).await;
    }
}

fn init_adc<'a, const N: usize>(
    pins: &'a [AnyInput; N],
    adc: &'a peripherals::SAADC,
) -> Saadc<'a, N> {
    let mut config = saadc::Config::default();
    config.oversample = Oversample::OVER256X;
    config.resolution = Resolution::_14BIT;

    let mut channel_configs: [ChannelConfig; N] = unsafe { mem::zeroed() };
    for (index, pin) in pins.into_iter().enumerate() {
        let mut channel_cfg = ChannelConfig::single_ended(pin);
        channel_cfg.resistor = Resistor::PULLDOWN;
        channel_configs[index] = channel_cfg;
    }

    let saadc = Saadc::new(adc, Irqs, config, channel_configs);
    saadc
}

fn compute_voltages<const N: usize>(adc_readings: &[i16; N], reference_voltage: f32) -> [f32; N] {
    let mut voltages = [0f32; N];
    static MAX_14_BIT: f32 = (1 << 15 - 1) as f32;
    let adc_readings_iter = adc_readings.iter().copied().map(|reading| reading as f32);
    voltages.iter_mut().zip(adc_readings_iter).for_each(|(voltage, reading)| {
        *voltage = reading / MAX_14_BIT * reference_voltage;
    });
    voltages
}

fn serialize_voltages<const N: usize>(raw_values: [f32; N]) -> [u16; N] {
    let mut ble_repr_values = [0u16; N];
    ble_repr_values.iter_mut().zip(raw_values).for_each(|(ble, raw)| {
        *ble = raw.as_voltage();
    });
    ble_repr_values
}
