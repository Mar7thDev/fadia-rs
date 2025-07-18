mod game_experience;

pub use game_experience::GameExperienceComponent;

use fadia_codegen::{RepLayout, dummy_rpc_handler};
use fadia_engine::{
    FNetworkGUID,
    replication::property::{
        PropertyBool, PropertyF64, PropertyName, PropertyObject, PropertyU32, PropertyU64,
    },
    util::FName,
};

use crate::{
    logic::actor::PropertyNetRole,
    net::{ClassHierarchy, World},
};

use super::{ObjectLayout, actor::NetRole};

pub trait GameStateBase {
    fn received_game_mode_class(&mut self, guid: FNetworkGUID);
    fn received_spectator_class(&mut self, guid: FNetworkGUID);
}

#[derive(Debug, RepLayout)]
#[dummy_rpc_handler]
pub struct HTGameState {
    #[rep(handle = 5)]
    pub remote_role: PropertyNetRole,
    #[rep(handle = 14)]
    pub role: PropertyNetRole,
    #[rep(handle = 17)]
    pub game_mode_class: PropertyObject,
    #[rep(handle = 18)]
    pub spectator_class: PropertyObject,
    #[rep(handle = 19)]
    pub replicated_has_begun_play: PropertyBool,
    #[rep(handle = 21)]
    pub replicated_world_time_seconds_double: PropertyF64,
    #[rep(handle = 22)]
    pub match_state: PropertyName,
    #[rep(handle = 23)]
    pub elapsed_time: PropertyU32,
    #[rep(handle = 24)]
    pub hotta_server_date_time_now: PropertyU64,
    #[rep(handle = 25)]
    pub hotta_server_date_time_utc_now: PropertyU64,
}

impl HTGameState {
    const GAME_EXPERIENCE_COMPONENT: &str = "GameExperienceComponent";
    const MASS_EXPERIENCE_CLASS: &[&str] = &[
        "/Game/GameExperiences/BP_MassExperience",
        "Default__BP_MassExperience_C",
    ];

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
        let experience_component_guid = world
            .net_guid_cache
            .assign_new_net_guid_for_dynamic_object(Some(Self::GAME_EXPERIENCE_COMPONENT));
        let mass_experience =
            world.register_hierarchy_for_static_objects(Self::MASS_EXPERIENCE_CLASS);

        world.register_hierarchy(ClassHierarchy::new(own_guid).child(experience_component_guid));

        (
            own_guid,
            Self {
                remote_role: PropertyNetRole::new(remote_role),
                role: PropertyNetRole::new(role),
                game_mode_class: PropertyObject::default(),
                spectator_class: PropertyObject::default(),
                replicated_has_begun_play: PropertyBool::new(true),
                replicated_world_time_seconds_double: PropertyF64::default(),
                match_state: PropertyName::new(FName::Custom(String::from("InProgress"))),
                elapsed_time: PropertyU32::default(),
                hotta_server_date_time_now: PropertyU64::default(),
                hotta_server_date_time_utc_now: PropertyU64::default(),
            },
            vec![(
                experience_component_guid,
                Box::new(GameExperienceComponent {
                    current_game_experience: PropertyObject::new(mass_experience),
                }),
            )],
        )
    }
}

impl GameStateBase for HTGameState {
    fn received_game_mode_class(&mut self, guid: FNetworkGUID) {
        self.game_mode_class.set_value(guid);
    }

    fn received_spectator_class(&mut self, guid: FNetworkGUID) {
        self.spectator_class.set_value(guid);
    }
}

impl ObjectLayout for HTGameState {
    fn on_channel_open(&self, channel: &mut crate::net::ActorChannel, world: &World) {
        let game_mode_class = self.game_mode_class.get();
        let spectator_class = self.spectator_class.get();

        if game_mode_class.is_valid() {
            channel.export_net_guid(world.export_guid(game_mode_class));
        }

        if spectator_class.is_valid() {
            channel.export_net_guid(world.export_guid(spectator_class));
        }
    }
}
