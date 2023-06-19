#[macro_export]
macro_rules! ble_notify {
    ($service:expr, $conn:expr, $characteristic:ident, $value:expr) => {
        paste::paste! {
            if let Err(err) = $service.[<$characteristic _notify>]($conn, $value) {
                $crate::ble_debug!("{} notify error: {:?} - {:?}", stringify!($characteristic), err, $value);
                if let Err(err) = $service.[<$characteristic _set>]($value) {
                    $crate::ble_debug!("{} notify error: {:?} - {:?}", stringify!($characteristic), err, $value);
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
        for connection in Connection::iter() {
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