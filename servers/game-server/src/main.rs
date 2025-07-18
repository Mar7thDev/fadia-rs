use std::{
    net::SocketAddr,
    sync::{LazyLock, OnceLock},
};

use assets::{AssetsLoadingError, GameAssets};
use common::config_util;
use config::ServerConfig;
use tracing::error;

mod assets;
mod config;
mod logic;
mod net;

#[derive(thiserror::Error, Debug)]
pub enum StartupError {
    #[error("failed to bind at udp://{0} cause: {1}")]
    BindFailed(SocketAddr, std::io::Error),
    #[error("{0}")]
    AssetsLoading(#[from] AssetsLoadingError),
}

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    static ASSETS: OnceLock<GameAssets> = OnceLock::new();
    static CONFIG: LazyLock<ServerConfig> = LazyLock::new(|| {
        config_util::load_or_create::<ServerConfig>(
            "game_server.toml",
            include_str!("../game_server.default.toml"),
        )
    });

    common::log_util::init_tracing();

    let assets = GameAssets::load().inspect_err(|err| error!("{err}"))?;
    let assets = ASSETS.get_or_init(|| assets);

    let cluster = logic::cluster::allocate_cluster(&CONFIG.gameplay, assets);

    net::serve(CONFIG.udp_addr, &cluster)
        .await
        .map_err(|err| StartupError::BindFailed(CONFIG.udp_addr, err))
}
