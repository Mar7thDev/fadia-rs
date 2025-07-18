use std::{io, net::SocketAddr};

use tokio::net::TcpListener;
use tracing::info;

mod client_connection;
pub use client_connection::ClientConnection;

pub async fn serve(host: SocketAddr) -> io::Result<()> {
    let listener = TcpListener::bind(host).await?;
    info!("listening at tcp://{host}");

    loop {
        let Ok((client, addr)) = listener.accept().await else {
            continue;
        };

        info!("new connection from {addr}");
        tokio::spawn(client_connection::process_connection(client, addr));
    }
}
