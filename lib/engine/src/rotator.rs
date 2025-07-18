use std::io;

use bitstream_io::{BitRead, BitWrite};

#[derive(Default, Debug, Clone)]
pub struct FRotator {
    pitch: f64,
    yaw: f64,
    roll: f64,
}

impl FRotator {
    pub fn new(pitch: f64, yaw: f64, roll: f64) -> Self {
        Self { pitch, yaw, roll }
    }

    pub fn set(&mut self, pitch: f64, yaw: f64, roll: f64) {
        self.pitch = pitch;
        self.yaw = yaw;
        self.roll = roll;
    }

    pub fn should_serialize(&self) -> bool {
        !matches!(
            self,
            Self {
                pitch: -0.1..0.1,
                yaw: -0.1..0.1,
                roll: -0.1..0.1
            }
        )
    }

    pub fn net_serialize<W: BitWrite>(&self, w: &mut W) -> io::Result<()> {
        let short_pitch = compress_axis_to_short(self.pitch);
        let short_yaw = compress_axis_to_short(self.yaw);
        let short_roll = compress_axis_to_short(self.roll);

        let b = short_pitch != 0;
        w.write_bit(b)?;
        if b {
            w.write(16, short_pitch)?;
        }

        let b = short_yaw != 0;
        w.write_bit(b)?;
        if b {
            w.write(16, short_yaw)?;
        }

        let b = short_roll != 0;
        w.write_bit(b)?;
        if b {
            w.write(16, short_roll)?;
        }

        Ok(())
    }

    pub fn net_deserialize<R: BitRead>(r: &mut R) -> io::Result<Self> {
        let short_pitch = if r.read_bit()? { r.read(16)? } else { 0 };
        let short_yaw = if r.read_bit()? { r.read(16)? } else { 0 };
        let short_roll = if r.read_bit()? { r.read(16)? } else { 0 };

        Ok(Self {
            pitch: decompress_axis_from_short(short_pitch),
            yaw: decompress_axis_from_short(short_yaw),
            roll: decompress_axis_from_short(short_roll),
        })
    }
}

fn compress_axis_to_short(angle: f64) -> u16 {
    round_to_int(angle * 65536.0 / 360.0) as u16
}

fn decompress_axis_from_short(angle: u16) -> f64 {
    angle as f64 * 360.0 / 65536.0
}

fn round_to_int(value: f64) -> i64 {
    let value = value + 0.5;

    let i = value.trunc() as i64;
    if i as f64 > value { i - 1 } else { i }
}
