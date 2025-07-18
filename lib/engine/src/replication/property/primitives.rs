use crate::{
    FNetworkGUID,
    util::{
        FName, FStringWriteExt, OutBitWriter, PackedBitWriteExt, WritePrimitivesExt,
        quantized::QuantizedWriteExt,
    },
    vector::FVector3d,
};
use bitstream_io::BitWrite;

use super::ReplicatedProperty;

macro_rules! impl_primitives {
    ($($type:ty),*) => {
        $(::paste::paste!(
            #[derive(Default, Debug, Clone)]
            pub struct [<Property $type:camel>] {
                value: $type,
                changed: bool,
            }

            impl [<Property $type:camel>] {
                pub fn new(value: $type) -> Self {
                    Self {
                        value,
                        changed: false,
                    }
                }

                pub fn set_value(&mut self, new_value: $type) {
                    if self.value != new_value {
                        self.value = new_value;
                        self.changed = true;
                    }
                }

                pub fn get(&self) -> $type {
                    self.value
                }
            }

            impl ReplicatedProperty for [<Property $type:camel>] {
                fn is_changed(&self) -> bool {
                    self.changed
                }

                fn acknowledge_changes(&mut self) {
                    self.changed = false;
                }

                fn serialize(&self, w: &mut OutBitWriter) -> std::io::Result<()> {
                    w.[<write_ $type:lower>](self.value)
                }
            }
        );)*
    };
}

impl_primitives! {
    u8, i8, u16, i16, u32, i32, u64, i64, f32, f64, bool
}

#[derive(Default, Debug, Clone)]
pub struct PropertyString {
    value: String,
    changed: bool,
}

#[derive(Debug)]
pub struct PropertyArray<T> {
    items: Vec<T>,
    is_changed: bool,
}

#[derive(Debug, Clone)]
pub struct PropertyName {
    value: FName,
    changed: bool,
}

#[derive(Debug, Default)]
pub struct PropertyObject {
    value: FNetworkGUID,
    changed: bool,
}

#[derive(Debug, Default)]
pub struct PropertyVector {
    value: FVector3d,
    changed: bool,
}

#[derive(Debug)]
pub struct PropertyVectorQuantize100 {
    value: FVector3d,
    changed: bool,
}

#[derive(Debug)]
pub struct GameplayTagContainer;

impl PropertyString {
    pub fn new(value: String) -> Self {
        Self {
            value,
            changed: false,
        }
    }

    pub fn set_value(&mut self, new_value: String) {
        if self.value != new_value {
            self.value = new_value;
            self.changed = true;
        }
    }

    pub fn get(&self) -> &str {
        &self.value
    }
}

impl ReplicatedProperty for PropertyString {
    fn is_changed(&self) -> bool {
        self.changed
    }

    fn acknowledge_changes(&mut self) {
        self.changed = false;
    }

    fn serialize(&self, w: &mut OutBitWriter) -> std::io::Result<()> {
        w.write_string(&self.value)
    }
}

impl PropertyName {
    pub fn new(value: FName) -> Self {
        Self {
            value,
            changed: false,
        }
    }

    pub fn set_value(&mut self, new_value: FName) {
        if self.value != new_value {
            self.value = new_value;
            self.changed = true;
        }
    }

    pub fn get(&self) -> &FName {
        &self.value
    }
}

impl ReplicatedProperty for PropertyName {
    fn is_changed(&self) -> bool {
        self.changed
    }

    fn acknowledge_changes(&mut self) {
        self.changed = false;
    }

    fn serialize(&self, w: &mut OutBitWriter) -> std::io::Result<()> {
        w.write_name(&self.value)
    }
}

impl PropertyObject {
    pub fn new(value: FNetworkGUID) -> Self {
        Self {
            value,
            changed: false,
        }
    }

    pub fn set_value(&mut self, new_value: FNetworkGUID) {
        if self.value != new_value {
            self.value = new_value;
            self.changed = true;
        }
    }

    pub fn get(&self) -> FNetworkGUID {
        self.value
    }
}

impl ReplicatedProperty for PropertyObject {
    fn is_changed(&self) -> bool {
        self.changed
    }

    fn acknowledge_changes(&mut self) {
        self.changed = false;
    }

    fn serialize(&self, w: &mut OutBitWriter) -> std::io::Result<()> {
        w.write_packed_int(self.value.0)
    }
}

impl PropertyVector {
    pub fn new(value: FVector3d) -> Self {
        Self {
            value,
            changed: false,
        }
    }

    pub fn set_value(&mut self, new_value: FVector3d) {
        if self.value != new_value {
            self.value = new_value;
            self.changed = true;
        }
    }

    pub fn get(&self) -> &FVector3d {
        &self.value
    }
}

impl ReplicatedProperty for PropertyVector {
    fn is_changed(&self) -> bool {
        self.changed
    }

    fn acknowledge_changes(&mut self) {
        self.changed = false;
    }

    fn serialize(&self, w: &mut OutBitWriter) -> std::io::Result<()> {
        w.write_vector(&self.value)
    }
}

impl PropertyVectorQuantize100 {
    pub fn new(value: FVector3d) -> Self {
        Self {
            value,
            changed: false,
        }
    }

    pub fn set_value(&mut self, new_value: FVector3d) {
        if self.value != new_value {
            self.value = new_value;
            self.changed = true;
        }
    }

    pub fn get(&self) -> &FVector3d {
        &self.value
    }
}

impl ReplicatedProperty for PropertyVectorQuantize100 {
    fn is_changed(&self) -> bool {
        self.changed
    }

    fn acknowledge_changes(&mut self) {
        self.changed = false;
    }

    fn serialize(&self, w: &mut OutBitWriter) -> std::io::Result<()> {
        w.write_packed_vector(&self.value, 100)
    }
}

impl ReplicatedProperty for GameplayTagContainer {
    fn is_changed(&self) -> bool {
        false
    }

    fn acknowledge_changes(&mut self) {}

    fn serialize(&self, w: &mut OutBitWriter) -> std::io::Result<()> {
        w.write_bit(true) // is_empty
    }
}

impl<T> PropertyArray<T> {
    pub fn push(&mut self, item: T) {
        self.is_changed = true;
        self.items.push(item);
    }

    pub fn extend(&mut self, items: impl IntoIterator<Item = T>) {
        self.is_changed = true;
        self.items.extend(items);
    }

    pub fn remove(&mut self, index: usize) -> T {
        self.items.remove(index)
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn clear(&mut self) {
        if !self.items.is_empty() {
            self.is_changed = true;
            self.items.clear();
        }
    }
}

impl<T: ReplicatedProperty> ReplicatedProperty for PropertyArray<T> {
    fn is_changed(&self) -> bool {
        self.is_changed
    }

    fn acknowledge_changes(&mut self) {
        self.is_changed = false;
    }

    fn serialize(&self, w: &mut OutBitWriter) -> std::io::Result<()> {
        w.write_u16(self.len() as u16)?;
        for (i, item) in self.items.iter().enumerate() {
            w.write_packed_int((i + 1) as u32)?;
            item.serialize(w)?;
        }
        w.write_packed_int(0) // array termination
    }
}

impl<T> Default for PropertyArray<T> {
    fn default() -> Self {
        Self {
            items: Default::default(),
            is_changed: Default::default(),
        }
    }
}
