use std::{any::Any, io};

use crate::util::OutBitWriter;

pub trait RepLayout: Any {
    fn rep_layout_changed(&self) -> bool;
    fn custom_properties_changed(&self) -> bool;
    fn acknowledge_changes(&mut self);
    fn serialize_layout_properties(&self, w: &mut OutBitWriter, full: bool) -> io::Result<()>;
    fn serialize_custom_properties(&self, full: bool) -> io::Result<Vec<(u32, Box<[u8]>)>>;
    fn is_empty(&self) -> bool;
    fn max_rep_index(&self) -> u32;
}

#[derive(Debug)]
pub struct NullLayout;

impl RepLayout for NullLayout {
    fn rep_layout_changed(&self) -> bool {
        false
    }

    fn custom_properties_changed(&self) -> bool {
        false
    }

    fn is_empty(&self) -> bool {
        true
    }

    fn acknowledge_changes(&mut self) {}

    fn max_rep_index(&self) -> u32 {
        1
    }

    fn serialize_layout_properties(&self, _: &mut OutBitWriter, _: bool) -> std::io::Result<()> {
        Ok(())
    }

    fn serialize_custom_properties(&self, _: bool) -> std::io::Result<Vec<(u32, Box<[u8]>)>> {
        Ok(Vec::new())
    }
}
