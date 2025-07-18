use bitstream_io::{BitRead, BitWrite};
use std::io;

use crate::vector::FVector3d;

use super::FName;

pub trait PackedBitReadExt {
    fn read_packed_int(&mut self) -> io::Result<u32>;
    fn read_packed_u16(&mut self) -> io::Result<u16>;
}

pub trait PackedBitWriteExt {
    fn write_packed_int(&mut self, value: u32) -> io::Result<()>;
}

pub trait FStringReadExt {
    fn read_string(&mut self) -> io::Result<String>;
    fn read_name(&mut self) -> io::Result<FName>;
}

pub trait FStringWriteExt {
    fn write_string(&mut self, str: &str) -> io::Result<()>;
    fn write_name(&mut self, name: &FName) -> io::Result<()>;
}

pub trait WriteBitsExt {
    fn write_bits(&mut self, bits: &[u8], len: usize) -> io::Result<()>;
}

pub trait ReadBitsExt {
    fn read_bits(&mut self, amount: usize) -> io::Result<Vec<u8>>;
}

pub trait CompressedReadExt: BitRead {
    fn read_compressed_int(&mut self, max_value: u32) -> io::Result<u32> {
        let mut mask = 1;
        let mut value = 0;
        while value + mask < max_value && mask != 0 {
            if self.read_bit()? {
                value |= mask;
            }
            mask <<= 1;
        }

        Ok(value)
    }
}

impl<T: BitRead> CompressedReadExt for T {}

pub trait CompressedWriteExt: BitWrite {
    fn write_compressed_int(&mut self, value: u32, max_value: u32) -> io::Result<()> {
        let mut mask = 1;
        let mut new_value = 0;
        while new_value + mask < max_value && mask != 0 {
            if value & mask != 0 {
                self.write_bit(true)?;
                new_value += mask;
            } else {
                self.write_bit(false)?;
            }
            mask <<= 1;
        }

        Ok(())
    }
}

impl<T: BitWrite> CompressedWriteExt for T {}

pub trait ReadPrimitivesExt {
    fn read_u8(&mut self) -> io::Result<u8>;
    fn read_u16(&mut self) -> io::Result<u16>;
    fn read_u24(&mut self) -> io::Result<u32>;
    fn read_u32(&mut self) -> io::Result<u32>;
    fn read_u64(&mut self) -> io::Result<u64>;
    fn read_f32(&mut self) -> io::Result<f32>;
    fn read_f64(&mut self) -> io::Result<f64>;
    fn read_vector(&mut self) -> io::Result<FVector3d>;
}

pub trait WritePrimitivesExt {
    fn write_bool(&mut self, value: bool) -> io::Result<()>;
    fn write_u8(&mut self, value: u8) -> io::Result<()>;
    fn write_u16(&mut self, value: u16) -> io::Result<()>;
    fn write_u24(&mut self, value: u32) -> io::Result<()>;
    fn write_u32(&mut self, value: u32) -> io::Result<()>;
    fn write_u64(&mut self, value: u64) -> io::Result<()>;
    fn write_f32(&mut self, value: f32) -> io::Result<()>;
    fn write_f64(&mut self, value: f64) -> io::Result<()>;
    fn write_vector(&mut self, value: &FVector3d) -> io::Result<()>;

    fn write_i8(&mut self, value: i8) -> io::Result<()> {
        self.write_u8(value as u8)
    }

    fn write_i16(&mut self, value: i16) -> io::Result<()> {
        self.write_u16(value as u16)
    }

    fn write_i32(&mut self, value: i32) -> io::Result<()> {
        self.write_u32(value as u32)
    }

    fn write_i64(&mut self, value: i64) -> io::Result<()> {
        self.write_u64(value as u64)
    }
}

impl<T: BitRead> ReadBitsExt for T {
    fn read_bits(&mut self, amount: usize) -> io::Result<Vec<u8>> {
        let bytes = amount.div_ceil(8);
        let mut buffer = vec![0u8; bytes];
        self.read_bytes(&mut buffer[..(amount / 8)])?;

        if amount % 8 != 0 {
            for i in 0..(amount % 8) {
                if self.read_bit()? {
                    buffer[bytes - 1] |= 1 << i;
                }
            }
        }

        Ok(buffer)
    }
}

impl<T: BitWrite> WriteBitsExt for T {
    fn write_bits(&mut self, bits: &[u8], len: usize) -> io::Result<()> {
        self.write_bytes(&bits[..(len / 8)])?;
        for i in 0..(len % 8) {
            self.write_bit((bits[bits.len() - 1] >> i) & 1 != 0)?;
        }

        Ok(())
    }
}

impl<T: BitWrite> FStringWriteExt for T {
    fn write_string(&mut self, str: &str) -> io::Result<()> {
        self.write(32, 1 + str.len() as i32)?;
        self.write_bytes(str.as_bytes())?;
        self.write(8, 0)
    }

    fn write_name(&mut self, name: &FName) -> io::Result<()> {
        match name {
            FName::Hardcoded(index) => {
                self.write_bit(true)?;
                self.write_packed_int(*index)
            }
            FName::Custom(s) => {
                self.write_bit(false)?;
                self.write_string(s)?;
                self.write(32, 0)
            }
        }
    }
}

impl<T: BitRead> FStringReadExt for T {
    fn read_string(&mut self) -> io::Result<String> {
        let mut save_num = self.read::<u32>(32)? as i32;
        let load_ucs2_char = save_num < 0;
        if load_ucs2_char {
            save_num = -save_num;
        }

        if load_ucs2_char {
            let mut bytes = vec![0u8; (save_num * 2) as usize];
            self.read_bytes(&mut bytes)?;
            let words = (0..(save_num as usize))
                .map(|i| u16::from_be_bytes([bytes[2 * i], bytes[1 + 2 * i]]));
            Ok(std::char::decode_utf16(words)
                .collect::<Result<String, _>>()
                .unwrap())
        } else {
            let mut bytes = vec![0u8; save_num as usize];
            self.read_bytes(&mut bytes)?;
            Ok(
                String::from_utf8_lossy(&bytes[..(save_num as usize).saturating_sub(1)])
                    .to_string(),
            )
        }
    }

    fn read_name(&mut self) -> io::Result<FName> {
        if self.read_bit()? {
            Ok(FName::Hardcoded(self.read_packed_int()?))
        } else {
            let str = self.read_string()?;
            let _in_number: u32 = self.read(32)?;
            Ok(FName::Custom(str))
        }
    }
}

impl<T: BitWrite> PackedBitWriteExt for T {
    fn write_packed_int(&mut self, mut value: u32) -> io::Result<()> {
        loop {
            let next_val = value >> 7;
            self.write_bit(next_val != 0)?;
            self.write(7, value & 0x7F)?;
            value = next_val;
            if next_val == 0 {
                break Ok(());
            }
        }
    }
}

pub fn packed_int_size_in_bits(mut value: u32) -> usize {
    let mut bits = 8;
    loop {
        value >>= 7;
        if value != 0 {
            bits += 8;
        } else {
            break bits;
        }
    }
}

impl<T: BitRead> PackedBitReadExt for T {
    fn read_packed_int(&mut self) -> io::Result<u32> {
        let mut shift_count = 0;
        let mut result = 0u32;

        loop {
            let next_byte_indicator = self.read_bit()?;
            let byte: u32 = self.read(7)?;
            result |= byte << shift_count;
            shift_count += 7;

            if !next_byte_indicator || shift_count > 32 {
                break Ok(result);
            }
        }
    }

    fn read_packed_u16(&mut self) -> io::Result<u16> {
        let byte_count_to_read = if self.read_bit()? { 2 } else { 1 };
        self.read(byte_count_to_read * 8)
    }
}

impl<T: BitWrite> WritePrimitivesExt for T {
    fn write_u8(&mut self, value: u8) -> io::Result<()> {
        self.write(8, value)
    }

    fn write_u16(&mut self, value: u16) -> io::Result<()> {
        self.write(16, value)
    }

    fn write_u24(&mut self, value: u32) -> io::Result<()> {
        self.write(24, value)
    }

    fn write_u32(&mut self, value: u32) -> io::Result<()> {
        self.write(32, value)
    }

    fn write_u64(&mut self, value: u64) -> io::Result<()> {
        self.write(64, value)
    }

    fn write_f32(&mut self, value: f32) -> io::Result<()> {
        self.write(32, value.to_bits())
    }

    fn write_f64(&mut self, value: f64) -> io::Result<()> {
        self.write(64, value.to_bits())
    }

    fn write_vector(&mut self, value: &FVector3d) -> io::Result<()> {
        self.write(64, value.x.to_bits())?;
        self.write(64, value.y.to_bits())?;
        self.write(64, value.z.to_bits())
    }

    fn write_bool(&mut self, value: bool) -> io::Result<()> {
        self.write_bit(value)
    }
}

impl<T: BitRead> ReadPrimitivesExt for T {
    fn read_u8(&mut self) -> io::Result<u8> {
        self.read(8)
    }

    fn read_u16(&mut self) -> io::Result<u16> {
        self.read(16)
    }

    fn read_u24(&mut self) -> io::Result<u32> {
        self.read(24)
    }

    fn read_u32(&mut self) -> io::Result<u32> {
        self.read(32)
    }

    fn read_u64(&mut self) -> io::Result<u64> {
        self.read(64)
    }

    fn read_f32(&mut self) -> io::Result<f32> {
        Ok(f32::from_bits(self.read(32)?))
    }

    fn read_f64(&mut self) -> io::Result<f64> {
        Ok(f64::from_bits(self.read(64)?))
    }

    fn read_vector(&mut self) -> io::Result<FVector3d> {
        Ok(FVector3d {
            x: f64::from_bits(self.read(64)?),
            y: f64::from_bits(self.read(64)?),
            z: f64::from_bits(self.read(64)?),
        })
    }
}
