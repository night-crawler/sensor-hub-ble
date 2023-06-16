use embedded_graphics_core::prelude::PixelColor;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    /// Black color
    Black,
    /// White color
    White,
}

impl Color {
    /// Get the color encoding of the color for one bit
    pub fn get_bit_value(self) -> u8 {
        match self {
            Color::White => 1u8,
            Color::Black => 0u8,
        }
    }

    /// Gets a full byte of black or white pixels
    pub fn get_byte_value(self) -> u8 {
        match self {
            Color::White => 0xff,
            Color::Black => 0x00,
        }
    }

    /// Parses from u8 to Color
    fn from_u8(val: u8) -> Self {
        match val {
            0 => Color::Black,
            1 => Color::White,
            e => panic!(
                "DisplayColor only parses 0 and 1 (Black and White) and not `{}`",
                e
            ),
        }
    }

    /// Returns the inverse of the given color.
    ///
    /// Black returns White and White returns Black
    pub fn inverse(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

/// Color trait for use in `Display`s
pub trait ColorType: PixelColor {
    /// Number of bit used to represent this color type in a single buffer.
    /// To get the real number of bits per pixel you should multiply this by `BUFFER_COUNT`
    const BITS_PER_PIXEL_PER_BUFFER: usize;

    /// Number of buffer used to represent this color type
    /// splitted buffer like tricolo is 2, otherwise this should be 1.
    const BUFFER_COUNT: usize;

    /// Return the data used to set a pixel color
    ///
    /// * bwrbit is used to tell the value of the unused bit when a chromatic
    /// color is set (TriColor only as for now)
    /// * pos is the pixel position in the line, used to know which pixels must be set
    ///
    /// Return values are :
    /// * .0 is the mask used to exclude this pixel from the byte (eg: 0x7F in BiColor)
    /// * .1 are the bits used to set the color in the byte (eg: 0x80 in BiColor)
    ///      this is u16 because we set 2 bytes in case of split buffer
    fn bitmask(&self, bwrbit: bool, pos: u32) -> (u8, u16);
}

impl ColorType for Color {
    const BITS_PER_PIXEL_PER_BUFFER: usize = 1;
    const BUFFER_COUNT: usize = 1;
    fn bitmask(&self, _bwrbit: bool, pos: u32) -> (u8, u16) {
        let bit = 0x80 >> (pos % 8);
        match self {
            Color::Black => (!bit, 0u16),
            Color::White => (!bit, bit as u16),
        }
    }
}

impl PixelColor for Color {
    type Raw = ();
}

impl From<Color> for embedded_graphics_core::pixelcolor::Rgb888 {
    fn from(color: Color) -> Self {
        use embedded_graphics_core::pixelcolor::RgbColor;
        match color {
            Color::Black => embedded_graphics_core::pixelcolor::Rgb888::BLACK,
            Color::White => embedded_graphics_core::pixelcolor::Rgb888::WHITE,
        }
    }
}
