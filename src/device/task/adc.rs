use core::mem;
use core::ops::DerefMut;

use embassy_nrf::{peripherals, saadc};
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::saadc::{AnyInput, ChannelConfig, Oversample, Resistor, Resolution, Saadc};
use embassy_nrf::timer::Frequency;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use rclite::Arc;

use crate::common::ble::{ADC_EVENT_PROCESSOR, DEVICE_EVENT_PROCESSOR, SERVER};
use crate::common::ble::conv::ConvExt;
use crate::common::device::peripherals_manager::{Irqs, SaadcPins};
use crate::common::device::ui::UI_STORE;
use crate::notify_all;

#[embassy_executor::task]
pub(crate) async fn read_saadc_battery_voltage_task(
    saadc_pins: Arc<Mutex<ThreadModeRawMutex, SaadcPins<8>>>,
) {
    let server = SERVER.get();

    loop {
        let _token = DEVICE_EVENT_PROCESSOR.wait_for_condition().await;

        // taking just battery pin does not work; on the second time initialization SAADC
        // ignores sample_counter from the current run
        let (measurements, _, _) = {
            let mut saadc_pins = saadc_pins.lock().await;
            let saadc_pins = saadc_pins.deref_mut();
            let mut bat_switch = Output::new(
                &mut saadc_pins.bat_switch, Level::High, OutputDrive::Standard,
            );
            bat_switch.set_high();
            Timer::after(Duration::from_millis(10)).await;

            let sample_counter = 600;
            let measurements = measure::<8, 10>(&mut saadc_pins.pins, &mut saadc_pins.adc, 1000, sample_counter)
                .await
                .unwrap();

            bat_switch.set_low();
            measurements
        };

        let mut voltages = compute_voltages(&measurements, 3.6);
        voltages[7] = calculate_voltage_divider_in(voltages[7]);
        let serialized_voltages = serialize_voltages(voltages);

        UI_STORE.lock().await.bat_voltage = voltages[7];

        // info!("Battery: {}; other: {:?}", voltages[7], voltages);

        notify_all!(
                DEVICE_EVENT_PROCESSOR,
                server.dis,
                battery_voltage = &serialized_voltages[7]
            );

        Timer::after(DEVICE_EVENT_PROCESSOR.get_timeout_duration()).await;
    }

}

#[embassy_executor::task]
pub(crate) async fn read_saadc_task(saadc_pins: Arc<Mutex<ThreadModeRawMutex, SaadcPins<8>>>) {
    let server = SERVER.get();
    loop {
        let _token = ADC_EVENT_PROCESSOR.wait_for_condition().await;

        let (measurements, elapsed, count) = {
            let mut saadc_pins = saadc_pins.lock().await;
            let saadc_pins = saadc_pins.deref_mut();
            let mut power_switch = Output::new(
                &mut saadc_pins.pw_switch, Level::High, OutputDrive::Standard,
            );
            power_switch.set_high();
            Timer::after(Duration::from_millis(10)).await;

            // 1 and values around 400 seem to work. Other values give unpredictable results:
            // i.e., [0] and [1] channels get misplaced
            // nrf doc says:
            // For continuous sampling, ensure that the sample rate fullfills the following criteria:
            // > fSAMPLE < 1/[tACQ + tconv]
            // Embassy doc says:
            // > The time spent within the callback supplied should not exceed the time taken to acquire
            // > the samples into a single buffer.
            //
            // I guess that single buffer is [[0; NUM_PINS]; BUF_SIZE]. At least one can draw such a
            // conclusion from:
            // https://github.com/embassy-rs/embassy/blob/main/examples/nrf52840/src/bin/saadc_continuous.rs#L52
            // As far as I can understand it:
            //  - acquisition time is set to 40us, we add 2us (the value is taken from saadc example)
            //  - there are 7 channels
            //  - the buffer has capacity of 10 elements
            // In total, it gives 10 * 7 * (40 + 2) == 2940us
            // As frequency, it's 1 / (10 * 7 * (40 + 2) / 1000 / 1000) == 340Hz
            // As per NRF doc formula, sampling frequency must be lower, let's say 300Hz.
            // Having a 1Mhz timer, we need to set counter to 3333. If we set sample_counter to 3333,
            // the measured time will be 10 times bigger. It might be the case, that the buffer here
            // means just one buffer for 7 channels, so I just set it to 400.
            // (now changed to 8)
            let mut sample_counter = 600;
            let measurements = loop {
                match measure::<8, 10>(&mut saadc_pins.pins, &mut saadc_pins.adc, 1000, sample_counter)
                    .await
                {
                    Ok(result) => {
                        break result;
                    }
                    Err(_) => {
                        sample_counter = sample_counter * 99 / 100;
                    }
                }
            };

            power_switch.set_low();
            measurements
        };

        let voltages = compute_voltages(&measurements, 3.6);
        {
            let mut store = UI_STORE.lock().await;
            for (index, voltage) in voltages.iter().enumerate() {
                store.nrf_adc_voltages[index] = voltage.max(0f32);
            }
        }

        let serialized_voltages = serialize_voltages(voltages);

        // info!("SAADC: {:?}\n{:?}", serialized_voltages, voltages);

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
            elapsed = &elapsed.as_micros(),
            samples = &(count as u16)
        );

        Timer::after(ADC_EVENT_PROCESSOR.get_timeout_duration()).await;
    }
}

async fn measure<const NUM_PINS: usize, const BUF_SIZE: usize>(
    pins: &mut [AnyInput; NUM_PINS],
    saadc_peripheral: &mut peripherals::SAADC,
    oversample: usize,
    mut sample_counter: u32,
) -> Result<([f32; NUM_PINS], Duration, usize), Duration> {
    sample_counter = sample_counter.max(1);
    let mut adc = init_adc(pins, saadc_peripheral);
    adc.calibrate().await;

    let mut bufs = [[[0; NUM_PINS]; BUF_SIZE]; 2];
    let mut accum: [f32; NUM_PINS] = [0f32; NUM_PINS];
    let mut count = 0;

    let mut t0 = unsafe { peripherals::TIMER2::steal() };
    let mut ppi0 = unsafe { peripherals::PPI_CH10::steal() };
    let mut ppi1 = unsafe { peripherals::PPI_CH11::steal() };

    let start_time = embassy_time::Instant::now();
    let mut last_sample_time = None;

    // (40us acquisition time + 2us) * number_of_pins * buf_size
    // let measure_time_us = (BUF_SIZE * NUM_PINS) as u64 * (40 + 2);

    let mut spent = Duration::from_millis(0);

    adc.run_task_sampler(
        &mut t0,
        &mut ppi0,
        &mut ppi1,
        Frequency::F1MHz,
        sample_counter,
        &mut bufs,
        |bufs| {
            let current_time = embassy_time::Instant::now();
            spent = current_time - last_sample_time.unwrap_or(start_time);
            // if spent.as_micros() > measure_time_us {
            // info!("Spent: {} / {}", spent.as_micros(), measure_time_us);
            // return saadc::CallbackResult::Stop;
            // }
            let _ = last_sample_time.insert(current_time);

            for buf in bufs {
                accum.iter_mut().zip(buf).for_each(|(avg, &curr)| {
                    *avg += (curr as f32 - *avg) / (count + 1) as f32;
                });
                count += 1;
            }

            if count > oversample {
                return saadc::CallbackResult::Stop;
            }
            saadc::CallbackResult::Continue
        },
    )
        .await;
    let elapsed = start_time.elapsed();

    // if count < oversample {
    //     return Err(spent);
    // }

    Ok((accum, elapsed, count))
}

fn init_adc<'a, const N: usize>(
    pins: &'a mut [AnyInput; N],
    adc: &'a mut peripherals::SAADC,
) -> Saadc<'a, N> {
    let mut config = saadc::Config::default();
    config.oversample = Oversample::BYPASS;
    config.resolution = Resolution::_14BIT;

    let mut channel_configs: [ChannelConfig; N] = unsafe { mem::zeroed() };
    for (index, pin) in pins.into_iter().enumerate() {
        let mut channel_cfg = ChannelConfig::single_ended(pin);
        channel_cfg.resistor = Resistor::BYPASS;
        channel_cfg.time = saadc::Time::_40US;
        channel_configs[index] = channel_cfg;
    }

    Saadc::new(adc, Irqs, config, channel_configs)
}

fn compute_voltages<const N: usize>(adc_readings: &[f32; N], reference_voltage: f32) -> [f32; N] {
    let mut voltages = [0f32; N];
    static MAX_14_BIT: f32 = (1 << 15 - 1) as f32;
    let adc_readings_iter = adc_readings.iter().copied().map(|reading| reading);
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

fn calculate_voltage_divider_in(v_out: f32) -> f32 {
    let r1 = 47_000.0f32;
    let r2 = 100_000.0f32;
    v_out * (r1 + r2) / r2
}
