use std::{io, net::SocketAddr};

use tokio::{
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::mpsc,
};
use tracing::{debug, error, info};

use crate::{
    packet::{MessageID, Packet},
    packet_processing,
};

pub struct ClientConnection {
    pub addr: SocketAddr,
    output: mpsc::Sender<Packet>,
}

impl ClientConnection {
    pub async fn send_packet(&self, message_id: MessageID, header: &[u8], payload: &[u8]) {
        let _ = self
            .output
            .send(Packet::build(message_id, header, payload))
            .await;
    }
}

pub(crate) async fn process_connection(stream: TcpStream, addr: SocketAddr) {
    let (read, write) = stream.into_split();
    let (tx, rx) = mpsc::channel(8);
    tokio::spawn(send_loop(write, rx));

    let client_connection = ClientConnection { addr, output: tx };

    let (tx, mut rx) = mpsc::channel(8);
    tokio::spawn(recv_loop(read, tx));

    packet_processing::send_server_version_cmd(&client_connection).await;

    while let Some(packet) = rx.recv().await {
        debug!(
            "received packet: message_id: {:?}, header: {}, payload: {}",
            packet
                .message_id()
                .map(|id| id as u32)
                .unwrap_or_else(|unk| unk),
            hex::encode(packet.header()),
            hex::encode(packet.payload())
        );

        if let Err(err) = packet_processing::on_packet(&client_connection, packet).await {
            error!("failed to process packet from {addr}, error: {err}");
        }
    }

    info!("client from {addr} disconnected");
}

async fn recv_loop(mut r: OwnedReadHalf, recv_packets: mpsc::Sender<Packet>) {
    while let Ok(packet) = recv_packet(&mut r).await {
        if recv_packets.send(packet).await.is_err() {
            break;
        }
    }
}

async fn recv_packet(stream: &mut OwnedReadHalf) -> io::Result<Packet> {
    use tokio::io::AsyncReadExt;

    let size = stream.read_u32_le().await? as usize;
    let mut payload = vec![0u8; size];

    stream.read_exact(&mut payload).await?;
    Ok(Packet::new(payload))
}

async fn send_loop(mut w: OwnedWriteHalf, mut send_packets: mpsc::Receiver<Packet>) {
    while let Some(packet) = send_packets.recv().await {
        if send_packet(&mut w, packet).await.is_err() {
            break;
        }
    }
}

async fn send_packet(w: &mut OwnedWriteHalf, packet: Packet) -> io::Result<()> {
    use tokio::io::AsyncWriteExt;

    let packet = Box::<[u8]>::from(packet);
    w.write_u32_le(packet.len() as u32).await?;
    w.write_all(packet.as_ref()).await
}
