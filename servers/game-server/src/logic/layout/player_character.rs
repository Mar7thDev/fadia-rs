use fadia_codegen::{RepLayout, ReplicatedProperty, dummy_rpc_handler};

use fadia_config::blueprint::PlayerCharacterConfig;
use fadia_engine::{
    FNetworkGUID,
    replication::{
        FastArraySerializer, NullLayout,
        property::{
            GameplayTagContainer, PropertyArray, PropertyBool, PropertyF32, PropertyObject,
            PropertyU32,
        },
    },
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
pub struct HTPlayerCharacter {
    #[rep(handle = 5)]
    pub remote_role: PropertyNetRole,
    #[rep(handle = 13)]
    pub owner: PropertyObject,
    #[rep(handle = 14)]
    pub role: PropertyNetRole,
    #[rep(handle = 15)]
    pub instigator: PropertyObject,
    #[rep(handle = 16)]
    pub group_id: PropertyU32,
    #[rep(handle = 18)]
    pub player_state: PropertyObject,
    #[rep(handle = 19)]
    pub controller: PropertyObject,
    #[rep(handle = 51)]
    pub self_ht_player_controller: PropertyObject,
    #[rep(handle = 56)]
    pub current_weapon: PropertyObject,
    #[rep(handle = 74)]
    pub server_ready_flag: PropertyBool,
    #[rep(handle = 75)]
    pub saved_player_state: PropertyObject,
}

#[derive(Debug, RepLayout)]
#[dummy_rpc_handler]
pub struct HTAttributeSet {
    #[rep(handle = 1)]
    pub hp_max_base: PropertyF32,
    #[rep(handle = 2)]
    pub hp_max_cur: PropertyF32,
    #[rep(handle = 17)]
    pub atk_base: PropertyF32,
    #[rep(handle = 18)]
    pub atk_cur: PropertyF32,
    #[rep(handle = 19)]
    pub crit_base: PropertyF32,
    #[rep(handle = 20)]
    pub crit_cur: PropertyF32,
    #[rep(handle = 27)]
    pub crit_damage_base: PropertyF32,
    #[rep(handle = 28)]
    pub crit_damage_cur: PropertyF32,
    #[rep(handle = 31)]
    pub charge_get_efficiency_base: PropertyF32,
    #[rep(handle = 32)]
    pub charge_get_efficiency_cur: PropertyF32,
    #[rep(handle = 35)]
    pub charge_max_base: PropertyF32,
    #[rep(handle = 36)]
    pub charge_max_cur: PropertyF32,
    #[rep(handle = 93)]
    pub def_base: PropertyF32,
    #[rep(handle = 94)]
    pub def_cur: PropertyF32,
    #[rep(handle = 167)]
    pub unbal_accrue_efficiency_base: PropertyF32,
    #[rep(handle = 168)]
    pub unbal_accrue_efficiency_cur: PropertyF32,
    #[rep(handle = 177)]
    pub unbal_reduce_natur_base: PropertyF32,
    #[rep(handle = 178)]
    pub unbal_reduce_natur_cur: PropertyF32,
    #[rep(handle = 187)]
    pub tenacity_recover_speed_base: PropertyF32,
    #[rep(handle = 188)]
    pub tenacity_recover_speed_cur: PropertyF32,
    #[rep(handle = 189)]
    pub tenacity_reset_time_base: PropertyF32,
    #[rep(handle = 190)]
    pub tenacity_reset_time_cur: PropertyF32,
    #[rep(handle = 193)]
    pub satiety_max_cur: PropertyF32,
    #[rep(handle = 194)]
    pub satiety_max_base: PropertyF32,
}

#[derive(Debug, RepLayout)]
#[max_rep_index(98)]
#[dummy_rpc_handler]
pub struct AbilitySystemComponent {
    #[rep(handle = 6)]
    pub owner_actor: PropertyObject,
    #[rep(handle = 7)]
    pub avatar_actor: PropertyObject,
    #[rep(handle = 18)]
    pub hp_current: PropertyF32,
    #[rep(handle = 19)]
    pub max_hp: PropertyF32,
    #[rep(handle = 20)]
    pub max_hp_temp: PropertyF32,
    #[rep(handle = 22)]
    pub satiety_change_time: PropertyF32,
    #[rep(handle = 23)]
    pub atk: PropertyF32,
    #[rep(handle = 24)]
    pub charge_current: PropertyF32,
    #[rep(handle = 28)]
    pub unbal_speed: PropertyF32,
    #[rep(index = 7)]
    pub activatable_abilities: FastArraySerializer<GameplayAbilitySpec>,
}

#[derive(Debug, ReplicatedProperty)]
pub struct GameplayAbilityHandle(pub PropertyU32);

#[derive(Debug, RepLayout)]
pub struct GameplayAbilitySpec {
    #[rep(handle = 1)]
    pub handle: GameplayAbilityHandle,
    #[rep(handle = 2)]
    pub ability: PropertyObject,
    #[rep(handle = 3)]
    pub level: PropertyU32,
    #[rep(handle = 4)]
    pub input_id: PropertyU32,
    #[rep(handle = 5)]
    pub source_object: PropertyObject,
    #[rep(handle = 6)]
    pub gameplay_tags: GameplayTagContainer,
    #[rep(handle = 7)]
    pub replicated_instances: PropertyArray<PropertyObject>,
}

impl HTPlayerCharacter {
    pub const MESH_COMPONENT_NAME: &str = "CharacterMesh0";

    pub fn new(
        world: &mut World,
        config: &'static PlayerCharacterConfig,
        remote_role: NetRole,
        role: NetRole,
    ) -> (
        FNetworkGUID,
        Self,
        Vec<(FNetworkGUID, Box<dyn ObjectLayout>)>,
    ) {
        let character_guid = world
            .net_guid_cache
            .assign_new_net_guid_for_dynamic_object(None);
        let mut sub_objects: Vec<(FNetworkGUID, Box<dyn ObjectLayout>)> = Vec::new();

        let attribute_set_guid = world
            .net_guid_cache
            .assign_new_net_guid_for_dynamic_object(Some("AttributeSet"));
        let ability_system_component_guid = world
            .net_guid_cache
            .assign_new_net_guid_for_dynamic_object(Some("AbilitySystemComponent"));
        let state_manager_component_guid = world
            .net_guid_cache
            .assign_new_net_guid_for_dynamic_object(Some("StateManagerComponent"));
        let motion_warping_guid = world
            .net_guid_cache
            .assign_new_net_guid_for_dynamic_object(Some("MotionWarping"));
        let ht_character_attribute_set_guid = world
            .net_guid_cache
            .assign_new_net_guid_for_dynamic_object(Some("HTCharacterAttributeSet"));

        let mut activatable_abilities = FastArraySerializer::default();
        let mut ability_guids = Vec::new();

        for (i, ability) in config
            .properties
            .granted_abilities
            .iter()
            .chain(config.properties.passive_abilities.iter())
            .filter(|config| !config.ability_class.is_empty())
            .enumerate()
        {
            let ability_config = world
                .assets
                .get_player_ability_config(&ability.ability_class)
                .unwrap();

            let (outer, _inner) = &ability.ability_class.asset_path_name;

            let ability_guid =
                world.register_hierarchy_for_static_objects(&[outer, &ability_config.name]);

            activatable_abilities.push(GameplayAbilitySpec {
                handle: GameplayAbilityHandle(PropertyU32::new(10000 + i as u32)),
                ability: PropertyObject::new(ability_guid),
                level: PropertyU32::new(1),
                input_id: PropertyU32::new(ability.input_id.into()),
                source_object: PropertyObject::new(character_guid),
                gameplay_tags: GameplayTagContainer,
                replicated_instances: PropertyArray::default(),
            });

            ability_guids.push(ability_guid);
        }

        sub_objects.push((
            attribute_set_guid,
            Box::new(HTAttributeSet {
                hp_max_base: PropertyF32::new(1000.0),
                hp_max_cur: PropertyF32::new(1000.0),
                atk_base: PropertyF32::new(75.0),
                atk_cur: PropertyF32::new(75.0),
                crit_base: PropertyF32::new(0.05),
                crit_cur: PropertyF32::new(0.05),
                crit_damage_base: PropertyF32::new(0.5),
                crit_damage_cur: PropertyF32::new(0.5),
                charge_get_efficiency_base: PropertyF32::new(1.0),
                charge_get_efficiency_cur: PropertyF32::new(1.0),
                charge_max_base: PropertyF32::new(30.0),
                charge_max_cur: PropertyF32::new(30.0),
                def_base: PropertyF32::new(60.0),
                def_cur: PropertyF32::new(60.0),
                unbal_accrue_efficiency_base: PropertyF32::new(1.0),
                unbal_accrue_efficiency_cur: PropertyF32::new(1.0),
                unbal_reduce_natur_base: PropertyF32::new(100.0),
                unbal_reduce_natur_cur: PropertyF32::new(100.0),
                tenacity_recover_speed_base: PropertyF32::new(10.0),
                tenacity_recover_speed_cur: PropertyF32::new(10.0),
                tenacity_reset_time_base: PropertyF32::new(3.0),
                tenacity_reset_time_cur: PropertyF32::new(3.0),
                satiety_max_base: PropertyF32::new(100.0),
                satiety_max_cur: PropertyF32::new(100.0),
            }),
        ));

        sub_objects.push((
            ability_system_component_guid,
            Box::new(AbilitySystemComponent {
                owner_actor: PropertyObject::new(character_guid),
                avatar_actor: PropertyObject::new(character_guid),
                hp_current: PropertyF32::new(1337.0),
                max_hp: PropertyF32::new(1337.0),
                max_hp_temp: PropertyF32::new(1337.0),
                satiety_change_time: PropertyF32::new(4912.057),
                atk: PropertyF32::new(75.0),
                charge_current: PropertyF32::new(100.0),
                unbal_speed: PropertyF32::new(200.0),
                activatable_abilities,
            }),
        ));

        sub_objects.push((motion_warping_guid, Box::new(NullLayout)));
        sub_objects.push((ht_character_attribute_set_guid, Box::new(NullLayout)));
        sub_objects.push((state_manager_component_guid, Box::new(NullLayout)));

        (
            character_guid,
            HTPlayerCharacter {
                remote_role: PropertyNetRole::new(remote_role),
                owner: PropertyObject::default(),
                role: PropertyNetRole::new(role),
                instigator: PropertyObject::new(character_guid),
                group_id: PropertyU32::default(),
                player_state: PropertyObject::default(),
                controller: PropertyObject::default(),
                self_ht_player_controller: PropertyObject::default(),
                current_weapon: PropertyObject::default(),
                server_ready_flag: PropertyBool::default(),
                saved_player_state: PropertyObject::default(),
            },
            sub_objects,
        )
    }
}

impl ObjectLayout for HTPlayerCharacter {}
impl ObjectLayout for HTAttributeSet {}

impl ObjectLayout for AbilitySystemComponent {
    fn on_channel_open(&self, channel: &mut crate::net::ActorChannel, world: &World) {
        self.activatable_abilities.iter().for_each(|(_, ability)| {
            channel.export_net_guid(world.export_guid(ability.ability.get()));
        });
    }
}
