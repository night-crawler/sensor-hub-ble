use embassy_nrf::gpio::{AnyPin, Input, Output};

use crate::common::device::ext::spi::SpimWrapper;

pub(crate) struct EpdControls<'a, I: SpimWrapper> {
    interface: &'a mut I,
    busy: &'a mut Input<'static, AnyPin>,
    cs: &'a mut Output<'static, AnyPin>,
    dc: &'a mut Output<'static, AnyPin>,
    rst: &'a mut Output<'static, AnyPin>,
}

impl<'a, I: SpimWrapper> EpdControls<'a, I> {
    pub(crate) fn new(
        interface: &'a mut I,
        busy: &'a mut Input<'static, AnyPin>,
        cs: &'a mut Output<'static, AnyPin>,
        dc: &'a mut Output<'static, AnyPin>,
        rst: &'a mut Output<'static, AnyPin>,
    ) -> Self {
        Self {
            interface,
            busy,
            cs,
            dc,
            rst,
        }
    }
}