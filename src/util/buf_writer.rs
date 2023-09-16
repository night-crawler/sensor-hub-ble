use core::cmp::min;
use core::fmt;
use core::str::from_utf8_unchecked;

use defmt::info;

pub struct WriteTo<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl<'a> WriteTo<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        WriteTo { buf, len: 0 }
    }

    pub fn to_str(self) -> Option<&'a str> {
        if self.len <= self.buf.len() {
            Some(unsafe { from_utf8_unchecked(&self.buf[..self.len]) })
        } else {
            None
        }
    }
}

impl<'a> fmt::Write for WriteTo<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.len > self.buf.len() {
            return Err(fmt::Error);
        }

        let rem = &mut self.buf[self.len..];
        let raw_s = s.as_bytes();
        let num = min(raw_s.len(), rem.len());

        rem[..num].copy_from_slice(&raw_s[..num]);
        self.len += raw_s.len();

        if num < raw_s.len() {
            let present_string = unsafe { from_utf8_unchecked(self.buf) };
            info!("Failed to write string: {}; present: {}", s, present_string);
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}
