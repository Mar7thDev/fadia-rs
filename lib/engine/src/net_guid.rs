use std::collections::HashMap;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct FNetworkGUID(pub u32);

impl FNetworkGUID {
    pub const INVALID: Self = Self(0);

    pub fn create_from_index(net_index: u32, is_static: bool) -> Self {
        Self((net_index << 1) | if is_static { 1 } else { 0 })
    }

    pub fn is_dynamic(&self) -> bool {
        self.0 > 0 && (self.0 & 1) == 0
    }

    pub fn is_valid(&self) -> bool {
        self.0 > 0
    }

    pub fn is_static(&self) -> bool {
        self.0 & 1 == 1
    }

    pub fn is_default(&self) -> bool {
        self.0 == 1
    }
}

impl Default for FNetworkGUID {
    fn default() -> Self {
        Self::INVALID
    }
}

pub struct NetGUIDCacheObject {
    pub path_name: &'static str,
    pub no_load: bool,
}

pub struct NetGUIDCache {
    map_name: &'static str,
    net_guid_index: [u32; 2],
    cache_objects: HashMap<FNetworkGUID, NetGUIDCacheObject>,
}

impl NetGUIDCache {
    const NET_GUID_INDEX_STATIC: usize = 0;
    const NET_GUID_INDEX_DYNAMIC: usize = 1;

    pub fn new(map_path: &'static str) -> Self {
        Self {
            map_name: map_path.split('/').next_back().unwrap(),
            net_guid_index: [1; 2],
            cache_objects: HashMap::new(),
        }
    }

    pub fn get(&self, guid: FNetworkGUID) -> Option<&NetGUIDCacheObject> {
        self.cache_objects.get(&guid)
    }

    pub fn assign_new_net_guid_for_dynamic_object(
        &mut self,
        dynamic_object_name: Option<&'static str>,
    ) -> FNetworkGUID {
        self.net_guid_index[Self::NET_GUID_INDEX_DYNAMIC] += 1;
        let new_guid = FNetworkGUID::create_from_index(
            self.net_guid_index[Self::NET_GUID_INDEX_DYNAMIC],
            false,
        );

        self.cache_objects.insert(
            new_guid,
            NetGUIDCacheObject {
                path_name: dynamic_object_name.unwrap_or_default(),
                no_load: true, // all dynamic objects are "no_load"
            },
        );

        new_guid
    }

    pub fn assign_new_net_guid_from_path(&mut self, path_name: &'static str) -> FNetworkGUID {
        self.net_guid_index[Self::NET_GUID_INDEX_STATIC] += 1;
        let new_guid =
            FNetworkGUID::create_from_index(self.net_guid_index[Self::NET_GUID_INDEX_STATIC], true);

        self.cache_objects.insert(
            new_guid,
            NetGUIDCacheObject {
                path_name,
                no_load: !self.can_client_load_object_internal(path_name, false),
            },
        );

        new_guid
    }

    pub fn get_or_assign_net_guid_for_static(&mut self, path_name: &'static str) -> FNetworkGUID {
        if let Some(existing_guid) = self.find_static_guid_by_path_name(path_name) {
            existing_guid
        } else {
            self.assign_new_net_guid_from_path(path_name)
        }
    }

    pub fn find_static_guid_by_path_name(&self, path_name: &str) -> Option<FNetworkGUID> {
        self.cache_objects
            .iter()
            .find(|(guid, cache)| guid.is_static() && cache.path_name == path_name)
            .map(|(&guid, _)| guid)
    }

    pub fn get_path_name_by_guid(&self, guid: FNetworkGUID) -> &'static str {
        self.cache_objects
            .get(&guid)
            .map(|object| object.path_name)
            .unwrap_or_default()
    }

    fn can_client_load_object_internal(&self, path: &str, is_dynamic: bool) -> bool {
        // PackageMapClient can't load maps, we must wait for the client to load the map when ready
        // These references are special references, where the reference and all child references resolve once the map has been loaded

        !is_dynamic && !self.contains_map(path)
    }

    fn contains_map(&self, path: &str) -> bool {
        path.contains(self.map_name)
    }
}
