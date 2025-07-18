use std::{borrow::Cow, io};

use super::{FStringReadExt, FStringWriteExt};
use bitstream_io::{BitRead, BitWrite};

pub trait BitSerialize: Sized {
    fn read_from_bits<R: BitRead>(r: &mut R) -> io::Result<Self>;
    fn write_to_bits<W: BitWrite>(&self, w: &mut W) -> io::Result<()>;
}

impl BitSerialize for u8 {
    fn read_from_bits<R: BitRead>(r: &mut R) -> io::Result<Self> {
        r.read(8)
    }

    fn write_to_bits<W: BitWrite>(&self, w: &mut W) -> io::Result<()> {
        w.write(8, *self)
    }
}

impl BitSerialize for u16 {
    fn read_from_bits<R: BitRead>(r: &mut R) -> io::Result<Self> {
        r.read(16)
    }

    fn write_to_bits<W: BitWrite>(&self, w: &mut W) -> io::Result<()> {
        w.write(16, *self)
    }
}

impl BitSerialize for u32 {
    fn read_from_bits<R: BitRead>(r: &mut R) -> io::Result<Self> {
        r.read(32)
    }

    fn write_to_bits<W: BitWrite>(&self, w: &mut W) -> io::Result<()> {
        w.write(32, *self)
    }
}

impl BitSerialize for String {
    fn read_from_bits<R: BitRead>(r: &mut R) -> io::Result<Self> {
        r.read_string()
    }

    fn write_to_bits<W: BitWrite>(&self, w: &mut W) -> io::Result<()> {
        w.write_string(self)
    }
}

impl BitSerialize for Cow<'_, str> {
    fn read_from_bits<R: BitRead>(r: &mut R) -> io::Result<Self> {
        Ok(Cow::Owned(r.read_string()?))
    }

    fn write_to_bits<W: BitWrite>(&self, w: &mut W) -> io::Result<()> {
        w.write_string(self)
    }
}
