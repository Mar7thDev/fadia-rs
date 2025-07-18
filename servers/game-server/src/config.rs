use std::net::SocketAddr;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct ServerConfig {
    pub udp_addr: SocketAddr,
    pub gameplay: GameplayGlobals,
}

#[derive(Deserialize)]
pub struct GameplayGlobals {
    pub map: String,
    pub game_name: String,
    pub redirect_url: String,
    pub player_character: String,
}
