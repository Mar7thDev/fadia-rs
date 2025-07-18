use fadia_codegen::{RepLayout, dummy_rpc_handler};
use fadia_engine::replication::property::PropertyObject;

use crate::logic::{actor::PropertyNetRole, ObjectLayout};

#[derive(Debug, RepLayout)]
#[dummy_rpc_handler]
pub struct GameplayDebugger {
    #[rep(handle = 5)]
    pub remote_role: PropertyNetRole,
    #[rep(handle = 14)]
    pub role: PropertyNetRole,
    #[rep(handle = 17)]
    pub player_state: PropertyObject,
}

impl ObjectLayout for GameplayDebugger {}
