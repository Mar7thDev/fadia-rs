use std::fmt;
use std::{collections::HashMap, io};

use bitstream_io::{BitRead, BitWrite};
use tracing::warn;

use super::FNetworkGUID;
use crate::util::{self, FStringReadExt, FStringWriteExt, PackedBitReadExt, PackedBitWriteExt};

#[derive(Default)]
pub struct UPackageMapClient {
    pub object_cache: FNetGUIDCache,
}

impl fmt::Display for UPackageMapClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (guid, obj) in self.object_cache.object_map.iter() {
            write!(f, "({guid:?}, {obj:?}),")?;
        }
        write!(f, "]")
    }
}

#[derive(Clone, Debug)]
pub struct FNetGUIDCacheObject {
    pub path_name: String,
    pub outer_guid: FNetworkGUID,
    pub export_flags: ExportFlags,
    pub network_checksum: u32,
    pub should_encode: bool,
}

#[derive(Default, Debug)]
pub struct FNetGUIDCache {
    pub object_map: HashMap<FNetworkGUID, FNetGUIDCacheObject>,
    pub is_exporting_net_guid_bunch: bool,
}

#[derive(Debug, Clone)]
pub struct FNetFieldExport {
    pub path: String, // FIXME: use Cow?
    pub outer_guid: FNetworkGUID,
    pub export_flags: ExportFlags,
    pub network_checksum: u32,
    pub should_encode: bool,
}

#[derive(Debug, Default)]
pub struct FNetFieldExportGroup {
    pub exported_fields: HashMap<FNetworkGUID, FNetFieldExport>,
}

impl FNetFieldExportGroup {
    pub fn export<W: BitWrite>(&self, w: &mut W) -> io::Result<()> {
        w.write_bit(false)?; // has_rep_layout_export
        w.write(
            32,
            self.exported_fields
                .iter()
                .filter(|f| f.1.should_encode)
                .count() as u32,
        )?;

        for (export_guid, exported_field) in self.exported_fields.iter() {
            if exported_field.should_encode {
                w.write_packed_int(export_guid.0)?;
                exported_field.export(export_guid, &self.exported_fields, w)?;
            }
        }

        Ok(())
    }

    pub fn encoded_size_in_bits(&self) -> usize {
        const HAS_REP_LAYOUT_FLAG_SIZE: usize = 1;
        const ENCODED_FIELD_COUNT_SIZE: usize = 32;

        HAS_REP_LAYOUT_FLAG_SIZE
            + ENCODED_FIELD_COUNT_SIZE
            + self
                .exported_fields
                .iter()
                .filter(|(_, field)| field.should_encode)
                .map(|(guid, field)| {
                    util::packed_int_size_in_bits(guid.0)
                        + field.encoded_size_in_bits(guid, &self.exported_fields)
                })
                .sum::<usize>()
    }

    // Split into multiple encoded groups if exceeds max_size
    pub fn split_to_fit(&mut self, max_size: usize) -> Option<Self> {
        if self
            .exported_fields
            .iter()
            .filter(|(_, field)| field.should_encode)
            .count()
            == 0
        {
            None
        } else if self.encoded_size_in_bits() <= max_size {
            Some(FNetFieldExportGroup {
                exported_fields: std::mem::take(&mut self.exported_fields),
            })
        } else {
            let mut new_group = FNetFieldExportGroup::default();

            // Copy all non-encoded exports
            self.exported_fields
                .iter()
                .filter(|(_, field)| !field.should_encode)
                .for_each(|(guid, field)| {
                    new_group.exported_fields.insert(*guid, field.clone());
                });

            while new_group.encoded_size_in_bits() < max_size
                && self.encoded_size_in_bits() > max_size
            {
                if let Some((guid, export)) = self.pop_next_encoded_export() {
                    new_group.exported_fields.insert(guid, export);
                } else {
                    break;
                }
            }

            Some(new_group)
        }
    }

    fn pop_next_encoded_export(&mut self) -> Option<(FNetworkGUID, FNetFieldExport)> {
        let guid = self
            .exported_fields
            .iter()
            .find(|(_, export)| export.should_encode)
            .map(|(guid, _)| *guid)?;

        Some((guid, self.exported_fields.remove(&guid).unwrap()))
    }
}

impl FNetFieldExport {
    pub fn export<W: BitWrite>(
        &self,
        guid: &FNetworkGUID,
        export_map: &HashMap<FNetworkGUID, FNetFieldExport>,
        w: &mut W,
    ) -> io::Result<()> {
        if !guid.is_valid() {
            warn!("exporting invalid guid!");
            return Ok(());
        }

        w.write(8, self.export_flags.0)?;
        if self.export_flags.has_path() {
            w.write_packed_int(self.outer_guid.0)?;
            if self.outer_guid.is_valid() {
                if let Some(export) = export_map.get(&self.outer_guid) {
                    export.export(&self.outer_guid, export_map, w)?;
                }
            }
            w.write_string(&self.path)?;
            if self.export_flags.has_network_checksum() {
                w.write(32, self.network_checksum)?;
            }
        }

        Ok(())
    }

    pub fn encoded_size_in_bits(
        &self,
        guid: &FNetworkGUID,
        export_map: &HashMap<FNetworkGUID, FNetFieldExport>,
    ) -> usize {
        const EXPORT_FLAGS_BITS: usize = 8;
        const NETWORK_CHECKSUM_BITS: usize = 32;
        const STRING_LENGTH_PREFIX_BITS: usize = 32;
        const STRING_NULL_TERMINATOR_BITS: usize = 8;

        if guid.is_valid() {
            EXPORT_FLAGS_BITS
                + if self.export_flags.has_path() {
                    util::packed_int_size_in_bits(self.outer_guid.0)
                        + (STRING_LENGTH_PREFIX_BITS
                            + STRING_NULL_TERMINATOR_BITS
                            + (self.path.len() * 8))
                        + if self.export_flags.has_network_checksum() {
                            NETWORK_CHECKSUM_BITS
                        } else {
                            0
                        }
                        + if self.outer_guid.is_valid() {
                            export_map
                                .get(&self.outer_guid)
                                .map(|field| {
                                    field.encoded_size_in_bits(&self.outer_guid, export_map)
                                })
                                .unwrap_or(0)
                        } else {
                            0
                        }
                } else {
                    0
                }
        } else {
            0
        }
    }
}

impl UPackageMapClient {
    pub fn receive_net_guid_bunch<R: BitRead>(&mut self, r: &mut R) -> io::Result<()> {
        let has_rep_layout_export = r.read_bit()?;
        assert!(
            !has_rep_layout_export,
            "rep_layout_export not implemented yet"
        );

        let num_guids_in_bunch: u32 = r.read(32)?;
        for _ in 0..num_guids_in_bunch {
            let _ = self.internal_load_object(r, false)?;
        }

        Ok(())
    }

    pub fn internal_load_object<R: BitRead>(
        &mut self,
        r: &mut R,
        is_recursive: bool,
    ) -> io::Result<(FNetworkGUID, Option<FNetGUIDCacheObject>)> {
        let guid = FNetworkGUID(r.read_packed_int().unwrap());
        if !guid.is_valid() {
            return Ok((guid, None));
        }

        //  let mut obj = match guid.is_valid() && !guid.is_default() {
        //      true => self
        //          .object_cache
        //          .get_object_from_net_guid(guid.clone())
        //          .cloned(),
        //      false => None,
        //  };

        let mut obj = None;
        if guid.is_default() || self.object_cache.is_exporting_net_guid_bunch {
            let export_flags = ExportFlags(r.read(8).unwrap());
            if export_flags.has_path() {
                let (outer_guid, _) = self.internal_load_object(r, true)?;
                let path_name = r.read_string()?;
                let network_checksum = match export_flags.has_network_checksum() {
                    true => r.read::<u32>(32)?,
                    false => 0,
                };

                let _is_package = guid.is_static() && !outer_guid.is_valid();
                if obj.is_some() || guid.is_default() {
                    return Ok((guid, obj));
                }

                let ignore_when_missing = export_flags.no_load();
                self.object_cache.register_net_guid_from_path_client(
                    guid,
                    path_name,
                    outer_guid,
                    export_flags,
                    network_checksum,
                    ignore_when_missing,
                    !is_recursive,
                );
                obj = self.object_cache.get_object_from_net_guid(guid).cloned();
            } else {
                self.object_cache.register_net_guid_from_path_client(
                    guid,
                    String::new(),
                    FNetworkGUID(0),
                    export_flags,
                    0,
                    false,
                    !is_recursive,
                );
            }
        }

        Ok((guid, obj))
    }
}

impl FNetGUIDCache {
    pub fn get(&self, guid: FNetworkGUID) -> Option<&FNetGUIDCacheObject> {
        self.object_map.get(&guid)
    }

    pub fn get_object_from_net_guid(&self, guid: FNetworkGUID) -> Option<&FNetGUIDCacheObject> {
        match self.object_map.get(&guid) {
            Some(obj) if obj.path_name.is_empty() => None,
            ret => ret,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_net_guid_from_path_client(
        &mut self,
        guid: FNetworkGUID,
        path_name: String,
        outer_guid: FNetworkGUID,
        export_flags: ExportFlags,
        network_checksum: u32,
        _ignore_when_missing: bool,
        should_encode: bool,
    ) {
        self.object_map.entry(guid).or_insert(FNetGUIDCacheObject {
            path_name,
            outer_guid,
            export_flags,
            network_checksum,
            should_encode,
        });
    }
}

#[derive(Debug, Default, Clone)]
pub struct ExportFlags(pub u8);

impl ExportFlags {
    const HAS_PATH_MASK: u8 = 1;
    const NO_LOAD_MASK: u8 = 2;
    const HAS_NETWORK_CHECKSUM_MASK: u8 = 4;

    pub fn set_has_path(self) -> Self {
        ExportFlags(self.0 | Self::HAS_PATH_MASK)
    }

    pub fn set_no_load(self) -> Self {
        ExportFlags(self.0 | Self::NO_LOAD_MASK)
    }

    pub fn has_path(&self) -> bool {
        self.0 & Self::HAS_PATH_MASK != 0
    }

    pub fn no_load(&self) -> bool {
        self.0 & Self::NO_LOAD_MASK != 0
    }

    pub fn has_network_checksum(&self) -> bool {
        self.0 & Self::HAS_NETWORK_CHECKSUM_MASK != 0
    }
}
