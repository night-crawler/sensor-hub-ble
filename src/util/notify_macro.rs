#[macro_export]
macro_rules! ble_notify {
    ($service:expr, $conn:expr, $characteristic:ident, $value:expr) => {
        paste::paste! {
            let __name = stringify!($characteristic);
            if let Err(err) = $service.[<$characteristic _notify>]($conn, $value) {
                if __name != "debug" {
                    $crate::ble_debug!("Notify {} error: {:?}", stringify!($characteristic), err);
                }
                if let Err(err) = $service.[<$characteristic _set>]($value) {
                    if __name != "debug" {
                        $crate::ble_debug!("Set {} notify error: {:?}", stringify!($characteristic), err);
                    }
                }
            }
        }
    };
}

#[macro_export]
macro_rules! notify_all {
    (
        $event_processor:ident, $service:expr,
        $(
            $characteristic:ident = $value:expr
        ),+
    ) => {
        for connection in nrf_softdevice::ble::Connection::iter() {
            // by default all notification settings are disabled
            let ns = if let Some(ns) = $event_processor.get_connection_settings(&connection).await {
                ns
            } else {
                defmt::info!(
                    "Notification settings were not found for connection for service {}, event_processor: {}",
                    stringify!($service),
                    stringify!($event_processor),
                );
                continue
            };

            $(
                if ns.$characteristic {
                    $crate::ble_notify!($service, &connection, $characteristic, $value);
                }
            )+
        }
    };
}
