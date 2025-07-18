use std::{net::SocketAddr, path::PathBuf};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct ServerConfig {
    pub tcp_addr: SocketAddr,
    pub serve_dir: PathBuf,
}
