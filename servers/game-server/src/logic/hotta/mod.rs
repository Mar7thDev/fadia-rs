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

pub mod player_state;
pub mod encoding;

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
        w.write_u32(1)?; // ??
        w.write(17, 0)?; // ??
        w.write_packed_int(self.owner.0)?;
        w.write_u32(self.properties.len() as u32)?;
        for property in self.properties.iter() {
            w.write_name(&property.name)?;
            w.write_u32(property.datas.len() as u32)?;
            w.write_bits(&property.datas, property.datas.len() * 8)?;
        }

        Ok(())
    }

    fn deserialize(r: &mut InBitReader) -> io::Result<Self> {
        let _ = r.read_u32()?;
        let _ = r.read::<u32>(17)?;

        Ok(Self {
            owner: FNetworkGUID(r.read_packed_int()?),
            properties: (0..r.read_u32()?)
                .map(|_| {
                    Ok(HottaReplicatedObjectProperty {
                        name: r.read_name()?,
                        datas: {
                            let length = (r.read_u32()? as usize) * 8;
                            r.read_bits(length)?.into_boxed_slice()
                        },
                    })
                })
                .collect::<io::Result<_>>()?,
        })
    }
}
