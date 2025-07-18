use fadia_codegen::{RepLayout, rpc_handlers};
use fadia_engine::FNetworkGUID;
use fadia_engine::replication::property::{PropertyObject, PropertyVector};

use crate::logic::actor::NetRole;
use crate::logic::hotta::HottaReplicatedObject;
use crate::logic::hotta::player_state::HottaPlayerState;
use crate::logic::rpc::call_rpcs;

use crate::logic::ObjectLayout;
use crate::logic::{actor::PropertyNetRole, rpc::RpcContext};
use crate::net::World;

use super::PlayerState;

#[derive(Debug, RepLayout)]
#[max_rep_index(182)]
pub struct PlayerControllerBase {
    #[rep(handle = 5)]
    pub remote_role: PropertyNetRole,
    #[rep(handle = 14)]
    pub role: PropertyNetRole,
    #[rep(handle = 17)]
    pub player_state: PropertyObject,
    #[rep(handle = 20)]
    pub spawn_location: PropertyVector,
    #[rep(ignore)]
    pub hud: FNetworkGUID,
}

impl PlayerControllerBase {
    pub fn new(
        world: &mut World,
        remote_role: NetRole,
        role: NetRole,
    ) -> (
        FNetworkGUID,
        Self,
        Vec<(FNetworkGUID, Box<dyn ObjectLayout>)>,
    ) {
        (
            world
                .net_guid_cache
                .assign_new_net_guid_for_dynamic_object(None),
            PlayerControllerBase {
                remote_role: PropertyNetRole::new(remote_role),
                role: PropertyNetRole::new(role),
                player_state: PropertyObject::default(),
                spawn_location: PropertyVector::default(),
                hud: FNetworkGUID::default(),
            },
            Vec::new(),
        )
    }
}

impl ObjectLayout for PlayerControllerBase {
    fn on_channel_open(&self, channel: &mut crate::net::ActorChannel, world: &World) {
        channel.export_net_guid(world.export_guid(self.hud));
    }
}

#[rpc_handlers]
impl PlayerControllerBase {
    #[rpc(173, server)]
    fn on_server_request_actor_items(context: RpcContext) {
        let assets = context.world.assets;

        let player_controller_guid = context
            .world
            .player_controller_map
            .get(&context.connection.net_player_index())
            .copied()
            .unwrap();

        let player_state_guid = context
            .world
            .get_actor_archetype_new::<PlayerControllerBase>(player_controller_guid)
            .unwrap()
            .data()
            .player_state
            .get();

        let pawn_guid = context
            .world
            .get_actor_archetype_new::<PlayerState>(player_state_guid)
            .unwrap()
            .data()
            .equipped_players
            .get(0)
            .unwrap()
            .get();

        let player_controller_base = context
            .world
            .get_actor_archetype_mut_new::<PlayerControllerBase>(player_controller_guid)
            .unwrap();

        call_rpcs!(player_controller_base.client_retry_client_restart(pawn_guid));

        let player_state = context
            .world
            .get_actor_archetype_mut_new::<PlayerState>(player_state_guid)
            .unwrap();

        let state_data = HottaPlayerState {
            has_named: true,
            world_level: 5,
            max_world_level: 5,
            unlock_avatar_ids: assets
                .data_asset_set
                .avatar_data_table
                .rows
                .keys()
                .cloned()
                .collect(),
            function_unlock_array: assets
                .data_asset_set
                .function_unlock_table
                .rows
                .values()
                .map(|data| data.id.clone())
                .collect(),
            ..Default::default()
        };

        let replicated_state_container = state_data.replicate(player_state_guid);

        call_rpcs! {
            player_state.send_replicated_object_property_array_to_client(replicated_state_container);
            player_state.client_initial_rpcs_finished()
        }
    }

    #[rpc(177, server)]
    fn on_server_set_lock_direction_type(context: RpcContext) {
        let player_controller_guid = context
            .world
            .player_controller_map
            .get(&context.connection.net_player_index())
            .copied()
            .unwrap();

        let player_state_guid = context
            .world
            .get_actor_archetype_new::<PlayerControllerBase>(player_controller_guid)
            .unwrap()
            .data()
            .player_state
            .get();

        context
            .world
            .get_actor_archetype_mut_new::<PlayerState>(player_state_guid)
            .unwrap()
            .data_mut()
            .server_ready_flag
            .set_value(true);
    }

    #[rpc(46, client)]
    pub fn client_retry_client_restart(&self, new_pawn: FNetworkGUID) {}

    #[rpc(53, client)]
    pub fn client_set_hud(&self, hud_guid: FNetworkGUID) {}
}
