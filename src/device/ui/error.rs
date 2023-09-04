use core::convert::Infallible;
use core::fmt;

use defmt::{error, Formatter};
use embassy_nrf::spim;
use thiserror_no_std::Error;

#[derive(Error, Debug)]
pub enum UiError<T> {
    #[error("Format error")]
    FmtError(#[from] fmt::Error),

    #[error("Qwe")]
    DisplayError(T),

    #[error("Qwe")]
    Infallible(#[from] Infallible),

    #[error("Spim")]
    Spim(#[from] spim::Error),
}

impl<T> defmt::Format for UiError<T> where T: defmt::Format {
    fn format(&self, fmt: Formatter) {
        match self {
            UiError::FmtError(err) => defmt::write!(fmt, "Formatting error"),
            UiError::DisplayError(err) => defmt::write!(fmt, "Display error: {}", err),
            UiError::Infallible(err) => defmt::write!(fmt, "Formatting error: {}", err),
            UiError::Spim(err) => defmt::write!(fmt, "spim error: {}", err),
        }
    }
}