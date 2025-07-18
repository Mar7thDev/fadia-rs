use fadia_codegen::{RepLayout, dummy_rpc_handler};
use fadia_engine::{
    FNetworkGUID,
    replication::property::{PropertyObject, PropertyVectorQuantize100},
    vector::FVector3d,
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
pub struct WeaponBase {
    #[rep(handle = 5)]
    pub remote_role: PropertyNetRole,
    #[rep(handle = 6)]
    pub attach_parent: PropertyObject,
    #[rep(handle = 8)]
    pub relative_scale_3d: PropertyVectorQuantize100,
    #[rep(handle = 11)]
    pub attach_component: PropertyObject,
    #[rep(handle = 13)]
    pub owner: PropertyObject,
    #[rep(handle = 14)]
    pub role: PropertyNetRole,
    #[rep(handle = 18)]
    pub owner_character: PropertyObject,
}

impl WeaponBase {
    pub fn new(
        remote_role: NetRole,
        role: NetRole,
        world: &mut World,
    ) -> (
        FNetworkGUID,
        Self,
        Vec<(FNetworkGUID, Box<dyn ObjectLayout>)>,
    ) {
        let guid = world
            .net_guid_cache
            .assign_new_net_guid_for_dynamic_object(None);

        (
            guid,
            WeaponBase {
                remote_role: PropertyNetRole::new(remote_role),
                role: PropertyNetRole::new(role),
                attach_parent: PropertyObject::default(),
                relative_scale_3d: PropertyVectorQuantize100::new(FVector3d::new(1.0, 1.0, 1.0)),
                attach_component: PropertyObject::default(),
                owner: PropertyObject::default(),
                owner_character: PropertyObject::default(),
            },
            Vec::new(),
        )
    }
}

impl ObjectLayout for WeaponBase {
    fn on_channel_open(&self, channel: &mut crate::net::ActorChannel, world: &World) {
        let attach_component_guid = self.attach_component.get();

        if attach_component_guid.is_valid() {
            channel.export_net_guid(world.export_guid(attach_component_guid));
        }
    }
}
