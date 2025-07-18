use std::io;

use bitstream_io::{BitRead, BitWrite};

use crate::vector::FVector3d;

pub trait QuantizedReadExt {
    fn read_packed_vector(&mut self, scale_factor: i32) -> io::Result<FVector3d>;
}

pub trait QuantizedWriteExt {
    fn write_packed_vector(&mut self, value: &FVector3d, scale_factor: i32) -> io::Result<()>;
}

impl<T: BitRead> QuantizedReadExt for T {
    fn read_packed_vector(&mut self, scale_factor: i32) -> io::Result<FVector3d> {
        let component_bit_count_and_extra_info: u8 = self.read(7)?;
        let component_bit_count = component_bit_count_and_extra_info & 63;
        let extra_info = component_bit_count_and_extra_info >> 6;

        if component_bit_count > 0 {
            let x: u64 = self.read(component_bit_count as u32)?;
            let y: u64 = self.read(component_bit_count as u32)?;
            let z: u64 = self.read(component_bit_count as u32)?;
            let sign_bit = (1u64 << (component_bit_count as u64 - 1)) as i64;

            let x = (x as i64 ^ sign_bit).wrapping_sub(sign_bit);
            let y = (y as i64 ^ sign_bit).wrapping_sub(sign_bit);
            let z = (z as i64 ^ sign_bit).wrapping_sub(sign_bit);

            let (x, y, z) = if extra_info != 0 {
                (
                    x as f64 / scale_factor as f64,
                    y as f64 / scale_factor as f64,
                    z as f64 / scale_factor as f64,
                )
            } else {
                (x as f64, y as f64, z as f64)
            };

            Ok(FVector3d { x, y, z })
        } else {
            let received_scalar_type_size = if extra_info != 0 { 8 } else { 4 };

            if received_scalar_type_size == 8 {
                let x: u64 = self.read(8 * 8)?;
                let y: u64 = self.read(8 * 8)?;
                let z: u64 = self.read(8 * 8)?;

                Ok(FVector3d {
                    x: f64::from_bits(x),
                    y: f64::from_bits(y),
                    z: f64::from_bits(z),
                })
            } else {
                let x: u32 = self.read(8 * 4)?;
                let y: u32 = self.read(8 * 4)?;
                let z: u32 = self.read(8 * 4)?;

                Ok(FVector3d {
                    x: f32::from_bits(x) as f64,
                    y: f32::from_bits(y) as f64,
                    z: f32::from_bits(z) as f64,
                })
            }
        }
    }
}

impl<T: BitWrite> QuantizedWriteExt for T {
    fn write_packed_vector(&mut self, value: &FVector3d, scale_factor: i32) -> io::Result<()> {
        const MAX_EXPONENT_FOR_SCALING: u64 = 52;
        const MAX_VALUE_TO_SCALE: f64 = (1u64 << MAX_EXPONENT_FOR_SCALING) as f64;
        const MAX_EXPONENT_AFTER_SCALING: u64 = 62;
        const MAX_SCALED_VALUE: f64 = (1u64 << MAX_EXPONENT_AFTER_SCALING) as f64;

        let scaled_value = FVector3d {
            x: value.x * scale_factor as f64,
            y: value.y * scale_factor as f64,
            z: value.z * scale_factor as f64,
        };

        if scaled_value.get_abs_max() < MAX_SCALED_VALUE {
            let use_scaled_value = scaled_value.get_abs_min() < MAX_VALUE_TO_SCALE;

            let (x, y, z) = if use_scaled_value {
                (
                    round_float_to_int(scaled_value.x),
                    round_float_to_int(scaled_value.y),
                    round_float_to_int(scaled_value.z),
                )
            } else {
                (
                    round_float_to_int(value.x),
                    round_float_to_int(value.y),
                    round_float_to_int(value.z),
                )
            };

            let component_bit_count =
                get_bits_needed(x).max(get_bits_needed(y).max(get_bits_needed(z)));

            let component_bit_count_and_scale_info =
                if use_scaled_value { 1 << 6 } else { 0 } | component_bit_count;

            self.write(7, component_bit_count_and_scale_info as u8)?;
            self.write(component_bit_count, x)?;
            self.write(component_bit_count, y)?;
            self.write(component_bit_count, z)
        } else {
            let component_bit_count_and_scale_info = 1 << 6;
            self.write(7, component_bit_count_and_scale_info as u8)?;
            self.write(64, value.x.to_bits())?;
            self.write(64, value.y.to_bits())?;
            self.write(64, value.z.to_bits())
        }
    }
}

const fn round_float_to_int(f: f64) -> i64 {
    (f + f.signum() * 0.5) as i64
}

const fn get_bits_needed(value: i64) -> u32 {
    let value = (value ^ (value >> 63)) as u64;
    65 - value.leading_zeros()
}
