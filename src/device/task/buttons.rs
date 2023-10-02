use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{AnyPin, Input, Pull};
use embassy_time::Timer;
use futures::{FutureExt, select_biased};

use crate::common::device::config::DEBOUNCE_INTERVAL;
use crate::common::device::pin_manager::ButtonPins;
use crate::common::device::ui::{BUTTON_EVENTS, BUTTON_STATE, DISPLAY_REFRESH_EVENTS};
use crate::common::device::ui::controls::{ButtonPosition, ButtonState, DisplayRefreshType, PressState};

#[embassy_executor::task]
pub(crate) async fn read_buttons(
    mut pins: ButtonPins,
) {
    select_biased! {
        _ = read_button(&mut pins.top_left, ButtonPosition::TopLeft).fuse() => {}
        _ = read_button(&mut pins.top_right, ButtonPosition::TopRight).fuse() => {}
        _ = read_button(&mut pins.bottom_left, ButtonPosition::BottomLeft).fuse() => {}
    }
}


async fn read_button(pin: &mut AnyPin, position: ButtonPosition) {
    let mut input = Input::new(pin, Pull::Down);
    loop {
        input.wait_for_high().await;
        let new_state = {
            let mut button_state = BUTTON_STATE.lock().await;
            button_state.update(position, PressState::Pressed)
        };
        BUTTON_EVENTS.send(new_state).await;

        Timer::after(DEBOUNCE_INTERVAL).await;

        input.wait_for_low().await;
        let new_state = {
            let mut button_state = BUTTON_STATE.lock().await;
            button_state.update(position, PressState::Released)
        };
        BUTTON_EVENTS.send(new_state).await;
        Timer::after(DEBOUNCE_INTERVAL).await;
    }
}

const PARTIAL_REFRESH_STATE: ButtonState = ButtonState {
    top_left: PressState::Released,
    top_right: PressState::Pressed,
    bottom_left: PressState::Released,
};

const FULL_REFRESH_STATE: ButtonState = ButtonState {
    top_left: PressState::Pressed,
    top_right: PressState::Pressed,
    bottom_left: PressState::Released,
};

#[embassy_executor::task]
pub(crate) async fn read_button_events() {
    loop {
        let state = BUTTON_EVENTS.receive().await;
        info!("Button event: {:?}", state);

        if state == PARTIAL_REFRESH_STATE {
            let _ = Spawner::for_current_executor().await.spawn(handle_refresh(DisplayRefreshType::Partial));
        } else if state == FULL_REFRESH_STATE {
            let _ = Spawner::for_current_executor().await.spawn(handle_refresh(DisplayRefreshType::Full));
        }
    }
}

#[embassy_executor::task]
async fn handle_refresh(refresh_type: DisplayRefreshType) {
    DISPLAY_REFRESH_EVENTS.send(refresh_type).await;
}