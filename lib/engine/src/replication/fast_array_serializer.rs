use std::collections::HashSet;

use bitstream_io::BitWrite;

use crate::util::{OutBitWriter, WritePrimitivesExt};

use super::{RepLayout, property::ReplicatedProperty};

#[derive(Debug)]
pub struct FastArraySerializer<T> {
    array_replication_key: u32,
    base_replication_key: u32,
    deleted_ids: HashSet<u32>,
    changed_ids: HashSet<u32>,
    items: Vec<FastArraySerializerItem<T>>,
}

#[derive(Debug)]
struct FastArraySerializerItem<T> {
    element_id: u32,
    item: T,
}

impl<T: RepLayout> FastArraySerializer<T> {
    pub fn push(&mut self, item: T) {
        self.array_replication_key += 1;
        self.changed_ids.insert(self.array_replication_key);
        self.items.push(FastArraySerializerItem {
            element_id: self.array_replication_key,
            item,
        });
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        self.items.iter().map(|item| (item.element_id, &item.item))
    }

    pub fn get(&self, element_id: u32) -> Option<&T> {
        self.items
            .iter()
            .find(|item| item.element_id == element_id)
            .map(|item| &item.item)
    }

    pub fn get_mut(&mut self, element_id: u32) -> Option<&mut T> {
        self.items
            .iter_mut()
            .find(|item| item.element_id == element_id)
            .map(|item| {
                self.changed_ids.insert(element_id);
                &mut item.item
            })
    }

    pub fn remove(&mut self, element_id: u32) -> Option<T> {
        if let Some(item) = self
            .items
            .iter()
            .enumerate()
            .find_map(|(index, item)| (item.element_id == element_id).then_some(index))
            .map(|index| self.items.remove(index))
        {
            self.deleted_ids.insert(item.element_id);
            Some(item.item)
        } else {
            None
        }
    }
}

impl<T: RepLayout> ReplicatedProperty for FastArraySerializer<T> {
    fn is_changed(&self) -> bool {
        !self.changed_ids.is_empty() || !self.deleted_ids.is_empty()
    }

    fn acknowledge_changes(&mut self) {
        self.changed_ids.clear();
        self.deleted_ids.clear();
    }

    fn serialize(&self, w: &mut OutBitWriter) -> std::io::Result<()> {
        let anything_changed = self.is_changed();
        w.write_bit(anything_changed)?;

        if anything_changed {
            w.write_u32(self.array_replication_key)?;
            w.write_u32(self.base_replication_key)?;
            w.write_u32(self.deleted_ids.len() as u32)?;
            w.write_u32(self.changed_ids.len() as u32)?;

            for &id in self.deleted_ids.iter() {
                w.write_u32(id)?;
            }

            for item in self.items.iter() {
                if self.changed_ids.contains(&item.element_id) {
                    w.write_u32(item.element_id)?;
                    w.write_bit(true)?; // bAnythingChanged

                    item.item.serialize_layout_properties(w, true)?;
                }
            }
        }

        Ok(())
    }
}

impl<T> Default for FastArraySerializer<T> {
    fn default() -> Self {
        Self {
            array_replication_key: Default::default(),
            base_replication_key: Default::default(),
            deleted_ids: Default::default(),
            changed_ids: Default::default(),
            items: Default::default(),
        }
    }
}
