use fadia_codegen::{RepLayout, rpc_handlers};
use fadia_engine::{
    FNetworkGUID,
    replication::{
        NullLayout,
        property::{
            PropertyArray, PropertyBool, PropertyF32, PropertyName, PropertyObject, PropertyString,
            PropertyU8, PropertyU32, PropertyU64, ReplicatedProperty,
        },
    },
    util::FName,
};
use tracing::info;

use crate::{
    logic::{
        ObjectLayout,
        actor::{NetRole, PropertyNetRole},
        hotta::HottaReplicatedObjectPropertyContainer,
        layout::PlayerControllerBase,
        rpc::RpcContext,
    },
    net::World,
};

#[derive(Debug, RepLayout)]
#[max_rep_index(424)]
pub struct PlayerState {
    #[rep(handle = 5)]
    pub remote_role: PropertyNetRole,
    #[rep(handle = 13)]
    pub owner: PropertyObject,
    #[rep(handle = 14)]
    pub role: PropertyNetRole,
    #[rep(handle = 16)]
    pub group_id: PropertyU32,
    #[rep(handle = 18)]
    pub player_id: PropertyU32,
    #[rep(handle = 25)]
    pub start_time: PropertyU32,
    #[rep(handle = 26)]
    pub unique_id: PlayerUniqueID,
    #[rep(handle = 27)]
    pub player_name_private: PropertyString,
    #[rep(handle = 28)]
    pub str_role_name: PropertyString,
    #[rep(handle = 30)]
    pub role_id: PropertyU64,
    #[rep(handle = 35)]
    pub server_ready_flag: PropertyBool,
    #[rep(handle = 37)]
    pub sign_content: PropertyString,
    #[rep(handle = 38)]
    pub birthday_month: PropertyU32,
    #[rep(handle = 39)]
    pub birthday_day: PropertyU32,
    #[rep(handle = 40)]
    pub play_time_seconds: PropertyF32,
    #[rep(handle = 41)]
    pub player_world_time_seconds_delta: PropertyF32,
    #[rep(handle = 46)]
    pub current_stamina: PropertyU32,
    #[rep(handle = 56)]
    pub rand_bean: PropertyU32,
    #[rep(handle = 57)]
    pub cur_bean_count: PropertyU32,
    #[rep(handle = 61)]
    pub strength_current: PropertyF32,
    #[rep(handle = 66)]
    pub role_level: PropertyU32,
    #[rep(handle = 67)]
    pub role_exp: PropertyU32,
    #[rep(handle = 71)]
    pub equipped_players: PropertyArray<PropertyObject>,
    #[rep(handle = 74)]
    pub curr_character_net_id_solt: PropertyU32,
    #[rep(handle = 75)]
    pub curr_character_net_id_serial: PropertyU32,
    #[rep(handle = 99)]
    pub avatar_id: PropertyName,
}

#[derive(Debug, Default)]
pub struct PlayerUniqueID {
    pub platform_id: PropertyU8,
    pub device: PropertyString,
}

const SUB_CLASSES: &[&str] = &[
    "InventoryComponent",
    "TimerClock",
    "CDManager",
    "ShieldComponent",
    "VehicleComponent",
    "SystematicPlayerComponent",
];

impl PlayerState {
    pub fn new(
        world: &mut World,
        remote_role: NetRole,
        role: NetRole,
    ) -> (
        FNetworkGUID,
        Self,
        Vec<(FNetworkGUID, Box<dyn ObjectLayout>)>,
    ) {
        let own_guid = world
            .net_guid_cache
            .assign_new_net_guid_for_dynamic_object(None);

        let sub_objects = SUB_CLASSES
            .iter()
            .map(|name| {
                let sub_guid = world
                    .net_guid_cache
                    .assign_new_net_guid_for_dynamic_object(Some(name));

                (sub_guid, Box::new(NullLayout) as _)
            })
            .collect();

        (
            own_guid,
            PlayerState::internal_new(remote_role, role),
            sub_objects,
        )
    }

    fn internal_new(remote_role: NetRole, role: NetRole) -> Self {
        PlayerState {
            remote_role: PropertyNetRole::new(remote_role),
            role: PropertyNetRole::new(role),
            owner: Default::default(),
            group_id: Default::default(),
            player_id: Default::default(),
            start_time: Default::default(),
            unique_id: Default::default(),
            player_name_private: Default::default(),
            str_role_name: Default::default(),
            role_id: Default::default(),
            server_ready_flag: Default::default(),
            sign_content: Default::default(),
            birthday_month: Default::default(),
            birthday_day: Default::default(),
            play_time_seconds: Default::default(),
            player_world_time_seconds_delta: Default::default(),
            current_stamina: PropertyU32::new(200),
            rand_bean: Default::default(),
            cur_bean_count: Default::default(),
            strength_current: PropertyF32::new(100.0),
            role_level: PropertyU32::new(1),
            role_exp: Default::default(),
            equipped_players: Default::default(),
            curr_character_net_id_solt: Default::default(),
            curr_character_net_id_serial: Default::default(),
            avatar_id: PropertyName::new(FName::Custom(String::from("1"))),
        }
    }
}

#[rpc_handlers]
impl PlayerState {
    #[rpc(103, client)]
    pub fn client_initial_rpcs_finished(&self) {}

    #[rpc(256, client)]
    pub fn send_replicated_object_property_array_to_client(
        &self,
        container: HottaReplicatedObjectPropertyContainer,
    ) {
    }

    #[rpc(364, server)]
    pub fn server_player_rename(context: RpcContext, new_name: String) {
        info!("player rename: {new_name}");

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
            .str_role_name
            .set_value(new_name);
    }
}

impl ReplicatedProperty for PlayerUniqueID {
    fn is_changed(&self) -> bool {
        self.platform_id.is_changed() || self.device.is_changed()
    }

    fn acknowledge_changes(&mut self) {
        self.platform_id.acknowledge_changes();
        self.device.acknowledge_changes();
    }

    fn serialize(&self, w: &mut fadia_engine::util::OutBitWriter) -> std::io::Result<()> {
        self.platform_id.serialize(w)?;
        self.device.serialize(w)
    }
}

impl ObjectLayout for PlayerState {}
