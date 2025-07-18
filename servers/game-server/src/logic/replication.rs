use bitstream_io::BitWrite;
use tracing::warn;

use fadia_engine::{
    FNetworkGUID,
    util::{
        self, CompressedReadExt, CompressedWriteExt, InBitReader, OutBitWriter, PackedBitReadExt,
        PackedBitWriteExt, ReadBitsExt, WriteBitsExt,
    },
};

use super::Object;

pub struct InRPC {
    pub rep_index: u32,
    pub data: Box<[u8]>,
}

pub fn serialize_object(
    writer: &mut OutBitWriter,
    guid: FNetworkGUID,
    object: &Object,
    is_actor: bool,
    full: bool,
) -> std::io::Result<()> {
    let has_rep_layout =
        !object.rep_layout.is_empty() && (full || object.rep_layout.rep_layout_changed());

    writer.write_bit(has_rep_layout)?;
    writer.write_bit(is_actor)?;

    if !is_actor {
        writer.write_packed_int(guid.0)?;
        writer.write_bit(true)?; // bStablyNamed
    }

    let mut replicator_payload = Vec::new();
    let mut out = OutBitWriter::new(&mut replicator_payload);

    if has_rep_layout {
        out.write_bit(false)?; // bDoChecksum
        object
            .rep_layout
            .serialize_layout_properties(&mut out, full)?;
    }

    for (rep_index, data) in object
        .rep_layout
        .serialize_custom_properties(full)
        .unwrap()
        .iter()
        .chain(object.queued_rpcs.iter())
    {
        let max_rep_index = object.rep_layout.max_rep_index();
        let size_in_bits = util::get_bits_from_terminated_stream(data).unwrap();
        out.write_compressed_int(*rep_index, max_rep_index + 1)?;
        out.write_packed_int(size_in_bits as u32)?;
        out.write_bits(data, size_in_bits)?;
    }

    out.write_bit(true)?; // termination bit
    out.byte_align()?;

    let payload_bits = util::get_bits_from_terminated_stream(&replicator_payload).unwrap();
    writer.write_packed_int(payload_bits as u32)?;
    writer.write_bits(&replicator_payload, payload_bits)?;

    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum ReceiveClientDataError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("max_rep_index is not defined for this object")]
    NoMaxRepIndex,
}

pub fn receive_client_data(
    object: &Object,
    r: &mut InBitReader,
    payload_size: usize,
) -> Result<Vec<InRPC>, ReceiveClientDataError> {
    // We're not reading RepLayout and Custom Properties here, since server shouldn't receive them.
    // TODO: sanity checks for if client sent a property like that?
    // this shouldn't happen, in the worst case we'll get a warning log, but just in case

    // Return early if we can't receive RPC for this Object
    if object.rep_layout.max_rep_index() == 1 {
        warn!("max_rep_index is not defined for object, skipping");
        return Err(ReceiveClientDataError::NoMaxRepIndex);
    }

    let mut output = Vec::new();
    let begin_offset = r.position_in_bits().unwrap() as usize;

    while (r.position_in_bits().unwrap() as usize) < begin_offset + payload_size {
        let rep_index = r.read_compressed_int(object.rep_layout.max_rep_index() + 1)?;
        let size_in_bits = r.read_packed_int()? as usize;
        let data = r.read_bits(size_in_bits)?.into_boxed_slice();

        output.push(InRPC {
            rep_index,
            data,
        });
    }

    Ok(output)
}
