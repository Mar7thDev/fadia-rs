mod actor_channel;
mod control_channel;

pub use actor_channel::ActorChannel;
pub use control_channel::*;
use std::io::{self, Cursor};

use bitstream_io::{BitReader, LittleEndian};
use tracing::error;

use fadia_engine::net::Bunch;
use fadia_engine::util::{FName, InBitReader, OutBitWriter, ReadBitsExt, WriteBitsExt};

use super::world::World;

pub struct Channel<Impl> {
    pub index: u32,
    pub in_reliable: u16,
    pub out_reliable: u16,
    pub channel_name: FName,
    pub channel_impl: Impl,
    pending_partial_bunches: Vec<(Bunch, Box<[u8]>)>,
}

pub trait ChannelImpl {
    fn received_bunch(
        &mut self,
        world: &mut World,
        bunch: &Bunch,
        r: &mut InBitReader,
    ) -> io::Result<()>;

    fn remove_queued_bunches(&mut self) -> Vec<(Bunch, Box<[u8]>)> {
        Vec::new()
    }
}

pub const NAME_CONTROL_CHANNEL: FName = FName::Hardcoded(255);
pub const NAME_ACTOR_CHANNEL: FName = FName::Hardcoded(102);

impl<Impl: ChannelImpl> Channel<Impl> {
    pub fn new(
        channel_idx: u32,
        in_reliable: u16,
        out_reliable: u16,
        channel_name: FName,
        channel_impl: Impl,
    ) -> Self {
        Self {
            index: channel_idx,
            in_reliable,
            out_reliable,
            channel_name,
            channel_impl,
            pending_partial_bunches: Vec::new(),
        }
    }

    pub fn received_raw_bunch(
        &mut self,
        world: &mut World,
        bunch: Bunch,
        data: Box<[u8]>,
    ) -> io::Result<()> {
        if bunch.bunch_data_bits == 0 {
            return Ok(());
        }

        // UChannel::ReceivedRawBunch
        if bunch.has_package_map_exports {
            error!("received package map exports from client!");
            return Ok(());
        }

        if bunch.partial {
            let partial_final = bunch.partial_final;

            self.pending_partial_bunches.push((bunch, data));
            self.pending_partial_bunches
                .sort_by_key(|(bunch, _)| bunch.ch_sequence);

            if partial_final && let Some((start, end)) = self.next_partial_sequence() {
                let mut data = Vec::new();
                let mut w = OutBitWriter::new(&mut data);

                let in_bunch = self.pending_partial_bunches.drain(start..end).fold(
                    Bunch::default(),
                    |mut in_bunch, (bunch, data)| {
                        in_bunch.control |= bunch.control;
                        in_bunch.open |= bunch.open;
                        in_bunch.close |= bunch.close;
                        in_bunch.bunch_data_bits += bunch.bunch_data_bits;

                        let data = &data[..bunch.bunch_data_bits.div_ceil(8)];
                        w.write_bits(data, bunch.bunch_data_bits).unwrap();

                        in_bunch
                    },
                );

                let mut r = InBitReader::new(Cursor::new(&data));
                self.received_bunch(world, in_bunch, &mut r)?;
            }
        } else {
            // If it's not partial, receive it right away
            let mut r = InBitReader::new(Cursor::new(data.as_ref()));
            self.received_bunch(world, bunch, &mut r)?;
        }
        Ok(())
    }

    fn next_partial_sequence(&self) -> Option<(usize, usize)> {
        let mut start_index = None;
        let mut prev_seq = 0;

        for (i, (bunch, _)) in self.pending_partial_bunches.iter().enumerate() {
            if bunch.partial_initial {
                start_index = Some(i);
                prev_seq = bunch.ch_sequence;
                continue;
            }

            if prev_seq + 1 != bunch.ch_sequence {
                // Some bunch in the middle isn't received yet
                continue;
            }

            prev_seq = bunch.ch_sequence;
            if bunch.partial_final
                && let Some(start) = start_index
            {
                return Some((start, i));
            }
        }

        None
    }

    pub fn remove_queued_bunches(&mut self) -> Vec<(Bunch, Box<[u8]>)> {
        let mut bunches = self.channel_impl.remove_queued_bunches();

        for (bunch, _) in bunches.iter_mut() {
            bunch.ch_index = self.index;

            if bunch.reliable {
                bunch.ch_name = Some(self.channel_name.clone());
                bunch.ch_sequence = self.next_out_reliable();
            }
        }

        bunches
    }

    fn received_bunch(
        &mut self,
        world: &mut World,
        bunch: Bunch,
        r: &mut InBitReader,
    ) -> io::Result<()> {
        if bunch.ch_sequence > self.in_reliable || bunch.ch_sequence == 0 {
            self.in_reliable = bunch.ch_sequence;
        } else {
            return Ok(());
        }

        self.channel_impl.received_bunch(world, &bunch, r)
    }

    pub fn next_out_reliable(&mut self) -> u16 {
        self.out_reliable = (self.out_reliable + 1) & 1023;
        self.out_reliable
    }
}

fn split_bunch_data(buffer: Box<[u8]>, size_in_bits: usize) -> Vec<(Box<[u8]>, usize)> {
    if size_in_bits > Bunch::MAX_DATA_BITS {
        let mut chunks = Vec::new();
        let mut reader = BitReader::endian(Cursor::new(buffer.as_ref()), LittleEndian);

        let mut remaining = size_in_bits - reader.position_in_bits().unwrap() as usize;
        while remaining != 0 {
            let chunk_size = remaining.min(Bunch::MAX_DATA_BITS);
            let bits = reader.read_bits(chunk_size).unwrap();

            chunks.push((bits.into_boxed_slice(), chunk_size));
            remaining = size_in_bits - reader.position_in_bits().unwrap() as usize;
        }

        chunks
    } else {
        vec![(buffer, size_in_bits)]
    }
}
