use fadia_engine::util::{FStringWriteExt, OutBitWriter, WritePrimitivesExt};

use super::HottaReplicatedProperty;

impl HottaReplicatedProperty for bool {
    fn replicate_impl(&self, w: &mut OutBitWriter) -> std::io::Result<()> {
        w.write_u8(*self as u8)
    }
}

impl HottaReplicatedProperty for u32 {
    fn replicate_impl(&self, w: &mut OutBitWriter) -> std::io::Result<()> {
        w.write_u32(*self)
    }
}

impl HottaReplicatedProperty for u64 {
    fn replicate_impl(&self, w: &mut OutBitWriter) -> std::io::Result<()> {
        w.write_u64(*self)
    }
}

impl HottaReplicatedProperty for String {
    fn replicate_impl(&self, w: &mut OutBitWriter) -> std::io::Result<()> {
        w.write_string(self)
    }
}

impl<T> HottaReplicatedProperty for Vec<T>
where
    T: HottaReplicatedProperty,
{
    fn replicate_impl(&self, w: &mut OutBitWriter) -> std::io::Result<()> {
        w.write_u16(self.len() as u16)?;
        for item in self.iter() {
            // call replicate_impl here right away, elements aren't length-prefixed
            item.replicate_impl(w)?;
        }

        Ok(())
    }
}

// Dummy array
impl HottaReplicatedProperty for Vec<()> {
    fn replicate_impl(&self, w: &mut OutBitWriter) -> std::io::Result<()> {
        w.write_u16(0)
    }
}
