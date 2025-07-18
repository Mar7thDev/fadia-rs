use std::io::{self, Cursor};

use crate::package_map::FNetFieldExportGroup;
use crate::util::{
    self, FName, FStringReadExt, FStringWriteExt, PackedBitReadExt, PackedBitWriteExt,
    get_bits_from_terminated_stream,
};
use bitstream_io::{BitRead, BitWrite, BitWriter, LittleEndian};

#[derive(Debug, Default, Clone)]
pub struct Bunch {
    pub control: bool,
    pub open: bool,
    pub close: bool,
    pub close_reason: u32,
    pub is_replication_paused: bool,
    pub reliable: bool,
    pub ch_index: u32,
    pub has_package_map_exports: bool,
    pub has_must_be_mapped_guids: bool,
    pub partial: bool,
    pub ch_sequence: u16,
    pub partial_initial: bool,
    pub partial_final: bool,
    pub ch_name: Option<FName>,
    pub bunch_data_bits: usize,
    pub network_exports_data: Option<(Vec<u8>, u32)>, // data, size_in_bits
}

impl Bunch {
    pub const MAX_DATA_BITS: usize = 7616;

    pub fn encode<W: BitWrite>(&self, w: &mut W) -> io::Result<()> {
        w.write_bit(self.control)?;
        if self.control {
            w.write_bit(self.open)?;
            w.write_bit(self.close)?;

            if self.close {
                w.write(4, self.close_reason)?;
            }
        }

        w.write_bit(self.is_replication_paused)?;
        w.write_bit(self.reliable)?;
        w.write_packed_int(self.ch_index)?;
        w.write_bit(self.has_package_map_exports)?;
        w.write_bit(self.has_must_be_mapped_guids)?;
        w.write_bit(self.partial)?;

        if self.reliable {
            w.write(10, self.ch_sequence)?;
        }

        if self.partial {
            w.write_bit(self.partial_initial)?;
            w.write_bit(false)?; // bPartialCustomExportsFinal
            w.write_bit(self.partial_final)?;
        }

        if self.reliable || self.open {
            w.write_name(self.ch_name.as_ref().unwrap())?;
        }

        w.write(13, self.bunch_data_bits as u16).unwrap();
        Ok(())
    }

    pub fn decode<R: BitRead>(r: &mut R, packet_id: u16) -> io::Result<Bunch> {
        let control = r.read_bit()?;
        let (open, close) = match control {
            true => (r.read_bit()?, r.read_bit()?),
            false => (false, false),
        };

        let close_reason = match close {
            true => r.read(4)?,
            false => 0, // EChannelCloseReason::Destroyed
        };

        let is_replication_paused = r.read_bit()?;
        let reliable = r.read_bit()?;
        let ch_index = r.read_packed_int()?;
        let has_package_map_exports = r.read_bit()?;
        let has_must_be_mapped_guids = r.read_bit()?;
        let partial = r.read_bit()?;

        let ch_sequence: u16 = if reliable {
            r.read(10)?
        } else if partial {
            packet_id
        } else {
            0
        };

        let (partial_initial, partial_custom_exports_final, partial_final) = match partial {
            true => (r.read_bit()?, r.read_bit()?, r.read_bit()?),
            false => (false, false, false),
        };

        assert!(
            !partial_custom_exports_final,
            "partial_custom_exports: not implemented"
        );

        let ch_name = match reliable || open {
            true => Some(r.read_name()?),
            false => None,
        };

        let bunch_data_bits: u32 = r.read(13)?;

        Ok(Bunch {
            control,
            open,
            close,
            close_reason,
            is_replication_paused,
            reliable,
            ch_index,
            has_package_map_exports,
            has_must_be_mapped_guids,
            partial,
            ch_sequence,
            partial_initial,
            partial_final,
            ch_name,
            bunch_data_bits: bunch_data_bits as usize,
            network_exports_data: None,
        })
    }

    pub fn refresh_data_bits_num(&mut self, data: &[u8]) {
        let mut num = 0;
        self.network_exports_data
            .as_ref()
            .inspect(|(_, size)| num += size);
        num += get_bits_from_terminated_stream(data).unwrap() as u32;
        self.bunch_data_bits = num as usize;
    }

    pub fn add_network_exports(&mut self, exports: &FNetFieldExportGroup) {
        self.has_package_map_exports = true;
        let mut buf = Vec::new();
        let mut w = BitWriter::endian(Cursor::new(&mut buf), LittleEndian);
        exports
            .export(&mut w)
            .expect("FNetFieldExportGroup::export failed!"); // shouldn't happen

        w.write_bit(true).unwrap(); // termination bit
        w.byte_align().unwrap();
        let bit_size = util::get_bits_from_terminated_stream(&buf).unwrap();

        self.network_exports_data = Some((buf, bit_size as u32));
    }
}
