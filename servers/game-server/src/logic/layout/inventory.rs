use std::io;

use fadia_codegen::{RepLayout, rpc_handlers};
use fadia_engine::util::{
    FName, FStringReadExt, FStringWriteExt, InBitReader, OutBitWriter, ReadPrimitivesExt,
    WritePrimitivesExt,
};

use crate::logic::{ObjectLayout, rpc::RpcArgument};

#[derive(Debug, RepLayout)]
#[max_rep_index(12)]
pub struct InventoryComponent {}

impl ObjectLayout for InventoryComponent {}

#[rpc_handlers]
impl InventoryComponent {
    #[rpc(6, client)]
    pub fn client_set_character_items(&self, items: CharacterItems) {}
}

/// UHTInventoryComponent::Client_SetCharacterItems uses TArray<FFormationItemInfo>.
/// These records are required in addition to the spawned EquippedPlayers actors.
pub struct CharacterItems(pub Vec<FormationItemInfo>);

pub struct FormationItemInfo {
    pub item_id: FName,
    pub slot: u32,
    pub serial: u32,
}

impl RpcArgument for CharacterItems {
    fn serialize(&self, w: &mut OutBitWriter) -> io::Result<()> {
        let count = u16::try_from(self.0.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many character items"))?;
        w.write_u16(count)?;
        for item in &self.0 {
            w.write_name(&item.item_id)?;
            w.write_u32(item.slot)?;
            w.write_u32(item.serial)?;
        }
        Ok(())
    }

    fn deserialize(r: &mut InBitReader) -> io::Result<Self> {
        let count = r.read_u16()?;
        let items = (0..count)
            .map(|_| {
                Ok(FormationItemInfo {
                    item_id: r.read_name()?,
                    slot: r.read_u32()?,
                    serial: r.read_u32()?,
                })
            })
            .collect::<io::Result<_>>()?;
        Ok(Self(items))
    }
}
