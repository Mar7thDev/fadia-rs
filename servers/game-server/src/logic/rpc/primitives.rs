use fadia_engine::{
    FNetworkGUID,
    util::{FStringReadExt, FStringWriteExt, PackedBitReadExt, PackedBitWriteExt},
};

use super::RpcArgument;

impl RpcArgument for FNetworkGUID {
    fn serialize(&self, w: &mut fadia_engine::util::OutBitWriter) -> std::io::Result<()> {
        w.write_packed_int(self.0)?;
        Ok(())
    }

    fn deserialize(r: &mut fadia_engine::util::InBitReader) -> std::io::Result<Self> {
        Ok(Self(r.read_packed_int()?))
    }
}

impl RpcArgument for String {
    fn serialize(&self, w: &mut fadia_engine::util::OutBitWriter) -> std::io::Result<()> {
        w.write_string(self)
    }

    fn deserialize(r: &mut fadia_engine::util::InBitReader) -> std::io::Result<Self> {
        r.read_string()
    }
}
