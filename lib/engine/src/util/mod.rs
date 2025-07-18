mod bit_stream_extensions;
pub mod quantized;
pub mod serialization;

use std::io::Cursor;

pub use bit_stream_extensions::*;
use bitstream_io::{BitReader, BitWriter, LittleEndian};

pub type OutBitWriter<'buf> = BitWriter<&'buf mut Vec<u8>, LittleEndian>;
pub type InBitReader<'buf> = BitReader<Cursor<&'buf [u8]>, LittleEndian>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FName {
    Hardcoded(u32),
    Custom(String),
}

#[derive(thiserror::Error, Debug)]
#[error("received packet with 0's in last byte of packet")]
pub struct UnterminatedBitsError;

#[inline]
pub const fn best_signed_difference(value: i32, reference: i32, max: i32) -> i32 {
    ((value - reference + max / 2) & (max - 1)) - max / 2
}

#[inline]
pub const fn make_relative(value: i32, reference: i32, max: i32) -> i32 {
    reference + best_signed_difference(value, reference, max)
}

pub fn get_bits_from_terminated_stream(data: &[u8]) -> Result<usize, UnterminatedBitsError> {
    match data {
        [] => Ok(0),
        [.., 0] => Err(UnterminatedBitsError),
        &[.., mut last_byte] => {
            let mut bit_size = (data.len() * 8) - 1;
            while last_byte & 0x80 == 0 {
                last_byte *= 2;
                bit_size -= 1;
            }

            Ok(bit_size)
        }
    }
}
