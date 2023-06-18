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
                        info!("{} {} notifications: {}", stringify!($typ), stringify!($field), notifications);
                        $dst.[<$field:snake:lower>] = notifications;
                    }
                 )+
                 _ => {}
             }
         }
    }
 }
