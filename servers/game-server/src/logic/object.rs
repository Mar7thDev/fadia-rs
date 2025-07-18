use std::marker::PhantomData;
use std::{any::Any, collections::HashSet};

use fadia_engine::FNetworkGUID;

use fadia_engine::replication::{NullLayout, RepLayout};

use crate::net::{ActorChannel, World};

use super::rpc::RpcHandler;

pub trait ObjectLayout: RepLayout + RpcHandler {
    fn on_channel_open(&self, _channel: &mut ActorChannel, _world: &World) {
        // on_channel_open.
    }
}

pub struct Object {
    pub rep_layout: Box<dyn ObjectLayout>,
    pub sub_objects: HashSet<FNetworkGUID>,
    pub queued_rpcs: Vec<(u32, Box<[u8]>)>,
}

impl Object {
    pub fn add_rpc(&mut self, index: u32, data: Box<[u8]>) {
        self.queued_rpcs.push((index, data));
    }

    pub fn layout<T: RepLayout>(&self) -> &T {
        let rep_layout = self.rep_layout.as_ref();
        (rep_layout as &dyn Any)
            .downcast_ref()
            .expect("downcast failed")
    }

    pub fn layout_mut<T: RepLayout>(&mut self) -> &mut T {
        let rep_layout = self.rep_layout.as_mut();
        (rep_layout as &mut dyn Any)
            .downcast_mut()
            .expect("downcast failed")
    }

    pub fn on_channel_open(
        &self,
        own_guid: FNetworkGUID,
        channel: &mut ActorChannel,
        world: &World,
    ) {
        if own_guid.is_dynamic() {
            let persistent_level_guid = world
                .net_guid_cache
                .find_static_guid_by_path_name("PersistentLevel")
                .unwrap();

            channel.export_net_guid(world.export_guid(persistent_level_guid));
        } else if own_guid.is_valid() {
            channel.export_net_guid(world.export_guid(own_guid));
        }

        self.sub_objects.iter().for_each(|&guid| {
            channel.export_net_guid(world.export_guid(guid));

            world.objects.get(&guid).unwrap().on_channel_open(
                FNetworkGUID::INVALID,
                channel,
                world,
            );
        });

        self.rep_layout.on_channel_open(channel, world);
    }
}

pub struct RefObjectWrap<'obj, T> {
    pub object: &'obj Object,
    _ty: PhantomData<T>,
}

impl<'obj, T> RefObjectWrap<'obj, T>
where
    T: ObjectLayout,
{
    pub fn new(obj_ref: &'obj Object) -> Self {
        Self {
            object: obj_ref,
            _ty: PhantomData,
        }
    }

    pub fn data(&self) -> &'obj T {
        self.object.layout()
    }
}

pub struct MutObjectWrap<'obj, T> {
    pub object: &'obj mut Object,
    _ty: PhantomData<T>,
}

impl<'obj, T> MutObjectWrap<'obj, T>
where
    T: ObjectLayout,
{
    pub fn new(obj_mut: &'obj mut Object) -> Self {
        Self {
            object: obj_mut,
            _ty: PhantomData,
        }
    }

    pub fn data(&'obj self) -> &'obj T {
        self.object.layout()
    }

    pub fn data_mut(&'obj mut self) -> &'obj mut T {
        self.object.layout_mut()
    }
}

impl ObjectLayout for NullLayout {}
