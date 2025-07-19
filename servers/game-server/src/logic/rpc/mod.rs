use std::io;

use fadia_engine::{
    FNetworkGUID,
    replication::NullLayout,
    util::{InBitReader, OutBitWriter},
};

use crate::net::{NetConnection, World};

use super::replication::InRPC;

mod primitives;

#[expect(dead_code)]
pub struct RpcContext<'world, 'connection> {
    pub actor_guid: FNetworkGUID,
    pub self_guid: FNetworkGUID,
    pub world: &'world mut World,
    pub connection: &'connection mut NetConnection,
}

pub type RpcHandlerFunc = fn(context: RpcContext, rpc: InRPC) -> io::Result<()>;

pub trait RpcArgument: Sized {
    fn serialize(&self, w: &mut OutBitWriter) -> io::Result<()>;
    fn deserialize(r: &mut InBitReader) -> io::Result<Self>;
}

pub trait RpcHandler {
    fn get_handler_func(&self, rep_index: u32) -> Option<RpcHandlerFunc>;
}

impl RpcHandler for NullLayout {
    fn get_handler_func(&self, _rep_index: u32) -> Option<RpcHandlerFunc> {
        None
    }
}

macro_rules! call_rpcs {
    // Fake dot-notation :D
    ($($obj_wrap_var:ident.$fn_name:ident($($arg:expr),*));+) => {
        $(let (rep_index, data) = $obj_wrap_var.data().$fn_name($($arg),*);
        $obj_wrap_var.object.add_rpc(rep_index, data);)*
    };
}

pub(crate) use call_rpcs;
