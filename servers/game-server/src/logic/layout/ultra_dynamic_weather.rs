use fadia_codegen::{RepLayout, dummy_rpc_handler};
use fadia_engine::replication::property::{PropertyF64, PropertyU32};

use crate::logic::{actor::PropertyNetRole, ObjectLayout};

#[derive(Debug, RepLayout)]
#[dummy_rpc_handler]
pub struct UltraDynamicWeather {
    #[rep(handle = 5)]
    pub remote_role: PropertyNetRole,
    #[rep(handle = 14)]
    pub role: PropertyNetRole,
    #[rep(handle = 17)]
    pub weater_index_replicate: PropertyU32,
    #[rep(handle = 18)]
    pub cloud_coverage: PropertyF64,
    #[rep(handle = 19)]
    pub fog: PropertyF64,
    #[rep(handle = 20)]
    pub wind_intensity: PropertyF64,
    #[rep(handle = 21)]
    pub rain: PropertyF64,
    #[rep(handle = 22)]
    pub snow: PropertyF64,
    #[rep(handle = 23)]
    pub thunder_lightning: PropertyF64,
    #[rep(handle = 26)]
    pub material_snow_coverage: PropertyF64,
    #[rep(handle = 29)]
    pub transition_state: PropertyU32,
    #[rep(handle = 31)]
    pub weather_speed: PropertyF64,
    #[rep(handle = 35)]
    pub replicated_material_snow: PropertyF64,
    #[rep(handle = 38)]
    pub season: PropertyF64,
}

impl ObjectLayout for UltraDynamicWeather {}
