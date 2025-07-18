use fadia_codegen::{RepLayout, dummy_rpc_handler};
use fadia_engine::{
    FNetworkGUID,
    replication::property::{PropertyArray, PropertyName},
    util::FName,
};

use crate::{
    logic::{
        ObjectLayout,
        actor::{NetRole, PropertyNetRole},
    },
    net::World,
};

#[derive(Debug, RepLayout)]
#[dummy_rpc_handler]
pub struct WorldDataLayers {
    #[rep(handle = 5)]
    pub remote_role: PropertyNetRole,
    #[rep(handle = 14)]
    pub role: PropertyNetRole,
    #[rep(handle = 17)]
    pub rep_active_data_layer_names: PropertyArray<PropertyName>,
    #[rep(handle = 18)]
    pub rep_loaded_data_layer_names: PropertyArray<PropertyName>,
    #[rep(handle = 19)]
    pub rep_effective_active_data_layer_names: PropertyArray<PropertyName>,
    #[rep(handle = 20)]
    pub rep_effective_loaded_data_layer_names: PropertyArray<PropertyName>,
}

const PARENT_CLASS_NAME: &str = "PersistentLevel";
const CLASS_NAME: &str = "WorldDataLayers";

const DEFAULT_DATA_LAYER_NAMES: &[&str] = &[
    "DataLayer_32E40D604085853606D65AB77404869F",
    "DataLayer_9C3063C44B65F891D62CB3A857FFC0E1",
    "DataLayer_44261C8249F74E7EDE70EA9FDEE9FC3B",
    "DataLayer_EBD9987940833C0A8E900595079BDCD1",
    "DataLayer_968B9D944A243F065C303186E44B578D",
    "DataLayer_5963168345EA946F9F5E9CB052933123",
    "DataLayer_6544E87E494B87F4B7A091AD30DBC102",
    "DataLayer_F99A219143708D8368AEE4B98AF5E78B",
];

const DEFAULT_LOADED_DATA_LAYER_NAMES: &[&str] = &[
    "DataLayer_9FBCFE1E49B3D2C6196E538050975D52"
];

impl WorldDataLayers {
    pub fn new(remote_role: NetRole, role: NetRole, world: &mut World) -> (FNetworkGUID, Self) {
        let guid = world.register_hierarchy_for_static_objects(&[PARENT_CLASS_NAME, CLASS_NAME]);

        let data_layers = DEFAULT_DATA_LAYER_NAMES
            .iter()
            .map(|name| PropertyName::new(FName::Custom(name.to_string())))
            .collect::<Vec<_>>();

        let loaded_data_layers = DEFAULT_LOADED_DATA_LAYER_NAMES
            .iter()
            .map(|name| PropertyName::new(FName::Custom(name.to_string())))
            .collect::<Vec<_>>();

        let mut rep_loaded_data_layer_names = PropertyArray::default();
        let mut rep_active_data_layer_names = PropertyArray::default();
        let mut rep_effective_loaded_data_layer_names = PropertyArray::default();
        let mut rep_effective_active_data_layer_names = PropertyArray::default();

        rep_loaded_data_layer_names.extend(loaded_data_layers.clone());
        rep_active_data_layer_names.extend(data_layers.clone());
        rep_effective_loaded_data_layer_names.extend(loaded_data_layers.clone());
        rep_effective_active_data_layer_names.extend(data_layers.clone());

        (
            guid,
            WorldDataLayers {
                remote_role: PropertyNetRole::new(remote_role),
                role: PropertyNetRole::new(role),
                rep_loaded_data_layer_names,
                rep_active_data_layer_names,
                rep_effective_active_data_layer_names,
                rep_effective_loaded_data_layer_names,
            },
        )
    }
}

impl ObjectLayout for WorldDataLayers {}
