#[macro_export]
macro_rules! impl_set_notification {
    (
        $typ:ty, $event:ident, $dst:ident,
            $(
                $field:ident
            ),+
    ) => {
        paste::paste! {
            match $event {
                $(
                    $typ::[<$field CccdWrite>] { notifications } => {
                        // info!("{} {} notifications: {}", stringify!($typ), stringify!($field), notifications);
                        $dst.[<$field:snake:lower>] = notifications;
                    }
                )+
                _ => {}
            }
        }
    }
 }

#[macro_export]
macro_rules! impl_timeout_event_characteristic {
    (
        $event_type:ty
    ) => {
        impl TimeoutEventCharacteristic for $event_type {
            fn get_timeout(&self) -> Option<u32> {
                match self {
                    Self::TimeoutWrite(timeout) => Some(*timeout),
                    _ => None,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_read_event_channel {
    (
        $name:literal, $channel:ident, $processor:ident
    ) => {
        paste::paste! {
            #[embassy_executor::task]
            pub(crate) async fn [<read_ $name _notification_settings_channel>]() {
                loop {
                    let (connection, settings) = $channel.receive().await;
                    $processor.process_event(connection, settings).await;
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_is_task_enabled {
    (
        $typ:ty, $($field:ident),+
    ) => {
        impl IsTaskEnabled for $typ {
            fn is_task_enabled(&self) -> bool {
                $(
                    self.$field ||
                )+ false
            }
        }
    }
}

#[macro_export]
macro_rules! impl_settings_event_consumer {
    (
        $settings_type:ty, $event_type:ty,  $($field:ident),+
    ) => {
        impl SettingsEventConsumer<$event_type> for $settings_type {
            async fn consume(&mut self, event: $event_type) {
                impl_set_notification!(
                    $event_type,
                    event,
                    self,
                    $($field),+
                );
            }
        }
    }
}
