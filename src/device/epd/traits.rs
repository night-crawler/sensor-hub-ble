use crate::common::device::error::CustomSpimError;

/// All commands need to have this trait which gives the address of the command
/// which needs to be send via SPI with activated CommandsPin (Data/Command Pin in CommandMode)
pub trait Command: Copy {
    fn address(self) -> u8;
}

/// Seperates the different LUT for the Display Refresh process
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum RefreshLut {
    /// The "normal" full Lookuptable for the Refresh-Sequence
    Full,
    /// The quick LUT where not the full refresh sequence is followed.
    /// This might lead to some
    Quick,
}

impl Default for RefreshLut {
    fn default() -> Self {
        RefreshLut::Full
    }
}

pub(crate) trait InternalWiAdditions
{
    /// This initialises the EPD and powers it up
    ///
    /// This function is already called from
    ///  - [new()](WaveshareDisplay::new())
    ///  - [`wake_up`]
    ///
    ///
    /// This function calls [reset](WaveshareDisplay::reset),
    /// so you don't need to call reset your self when trying to wake your device up
    /// after setting it to sleep.
    async fn init(&mut self) -> Result<(), CustomSpimError>;
}

/// All the functions to interact with the EPDs
///
/// This trait includes all public functions to use the EPDs
///
/// # Example
///
///```rust, no_run
///# use embedded_hal_mock::*;
///# fn main() -> Result<(), MockError> {
///use embedded_graphics::{
///    pixelcolor::BinaryColor::On as Black, prelude::*, primitives::{Line, PrimitiveStyle},
///};
///use epd_waveshare::{epd4in2::*, prelude::*};
///#
///# let expectations = [];
///# let mut spi = spi::Mock::new(&expectations);
///# let expectations = [];
///# let cs_pin = pin::Mock::new(&expectations);
///# let busy_in = pin::Mock::new(&expectations);
///# let dc = pin::Mock::new(&expectations);
///# let rst = pin::Mock::new(&expectations);
///# let mut delay = delay::MockNoop::new();
///
///// Setup EPD
///let mut epd = Epd4in2::new(&mut spi, cs_pin, busy_in, dc, rst, &mut delay, None)?;
///
///// Use display graphics from embedded-graphics
///let mut display = Display4in2::default();
///
///// Use embedded graphics for drawing a line
///
///let _ = Line::new(Point::new(0, 120), Point::new(0, 295))
///    .into_styled(PrimitiveStyle::with_stroke(Color::Black, 1))
///    .draw(&mut display);
///
///    // Display updated frame
///epd.update_frame(&mut spi, &display.buffer(), &mut delay)?;
///epd.display_frame(&mut spi, &mut delay)?;
///
///// Set the EPD to sleep
///epd.sleep(&mut spi, &mut delay)?;
///# Ok(())
///# }
///```
pub trait WaveshareDisplay {
    /// The Color Type used by the Display
    type DisplayColor;

    /// Let the device enter deep-sleep mode to save power.
    ///
    /// The deep sleep mode returns to standby with a hardware reset.
    async fn sleep(&mut self) -> Result<(), CustomSpimError>;

    /// Wakes the device up from sleep
    ///
    /// Also reintialises the device if necessary.
    async fn wake_up(&mut self) -> Result<(), CustomSpimError>;

    /// Sets the backgroundcolor for various commands like [clear_frame](WaveshareDisplay::clear_frame)
    async fn set_background_color(&mut self, color: Self::DisplayColor);

    /// Get current background color
    async fn background_color(&self) -> &Self::DisplayColor;

    /// Get the width of the display
    fn width(&self) -> u32;

    /// Get the height of the display
    fn height(&self) -> u32;

    /// Transmit a full frame to the SRAM of the EPD
    async fn update_frame(&mut self, buffer: &[u8]) -> Result<(), CustomSpimError>;

    /// Transmits partial data to the SRAM of the EPD
    ///
    /// (x,y) is the top left corner
    ///
    /// BUFFER needs to be of size: width / 8 * height !
    #[allow(clippy::too_many_arguments)]
    async fn update_partial_frame(
        &mut self, buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), CustomSpimError>;

    /// Displays the frame data from SRAM
    ///
    /// This function waits until the device isn`t busy anymore
    async fn display_frame(&mut self) -> Result<(), CustomSpimError>;

    /// Provide a combined update&display and save some time (skipping a busy check in between)
    async fn update_and_display_frame(&mut self, buffer: &[u8]) -> Result<(), CustomSpimError>;

    /// Clears the frame buffer on the EPD with the declared background color
    ///
    /// The background color can be changed with [`WaveshareDisplay::set_background_color`]
    async fn clear_frame(&mut self) -> Result<(), CustomSpimError>;

    /// Trait for using various Waveforms from different LUTs
    /// E.g. for partial refreshes
    ///
    /// A full refresh is needed after a certain amount of quick refreshes!
    ///
    /// WARNING: Quick Refresh might lead to ghosting-effects/problems with your display. Especially for the 4.2in Display!
    ///
    /// If None is used the old value will be loaded on the LUTs once more
    async fn set_lut(&mut self, refresh_rate: Option<RefreshLut>) -> Result<(), CustomSpimError>;

    /// Wait until the display has stopped processing data
    ///
    /// You can call this to make sure a frame is displayed before goin further
    async fn wait_until_idle(&mut self) -> Result<(), CustomSpimError>;
}
