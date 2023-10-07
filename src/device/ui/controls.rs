
#[derive(Copy, Clone, Debug,  Default, defmt::Format, PartialEq, Eq)]
pub(crate) struct ButtonState {
    pub(crate) top_left: PressState,
    pub(crate) top_right: PressState,
    pub(crate) bottom_left: PressState,
}


#[derive(Default, Copy, Clone, Debug, defmt::Format, PartialEq, Eq)]
pub(crate) enum PressState {
    Pressed,

    #[default]
    Released,
}

#[derive(Copy, Clone, Debug, defmt::Format)]
pub(crate) enum ButtonPosition {
    TopLeft,
    TopRight,
    BottomLeft,
}

#[derive(Copy, Clone, Debug, defmt::Format)]

pub(crate) enum DisplayRefreshType {
    Partial,
    Full,
}

impl ButtonState {
    pub(crate) fn update(&mut self, position: ButtonPosition, state: PressState) -> Self {
        match position {
            ButtonPosition::TopLeft => self.top_left = state,
            ButtonPosition::TopRight => self.top_right = state,
            ButtonPosition::BottomLeft => self.bottom_left = state,
        };
        self.clone()
    }
}
