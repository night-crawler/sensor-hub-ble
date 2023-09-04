use core::cmp::min;
use core::fmt;
use core::str::from_utf8_unchecked;

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

    pub fn fmt(mut self, args: fmt::Arguments) -> Result<&'a str, fmt::Error> {
        fmt::write(&mut self, args)?;
        match self.to_str() {
            None => Err(fmt::Error),
            Some(s) => Ok(s)
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

        if num < raw_s.len() { Err(fmt::Error) } else { Ok(()) }
    }
}
