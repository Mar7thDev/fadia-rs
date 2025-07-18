mod channel;
mod connection;
mod stateless_connect_handler;
mod udp_server;
mod world;

use std::{fmt, net::SocketAddr};

#[allow(unused_imports)]
pub use channel::*;

pub use connection::NetConnection;
pub use world::*;

pub use udp_server::{ConnectParams, NetworkEventListener, ReceiveParams, serve};

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub struct SessionID {
    pub session_id: u8,
    pub client_id: u8,
    pub remote_addr: SocketAddr,
}

impl fmt::Display for SessionID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}-{}]@{}",
            self.session_id, self.client_id, self.remote_addr
        )
    }
}
