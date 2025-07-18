use std::{
    borrow::Cow,
    cell::OnceCell,
    collections::{HashMap, HashSet},
    io,
    rc::Rc,
};

use common::time_util;
use tracing::{error, info, warn};

use super::{
    channel::{ControlChannelMessage, NAME_ACTOR_CHANNEL},
    connection::NetConnection,
};
use crate::{
    assets::GameAssets,
    config::GameplayGlobals,
    logic::{
        MutObjectWrap, Object, ObjectLayout, RefObjectWrap,
        actor::{Actor, NetPlayerIndex, NetRole},
        layout::WorldDataLayers,
        mode::{GameModeBase, LoginError, NewGameMode},
        rpc::RpcContext,
        state::HTGameState,
    },
    net::channel,
};
use fadia_engine::vector::FVector3d;
use fadia_engine::{FNetworkGUID, rotator::FRotator};
use fadia_engine::{NetGUIDCache, replication::NullLayout};

pub struct World {
    // settings
    pub assets: &'static GameAssets,
    pub globals: &'static GameplayGlobals,
    // logic
    pub actors: HashMap<FNetworkGUID, Actor>,
    pub objects: HashMap<FNetworkGUID, Object>,
    class_hierarchy: HashMap<FNetworkGUID, FNetworkGUID>,
    pub net_guid_cache: NetGUIDCache,
    pub player_controller_map: HashMap<NetPlayerIndex, FNetworkGUID>,
    game_mode: OnceCell<Rc<dyn GameModeBase>>,
    game_state: OnceCell<FNetworkGUID>,
    world_data_layers: OnceCell<FNetworkGUID>,
}

pub enum SpawnActorParams {
    Static(FNetworkGUID, Box<dyn ObjectLayout>),
    Dynamic {
        guid: FNetworkGUID,
        pos: FVector3d,
        rot: FRotator,
        archetype: FNetworkGUID,
        archetype_rep: Box<dyn ObjectLayout>,
    },
}

pub struct ClassHierarchy(Vec<FNetworkGUID>);

impl ClassHierarchy {
    pub fn new(outermost: FNetworkGUID) -> Self {
        Self(vec![outermost])
    }

    pub fn child(mut self, child: FNetworkGUID) -> Self {
        self.0.push(child);
        self
    }
}

#[derive(Debug)]
pub struct FNetFieldExportChain(pub Vec<(FNetworkGUID, &'static str, bool)>);

impl World {
    pub fn new<GameMode: NewGameMode + 'static>(
        assets: &'static GameAssets,
        globals: &'static GameplayGlobals,
    ) -> Self {
        let mut world = Self {
            assets,
            globals,
            actors: HashMap::new(),
            objects: HashMap::new(),
            class_hierarchy: HashMap::new(),
            net_guid_cache: NetGUIDCache::new(&globals.map),
            player_controller_map: HashMap::new(),
            game_mode: OnceCell::new(),
            game_state: OnceCell::new(),
            world_data_layers: OnceCell::new(),
        };

        let game_mode = GameMode::new(&mut world);
        let _ = world.game_mode.set(Rc::new(game_mode));

        world.init_state();
        world.init_level();

        world
    }

    fn init_state(&mut self) {
        let game_mode = self.game_mode();
        let class = game_mode.game_state_class();

        let (game_state_guid, mut game_state, sub_objects) =
            HTGameState::new(self, NetRole::SimulatedProxy, NetRole::Authority);

        game_mode.init_game_state(&mut game_state);

        game_state
            .hotta_server_date_time_now
            .set_value(time_util::unix_timestamp_ms());

        game_state
            .hotta_server_date_time_utc_now
            .set_value(time_util::unix_utc_timestamp_ms());

        let game_state_guid = self.spawn_actor(
            SpawnActorParams::Dynamic {
                guid: game_state_guid,
                pos: FVector3d::default(),
                rot: FRotator::default(),
                archetype: class,
                archetype_rep: Box::new(game_state),
            },
            sub_objects,
        );

        let _ = self.game_state.set(game_state_guid);
    }

    fn init_level(&mut self) {
        self.register_hierarchy_for_static_objects(&[
            "/Game/Maps/Map_bigworld/XL_map_bigworld_test",
            "XL_map_bigworld_test",
            "PersistentLevel",
        ]);

        let (data_layers_guid, data_layers) =
            WorldDataLayers::new(NetRole::SimulatedProxy, NetRole::Authority, self);

        self.spawn_actor(
            SpawnActorParams::Static(data_layers_guid, Box::new(data_layers)),
            Vec::new(),
        );

        let _ = self.world_data_layers.set(data_layers_guid);
    }

    pub fn game_mode(&self) -> Rc<dyn GameModeBase> {
        Rc::clone(self.game_mode.get().unwrap())
    }

    pub fn register_hierarchy(&mut self, hierarchy: ClassHierarchy) {
        let mut outer_guids = hierarchy.0.iter();
        let mut inner_guids = hierarchy.0.iter().skip(1);

        while let Some((outer, inner)) = outer_guids.next().zip(inner_guids.next()) {
            self.class_hierarchy.insert(*inner, *outer);
        }
    }

    pub fn register_hierarchy_for_static_objects(
        &mut self,
        path_names_ordered: &[&'static str],
    ) -> FNetworkGUID {
        let mut path_names_ordered = path_names_ordered.iter();
        let outermost = path_names_ordered.next().unwrap();
        let mut last_child_guid = self
            .net_guid_cache
            .get_or_assign_net_guid_for_static(outermost);

        let mut hierarchy = ClassHierarchy::new(last_child_guid);

        for path in path_names_ordered {
            last_child_guid = self.net_guid_cache.get_or_assign_net_guid_for_static(path);
            hierarchy = hierarchy.child(last_child_guid);
        }

        self.register_hierarchy(hierarchy);
        last_child_guid
    }

    pub fn export_guid(&self, export_guid: FNetworkGUID) -> FNetFieldExportChain {
        let mut next_guid = Some(export_guid);
        let mut exports = FNetFieldExportChain(Vec::new());

        loop {
            let Some(guid) = next_guid else {
                break exports;
            };

            let cache_object = self.net_guid_cache.get(guid).unwrap();
            exports
                .0
                .push((guid, cache_object.path_name, cache_object.no_load));

            next_guid = self.class_hierarchy.get(&guid).copied();
        }
    }

    pub fn notify_control_message(
        &mut self,
        connection: &mut NetConnection,
        message: ControlChannelMessage,
    ) -> io::Result<()> {
        use ControlChannelMessage::*;
        match message {
            Hello(channel::Hello(
                is_little_endian,
                remote_network_version,
                _encryption_token,
                remote_network_features,
            )) => {
                info!(
                    "received hello from client: little_endian: {is_little_endian}, network version: {remote_network_version}, network features: {remote_network_features}"
                );

                connection.send_challenge_control_message().unwrap();
            }
            Netspeed(channel::Netspeed(net_speed)) => {
                connection.current_net_speed = net_speed;
                info!("Client netspeed is {net_speed}");
            }
            Login(channel::Login(
                client_response,
                request_url,
                _uid_flags,
                unique_id,
                online_platform,
            )) => {
                info!(
                    "Login: response: {client_response} request_url: {request_url} unique_id: {unique_id} online_platform: {online_platform}"
                );

                if let Err(err) = self.game_mode().pre_login(connection.login_options()) {
                    error!("GameMode::pre_login failed: {err}");
                } else {
                    let _ = connection.unique_id.set(unique_id);
                    self.welcome_player(connection);
                }
            }
            Join(channel::Join()) => {
                if connection.player_controller.is_none() {
                    let Some(unique_id) = connection.unique_id.get().cloned() else {
                        error!("received NMT_Join, but connection.unique_id is not set!");
                        return Ok(());
                    };

                    if let Err(err) = self.spawn_play_actor(
                        connection,
                        NetRole::AutonomousProxy,
                        unique_id,
                        connection.net_player_index(),
                    ) {
                        error!("spawn_play_actor failed: {err}");
                        return Ok(());
                    }

                    // Open channels for GameState and WorldDataLayers

                    self.open_actor_channel_at(
                        connection,
                        6,
                        self.game_state.get().copied().unwrap(),
                    );

                    self.open_actor_channel_at(
                        connection,
                        7,
                        self.world_data_layers.get().copied().unwrap(),
                    );
                }
            }
            Welcome(_) | Challenge(_) => {
                error!("received server-side control channel message from client!");
            }
            Unknown(unknown) => {
                warn!("handler for control message with id {unknown} is not implemented")
            }
        }

        Ok(())
    }

    fn welcome_player(&self, connection: &mut NetConnection) {
        connection
            .send_welcome_control_message(
                Cow::Borrowed(&self.globals.map),
                Cow::Borrowed(&self.globals.game_name),
                Cow::Borrowed(&self.globals.redirect_url),
            )
            .unwrap();
    }

    fn spawn_play_actor(
        &mut self,
        connection: &mut NetConnection,
        remote_role: NetRole,
        unique_id: String,
        player_index: NetPlayerIndex,
    ) -> Result<FNetworkGUID, LoginError> {
        let game_mode = self.game_mode();
        let login_options = connection.login_options();

        let player_controller_guid =
            game_mode.login(connection, remote_role, self, login_options, unique_id)?;

        let player_controller_actor = self.actors.get_mut(&player_controller_guid).unwrap();
        player_controller_actor.set_controlled_player(player_index);

        self.player_controller_map
            .insert(player_index, player_controller_guid);

        self.game_mode()
            .post_login(connection, player_controller_guid, self);

        Ok(player_controller_guid)
    }

    pub fn spawn_actor(
        &mut self,
        params: SpawnActorParams,
        sub_objects: Vec<(FNetworkGUID, Box<dyn ObjectLayout>)>,
    ) -> FNetworkGUID {
        let mut sub_object_set = sub_objects
            .iter()
            .map(|(guid, _)| *guid)
            .collect::<HashSet<_>>();

        let actor_guid = match params {
            SpawnActorParams::Static(guid, rep) => {
                self.actors.insert(guid, Actor::new(guid, guid));
                self.objects.insert(
                    guid,
                    Object {
                        rep_layout: rep,
                        sub_objects: sub_object_set,
                        queued_rpcs: Vec::new(),
                    },
                );

                guid
            }
            SpawnActorParams::Dynamic {
                guid,
                pos,
                rot,
                archetype,
                archetype_rep,
            } => {
                let mut actor = Actor::new(guid, archetype);
                actor.position = pos;
                actor.rotation = rot;

                self.actors.insert(guid, actor);

                sub_object_set.insert(archetype);
                self.objects.insert(
                    guid,
                    Object {
                        rep_layout: Box::new(NullLayout),
                        sub_objects: sub_object_set,
                        queued_rpcs: Vec::new(),
                    },
                );

                self.objects.insert(
                    archetype,
                    Object {
                        rep_layout: archetype_rep,
                        sub_objects: HashSet::new(),
                        queued_rpcs: Vec::new(),
                    },
                );

                guid
            }
        };

        for (guid, layout) in sub_objects {
            if guid.is_dynamic() {
                self.register_hierarchy(ClassHierarchy::new(actor_guid).child(guid));
            }

            self.objects.insert(
                guid,
                Object {
                    rep_layout: layout,
                    sub_objects: HashSet::new(),
                    queued_rpcs: Vec::new(),
                },
            );
        }

        actor_guid
    }

    pub fn open_actor_channel_at(
        &mut self,
        connection: &mut NetConnection,
        index: u32,
        guid: FNetworkGUID,
    ) {
        if !self.actors.contains_key(&guid) {
            error!("failed to open channel, guid {guid:?} is not an actor");
            return;
        }

        let channel = connection
            .create_actor_channel(index, guid, NAME_ACTOR_CHANNEL)
            .unwrap();

        let object = self.objects.get(&guid).unwrap();
        object.on_channel_open(guid, &mut channel.channel_impl, self);
    }

    pub fn tick(&mut self, connection: &mut NetConnection) {
        self.tick_rpc(connection);
        self.tick_network(connection);
    }

    fn tick_rpc(&mut self, connection: &mut NetConnection) {
        while let Some((ch_index, obj_guid, rpc)) = connection.next_rpc() {
            info!(
                "received RPC to channel {ch_index}, rep_index: {}",
                rpc.rep_index
            );

            let rep_index = rpc.rep_index;

            let Some(rpc_handler) = self
                .objects
                .get(&obj_guid)
                .and_then(|obj| obj.rep_layout.get_handler_func(rep_index))
            else {
                warn!(
                    "no handler for RPC with index {rep_index}, channel: {ch_index}, object path: {path}, payload: {hex}",
                    path = self.net_guid_cache.get_path_name_by_guid(obj_guid),
                    hex = hex::encode(&rpc.data),
                );
                continue;
            };

            let actor_guid = connection.get_channel_actor(ch_index).unwrap();

            if let Err(err) = rpc_handler(
                RpcContext {
                    world: self,
                    connection,
                    actor_guid,
                    self_guid: obj_guid,
                },
                rpc,
            ) {
                error!(
                    "failed to handle RPC with index {rep_index}, channel: {ch_index}, object path: {path}, error: {err}",
                    path = self.net_guid_cache.get_path_name_by_guid(obj_guid)
                );
            }
        }
    }

    fn tick_network(&mut self, connection: &mut NetConnection) {
        let mut bunches = Vec::new();
        connection
            .actor_channels
            .iter_mut()
            .for_each(|(_, channel)| {
                channel.channel_impl.tick(self);
                bunches.extend(channel.remove_queued_bunches());
            });

        for (bunch, data) in bunches {
            connection.send_raw_bunch(bunch, &data).unwrap();
        }
    }

    pub fn any_sub_object_has_changes(&self, guid: FNetworkGUID) -> bool {
        let object = self.objects.get(&guid).unwrap();
        if object.rep_layout.rep_layout_changed() || object.rep_layout.custom_properties_changed() {
            return true;
        }

        object
            .sub_objects
            .iter()
            .any(|&guid| self.any_sub_object_has_changes(guid))
    }

    pub fn any_sub_object_has_queued_rpc(&self, guid: FNetworkGUID) -> bool {
        let object = self.objects.get(&guid).unwrap();
        if !object.queued_rpcs.is_empty() {
            return true;
        }

        object
            .sub_objects
            .iter()
            .any(|&guid| self.any_sub_object_has_queued_rpc(guid))
    }

    // Getters

    pub fn get_actor_archetype_new<T: ObjectLayout>(
        &self,
        actor_guid: FNetworkGUID,
    ) -> Option<RefObjectWrap<T>> {
        self.actors
            .get(&actor_guid)
            .map(|a| a.archetype_guid)
            .and_then(|guid| self.get_object(guid))
    }

    pub fn get_actor_archetype_mut_new<T: ObjectLayout>(
        &mut self,
        actor_guid: FNetworkGUID,
    ) -> Option<MutObjectWrap<T>> {
        self.actors
            .get(&actor_guid)
            .map(|a| a.archetype_guid)
            .and_then(|guid| self.get_object_mut(guid))
    }

    pub fn get_object<T: ObjectLayout>(&self, guid: FNetworkGUID) -> Option<RefObjectWrap<T>> {
        self.objects.get(&guid).map(RefObjectWrap::new)
    }

    pub fn get_object_mut<T: ObjectLayout>(
        &mut self,
        guid: FNetworkGUID,
    ) -> Option<MutObjectWrap<T>> {
        self.objects.get_mut(&guid).map(MutObjectWrap::new)
    }
}
