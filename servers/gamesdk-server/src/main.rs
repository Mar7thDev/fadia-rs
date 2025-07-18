use std::net::SocketAddr;

use config::ServerConfig;
use tracing::error;

mod config;
mod net;
mod packet;
mod packet_processing;

#[allow(unused, unsafe_op_in_unsafe_fn)]
#[path = "../gen_flatbuffers/data_generated.rs"]
mod data;

#[derive(thiserror::Error, Debug)]
pub enum StartupError {
    #[error("failed to bind at tcp://{0}, cause: {1}")]
    BindFailed(SocketAddr, std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    common::log_util::init_tracing();

    let config = common::config_util::load_or_create::<ServerConfig>(
        "gamesdk_server.toml",
        include_str!("../gamesdk_server.default.toml"),
    );

    net::serve(config.tcp_addr)
        .await
        .map_err(|err| StartupError::BindFailed(config.tcp_addr, err))
}
