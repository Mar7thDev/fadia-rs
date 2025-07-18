use fadia_codegen::{RepLayout, dummy_rpc_handler};

use crate::logic::{ObjectLayout, actor::PropertyNetRole};

#[derive(Debug, RepLayout)]
#[dummy_rpc_handler]
pub struct GenericActor {
    #[rep(handle = 5)]
    pub remote_role: PropertyNetRole,
    #[rep(handle = 14)]
    pub role: PropertyNetRole,
}

impl ObjectLayout for GenericActor {}
