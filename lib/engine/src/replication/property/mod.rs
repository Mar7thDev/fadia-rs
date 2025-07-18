mod primitives;

use std::io;

pub use primitives::*;

use crate::util::OutBitWriter;

pub trait ReplicatedProperty {
    fn is_changed(&self) -> bool;
    fn acknowledge_changes(&mut self);
    fn serialize(&self, w: &mut OutBitWriter) -> io::Result<()>;
}
