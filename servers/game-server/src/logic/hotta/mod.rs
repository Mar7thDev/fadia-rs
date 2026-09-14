use std::io;

use bitstream_io::{BitRead, BitWrite};
use fadia_engine::{
    FNetworkGUID,
    util::{
        FName, FStringReadExt, FStringWriteExt, InBitReader, OutBitWriter, PackedBitReadExt,
        PackedBitWriteExt, ReadBitsExt, ReadPrimitivesExt, WriteBitsExt, WritePrimitivesExt,
    },
};

use super::rpc::RpcArgument;

pub mod encoding;
mod generated_rpc_indices;
pub(crate) mod initialization;
pub mod player_state;

pub trait HottaReplicatedObject {
    fn replicate(&self, owner: FNetworkGUID) -> HottaReplicatedObjectPropertyContainer;
}

pub trait HottaReplicatedProperty {
    fn replicate(&self) -> Box<[u8]> {
        let mut data = Vec::new();
        let mut writer = OutBitWriter::new(&mut data);
        self.replicate_impl(&mut writer).unwrap();
        writer.byte_align().unwrap();
        data.into_boxed_slice()
    }

    fn replicate_impl(&self, w: &mut OutBitWriter) -> io::Result<()>;
}

pub struct HottaReplicatedObjectPropertyContainer {
    pub owner: FNetworkGUID,
    pub properties: Vec<HottaReplicatedObjectProperty>,
}

pub struct HottaReplicatedObjectProperty {
    pub name: FName,
    pub datas: Box<[u8]>,
}

impl RpcArgument for HottaReplicatedObjectPropertyContainer {
    fn serialize(&self, w: &mut OutBitWriter) -> io::Result<()> {
        // RPC dynamic-array length, then HTReplicatedObjectPropertyContainer::NetSerialize.
        // 1.3.10: one bit selects a character NetID; the following bool uses FArchive's u32.
        w.write_u16(1)?;
        w.write_bit(false)?;
        w.write_u32(0)?; // bUseCharacterForNetID: serialize the Owner UObject instead.
        w.write_packed_int(self.owner.0)?;
        w.write_u32(self.properties.len() as u32)?;
        for property in self.properties.iter() {
            w.write_name(&property.name)?;
            w.write_u32(property.datas.len() as u32)?;
            w.write_bits(&property.datas, property.datas.len() * 8)?;
            // HottaReplicatedObjectProperty::bCompressed is an archive bool (32 bits).
            w.write_u32(0)?;
        }

        Ok(())
    }

    fn deserialize(r: &mut InBitReader) -> io::Result<Self> {
        if r.read_u16()? != 1 || r.read_bit()? || r.read_u32()? != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "expected one property container addressed by Owner",
            ));
        }

        Ok(Self {
            owner: FNetworkGUID(r.read_packed_int()?),
            properties: (0..r.read_u32()?)
                .map(|_| {
                    let property = HottaReplicatedObjectProperty {
                        name: r.read_name()?,
                        datas: {
                            let length = (r.read_u32()? as usize) * 8;
                            r.read_bits(length)?.into_boxed_slice()
                        },
                    };
                    if r.read_u32()? != 0 {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "compressed property data is unsupported",
                        ));
                    }
                    Ok(property)
                })
                .collect::<io::Result<_>>()?,
        })
    }
}
