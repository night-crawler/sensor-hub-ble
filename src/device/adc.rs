use core::sync::atomic::{AtomicU32, Ordering};

use embassy_nrf::saadc::{CallbackResult, Saadc};
use embassy_nrf::timer::Frequency;
use embassy_time::{Duration, Instant, Timer};
use nrf_softdevice::ble::Connection;
use paste::paste;

use crate::common::ble::conv::ConvExt;
use crate::common::ble::services::{AdcService, BleServer};
use crate::impl_set_many;

pub static ADC_TIMEOUT: AtomicU32 = AtomicU32::new(1000);

pub(crate) async fn notify_adc_value<'a>(saadc: &'a mut Saadc<'_, 6>, server: &'a BleServer, connection: &'a Connection) {
    let mut count = 0;
    let mut bufs = [[[0; 6]; 200]; 2];
    let mut accum: [f32; 6] = [0f32; 6];

    saadc.calibrate().await;

    let mut t0 = unsafe { embassy_nrf::peripherals::TIMER2::steal() };
    let mut ppi0 = unsafe { embassy_nrf::peripherals::PPI_CH10::steal() };
    let mut ppi1 = unsafe { embassy_nrf::peripherals::PPI_CH11::steal() };

    loop {
        let start_time = Instant::now();
        saadc
            .run_task_sampler(
                &mut t0,
                &mut ppi0,
                &mut ppi1,
                Frequency::F1MHz,
                1000, // We want to sample at 1KHz
                &mut bufs,
                move |bufs| {
                    for buf in bufs {
                        accum.iter_mut().zip(buf).for_each(|(prev, next)| {
                            *prev += (*next as f32 - *prev) * 0.05;
                        })
                    }
                    count += bufs.len();

                    if count > 100 {
                        let voltages = compute_voltages(&accum, 3.6);
                        let voltages = serialize_voltages(voltages);
                        server.adc.notify_all_voltages(connection, voltages.as_ref());
                        let _ = server.adc.samples_notify(connection, &(count as u16));
                        count = 0;
                        return CallbackResult::Stop;
                    }
                    CallbackResult::Continue
                },
            )
            .await;
        let elapsed = start_time.elapsed().as_micros();
        let _ = server.adc.elapsed_notify(connection, &elapsed);

        Timer::after(Duration::from_millis(ADC_TIMEOUT.load(Ordering::Relaxed) as u64)).await;
    }
}


pub(crate) trait NotifyAllAdcVoltage {
    fn notify_all_voltages(&self, conn: &Connection, voltages: &[u16]);
}

impl NotifyAllAdcVoltage for AdcService {
    fn notify_all_voltages(&self, conn: &Connection, voltages: &[u16]) {
        impl_set_many!(self, conn, voltage, voltages, 0, 1, 2, 3, 4, 5);
    }
}


fn compute_voltages<const N: usize>(adc_readings: &[f32; N], reference_voltage: f32) -> [f32; N] {
    let mut voltages = [0f32; N];
    voltages.iter_mut().zip(adc_readings).for_each(|(voltage, &reading)| {
        *voltage = reading / 4095f32 * reference_voltage;
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


#[macro_export]
macro_rules! impl_set_many {
     (
        $self:ident, $conn:ident, $characteristic:ident, $arr:ident,
            $(
                $field:literal
            ),+
    ) => {
         paste! {
             $(

                if let Some(val) = $arr.get($field) {
                    match $self.[<$characteristic $field _notify>]($conn, val) {
                        Ok(_) => {}
                        Err(_) => {
                            let _ = $self.[<$characteristic $field _set>](val);
                        }
                    }
                }
            )+

         }
    }
}