use axum::{
    handler::HandlerWithoutStateExt,
    http::{StatusCode, Uri},
    Router,
};
use config::ServerConfig;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tracing::{info, warn};

mod config;

#[derive(thiserror::Error, Debug)]
enum StartupError {
    #[error("failed to bind, is another instance of this server already running? Error: {0}")]
    BindFail(std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    common::log_util::init_tracing();

    let config = common::config_util::load_or_create::<ServerConfig>(
        "patchersdk_server.toml",
        include_str!("../patchersdk_server.default.toml"),
    );

    let app = Router::new().fallback_service(
        ServeDir::new(config.serve_dir).not_found_service(not_found.into_service()),
    );

    let listener = TcpListener::bind(config.tcp_addr)
        .await
        .map_err(StartupError::BindFail)?;

    info!("listening at http://{}", config.tcp_addr);

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn not_found(uri: Uri) -> (StatusCode, &'static str) {
    warn!("requested asset not found: {uri}");
    (StatusCode::NOT_FOUND, "requested asset was not found")
}
