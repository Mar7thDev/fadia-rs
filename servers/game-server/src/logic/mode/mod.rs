use fadia_engine::{
    FNetworkGUID, replication::property::PropertyObject, rotator::FRotator, util::FName,
    vector::FVector3d,
};
use game_session::GameSession;

use crate::{
    logic::rpc::call_rpcs,
    net::{ClassHierarchy, NetConnection, SpawnActorParams, World},
};

use super::{
    actor::NetRole,
    layout::{HTPlayerCharacter, PlayerControllerBase, PlayerState, WeaponBase},
    state::GameStateBase,
};

mod game_session;

pub use game_session::SessionLoginOptions;

#[derive(thiserror::Error, Debug)]
#[error("PreLogin failed: {0}")]
pub struct PreLoginError(pub &'static str);

#[derive(thiserror::Error, Debug)]
#[error("Login failed: {0}")]
pub struct LoginError(pub &'static str);

pub trait GameModeBase {
    fn init_game_state(&self, state: &mut dyn GameStateBase);
    fn game_state_class(&self) -> FNetworkGUID;
    fn pre_login(&self, options: SessionLoginOptions) -> Result<(), PreLoginError>;
    fn login(
        &self,
        connection: &mut NetConnection,
        remote_role: NetRole,
        world: &mut World,
        options: SessionLoginOptions,
        unique_id: String,
    ) -> Result<FNetworkGUID, LoginError>;
    fn post_login(
        &self,
        connection: &mut NetConnection,
        player_controller_guid: FNetworkGUID,
        world: &mut World,
    );
}

pub trait NewGameMode: GameModeBase {
    fn new(world: &mut World) -> Self;
}

pub struct HTGameMode {
    self_guid: FNetworkGUID,
    player_controller_class: FNetworkGUID,
    player_state_class: FNetworkGUID,
    game_state_class: FNetworkGUID,
    hud_class: FNetworkGUID,
    spectator_class: FNetworkGUID,
    game_session: GameSession,
}

const PLAYER_CONTROLLER_CLASS: &[&str] = &[
    "/Game/Blueprints/Share/Character/Player/BP_PlayerControllerBase",
    "Default__BP_PlayerControllerBase_C",
];

const PLAYER_STATE_CLASS: &[&str] = &[
    "/Game/Blueprints/Share/Character/Player/BP_PlayerStateBase",
    "Default__BP_PlayerStateBase_C",
];

const GAME_STATE_CLASS: &[&str] = &[
    "/Game/Blueprints/GameMode/BP_GameStateBase",
    "Default__BP_GameStateBase_C",
];

const HUD_CLASS: &[&str] = &["/Game/HUD/BP_HTHUD", "BP_HTHUD_C"];

const GAME_MODE_CLASS: &[&str] = &["/Game/Blueprints/GameMode/BP_HTGameMode", "BP_HTGameMode_C"];

const SPECTATOR_CLASS: &[&str] = &["/Script/Engine", "SpectatorPawn"];

impl NewGameMode for HTGameMode {
    fn new(world: &mut World) -> Self {
        let self_guid = world.register_hierarchy_for_static_objects(GAME_MODE_CLASS);

        let player_controller_class =
            world.register_hierarchy_for_static_objects(PLAYER_CONTROLLER_CLASS);

        let player_state_class = world.register_hierarchy_for_static_objects(PLAYER_STATE_CLASS);
        let game_state_class = world.register_hierarchy_for_static_objects(GAME_STATE_CLASS);
        let hud_class = world.register_hierarchy_for_static_objects(HUD_CLASS);
        let spectator_class = world.register_hierarchy_for_static_objects(SPECTATOR_CLASS);

        Self {
            self_guid,
            player_controller_class,
            player_state_class,
            game_state_class,
            hud_class,
            spectator_class,
            game_session: GameSession::default(),
        }
    }
}

impl GameModeBase for HTGameMode {
    fn init_game_state(&self, state: &mut dyn GameStateBase) {
        state.received_game_mode_class(self.self_guid);
        state.received_spectator_class(self.spectator_class);
    }

    fn game_state_class(&self) -> FNetworkGUID {
        self.game_state_class
    }

    fn pre_login(&self, options: SessionLoginOptions) -> Result<(), PreLoginError> {
        self.game_session
            .approve_login(options)
            .map_err(PreLoginError)
    }

    fn login(
        &self,
        _connection: &mut NetConnection,
        remote_role: NetRole,
        world: &mut World,
        options: SessionLoginOptions,
        unique_id: String,
    ) -> Result<FNetworkGUID, LoginError> {
        self.game_session
            .approve_login(options)
            .map_err(LoginError)?;

        let player_controller_guid = self.spawn_player_controller_common(
            remote_role,
            FVector3d::new(-79551.5, 158422.4, 4939.1),
            FRotator::default(),
            world,
        );

        self.init_new_player(player_controller_guid, world, unique_id);

        Ok(player_controller_guid)
    }

    fn post_login(
        &self,
        connection: &mut NetConnection,
        player_controller_guid: FNetworkGUID,
        world: &mut World,
    ) {
        self.generic_player_initialization(player_controller_guid, world);
        self.handle_starting_new_player(connection, player_controller_guid, world);
    }
}

impl HTGameMode {
    fn spawn_player_controller_common(
        &self,
        remote_role: NetRole,
        spawn_location: FVector3d,
        spawn_rotation: FRotator,
        world: &mut World,
    ) -> FNetworkGUID {
        let (controller_guid, mut controller_archetype_rep, controller_sub_objects) =
            PlayerControllerBase::new(world, remote_role, NetRole::Authority);

        controller_archetype_rep
            .spawn_location
            .set_value(spawn_location.clone());

        // Spawn PlayerState right away as well
        let (state_guid, mut state_archetype_rep, state_sub_objects) =
            PlayerState::new(world, NetRole::SimulatedProxy, NetRole::Authority);

        controller_archetype_rep.player_state.set_value(state_guid);
        state_archetype_rep.owner.set_value(controller_guid);

        world.spawn_actor(
            SpawnActorParams::Dynamic {
                guid: controller_guid,
                pos: spawn_location,
                rot: spawn_rotation,
                archetype: self.player_controller_class,
                archetype_rep: Box::new(controller_archetype_rep),
            },
            controller_sub_objects,
        );

        world.spawn_actor(
            SpawnActorParams::Dynamic {
                guid: state_guid,
                pos: FVector3d::default(),
                rot: FRotator::default(),
                archetype: self.player_state_class,
                archetype_rep: Box::new(state_archetype_rep),
            },
            state_sub_objects,
        );

        controller_guid
    }

    fn init_new_player(&self, controller_guid: FNetworkGUID, world: &mut World, unique_id: String) {
        self.game_session
            .register_player(controller_guid, world, unique_id);
    }

    fn generic_player_initialization(&self, controller_guid: FNetworkGUID, world: &mut World) {
        if let Some(mut player_controller) =
            world.get_actor_archetype_mut_new::<PlayerControllerBase>(controller_guid)
        {
            call_rpcs!(player_controller.client_set_hud(self.hud_class));

            player_controller.data_mut().hud = self.hud_class;
        }
    }

    fn handle_starting_new_player(
        &self,
        connection: &mut NetConnection,
        controller_guid: FNetworkGUID,
        world: &mut World,
    ) {
        self.restart_player(connection, controller_guid, world);
    }

    fn restart_player(
        &self,
        connection: &mut NetConnection,
        controller_guid: FNetworkGUID,
        world: &mut World,
    ) {
        let pawn_guid = self.spawn_default_pawn_for(controller_guid, world);

        let player_controller = world
            .get_actor_archetype_new::<PlayerControllerBase>(controller_guid)
            .unwrap();

        let state_guid = player_controller.data().player_state.get();

        let mut player_state = world
            .get_actor_archetype_mut_new::<PlayerState>(state_guid)
            .unwrap();

        let player_state = player_state.data_mut();
        player_state
            .equipped_players
            .push(PropertyObject::new(pawn_guid));

        // Fill data. Maybe make it configurable later.
        player_state.role_id.set_value(1337);
        player_state.role_level.set_value(60);
        player_state
            .player_name_private
            .set_value(String::from("Hotta"));
        player_state
            .str_role_name
            .set_value(String::from("fadia-rs"));
        player_state
            .sign_content
            .set_value(String::from("discord.gg/reversedrooms"));
        player_state
            .avatar_id
            .set_value(FName::Custom(String::from("1039")));

        world.open_actor_channel_at(connection, 3, controller_guid);
        world.open_actor_channel_at(connection, 4, pawn_guid);
        world.open_actor_channel_at(connection, 5, state_guid);

        // Open a channel for Weapon, if needed

        let character = world
            .get_actor_archetype_new::<HTPlayerCharacter>(pawn_guid)
            .unwrap();

        let weapon_guid = character.data().current_weapon.get();

        if weapon_guid.is_valid() {
            world.open_actor_channel_at(connection, 11, weapon_guid);
        }
    }

    fn spawn_default_pawn_for(
        &self,
        controller_guid: FNetworkGUID,
        world: &mut World,
    ) -> FNetworkGUID {
        let player_controller = world
            .get_actor_archetype_new::<PlayerControllerBase>(controller_guid)
            .unwrap();

        let spawn_location = player_controller.data().spawn_location.get().clone();
        let player_state_guid = player_controller.data().player_state.get();

        let character_config = world
            .assets
            .get_player_character_config(&world.globals.player_character)
            .unwrap();

        let archetype_class = world.register_hierarchy_for_static_objects(&[
            &character_config.class.path,
            &character_config.name,
        ]);

        let (guid, mut character, sub_objects) = HTPlayerCharacter::new(
            world,
            character_config,
            NetRole::AutonomousProxy,
            NetRole::AutonomousProxy,
        );

        character.owner.set_value(controller_guid);
        character.controller.set_value(controller_guid);
        character
            .self_ht_player_controller
            .set_value(controller_guid);

        character.player_state.set_value(player_state_guid);
        character.saved_player_state.set_value(player_state_guid);
        character.server_ready_flag.set_value(true);

        if let Some(weapon) = world
            .assets
            .get_weapon_for_character(&world.globals.player_character)
        {
            let mesh_guid = world
                .net_guid_cache
                .assign_new_net_guid_for_dynamic_object(Some(
                    HTPlayerCharacter::MESH_COMPONENT_NAME,
                ));

            world.register_hierarchy(ClassHierarchy::new(guid).child(mesh_guid));

            let weapon_archetype_guid =
                world.register_hierarchy_for_static_objects(&[&weapon.class.path, &weapon.name]);

            let (weapon_guid, mut weapon, sub_objects) =
                WeaponBase::new(NetRole::SimulatedProxy, NetRole::Authority, world);

            weapon.attach_parent.set_value(guid);
            weapon.attach_component.set_value(mesh_guid);
            weapon.owner.set_value(guid);
            weapon.owner_character.set_value(guid);

            character.current_weapon.set_value(weapon_guid);

            world.spawn_actor(
                SpawnActorParams::Dynamic {
                    guid: weapon_guid,
                    pos: spawn_location.clone(),
                    rot: FRotator::default(),
                    archetype: weapon_archetype_guid,
                    archetype_rep: Box::new(weapon),
                },
                sub_objects,
            );
        }

        world.spawn_actor(
            SpawnActorParams::Dynamic {
                guid,
                pos: spawn_location,
                rot: FRotator::default(),
                archetype: archetype_class,
                archetype_rep: Box::new(character),
            },
            sub_objects,
        )
    }
}
