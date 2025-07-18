use std::net::SocketAddr;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct ServerConfig {
    pub tcp_addr: SocketAddr,
}
