use std::{
    io::{self, Cursor},
    net::SocketAddr,
    sync::Arc,
};

use bitstream_io::{BitRead, BitReader, LittleEndian};
use fadia_engine::util::{self, UnterminatedBitsError};
use tokio::{net::UdpSocket, sync::mpsc};
use tracing::{debug, error, info};

use super::{
    SessionID,
    stateless_connect_handler::{self, HandshakeResult},
};

struct UdpServer<'listener> {
    socket: Arc<UdpSocket>,
    output_tx: mpsc::Sender<(SessionID, Box<[u8]>)>,
    listener: &'listener dyn NetworkEventListener,
}

pub async fn send_task(socket: Arc<UdpSocket>, mut rx: mpsc::Receiver<(SessionID, Box<[u8]>)>) {
    while let Some((id, data)) = rx.recv().await {
        socket.send_to(&data, id.remote_addr).await.unwrap();
    }
}

pub async fn serve(addr: SocketAddr, listener: &dyn NetworkEventListener) -> io::Result<()> {
    let socket = UdpSocket::bind(addr).await?;
    let mut buf = [0u8; 1200];

    info!("listening at udp://{addr}");

    let (tx, rx) = mpsc::channel(256);

    let mut server = UdpServer {
        socket: Arc::new(socket),
        output_tx: tx,
        listener,
    };

    let socket = server.socket.clone();
    tokio::spawn(async move { send_task(socket, rx).await });

    loop {
        if let Ok((len, client_addr)) = server
            .socket
            .recv_from(&mut buf)
            .await
            .inspect_err(|err| debug!("recv_from failed: {err}"))
        {
            if let Err(err) = server.on_receive(&buf[..len], client_addr).await {
                error!("on_receive failed: {err}");
            }
        }
    }
}

pub trait NetworkEventListener {
    fn on_connect(&self, params: ConnectParams);
    fn on_receive(&self, params: ReceiveParams);
}

pub struct ConnectParams {
    pub session_id: SessionID,
    pub output: PacketSender,
    pub server_seq: u16,
    pub client_seq: u16,
}

pub struct ReceiveParams {
    pub session_id: SessionID,
    pub data: Box<[u8]>,
    pub data_offset_in_bits: usize,
    pub data_size_in_bits: usize,
}

pub struct PacketSender(SessionID, mpsc::Sender<(SessionID, Box<[u8]>)>);

impl PacketSender {
    pub fn send(&self, data: Box<[u8]>) {
        let _ = self.1.blocking_send((self.0, data));
    }
}

#[derive(thiserror::Error, Debug)]
enum ReceiveError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    UnterminatedPayload(#[from] UnterminatedBitsError),
}

impl UdpServer<'_> {
    pub async fn on_receive(&mut self, data: &[u8], addr: SocketAddr) -> Result<(), ReceiveError> {
        let mut r = BitReader::endian(Cursor::new(&data), LittleEndian);

        let session_id = SessionID {
            session_id: r.read(2)?,
            client_id: r.read(3)?,
            remote_addr: addr,
        };

        let is_handshake = r.read_bit()?;

        if is_handshake {
            match stateless_connect_handler::on_receive(&session_id, r, addr)? {
                HandshakeResult::None => (),
                HandshakeResult::Send(buf) => {
                    self.socket.send_to(&buf, addr).await?;
                }
                HandshakeResult::SendAndCreateSession(buf, server_seq, client_seq) => {
                    self.socket.send_to(&buf, addr).await?;

                    self.listener.on_connect(ConnectParams {
                        session_id,
                        output: PacketSender(session_id, self.output_tx.clone()),
                        server_seq,
                        client_seq,
                    });
                }
            }
        } else if data.len() > 12 {
            let data_size_in_bits = util::get_bits_from_terminated_stream(data)? - 1;

            self.listener.on_receive(ReceiveParams {
                session_id,
                data: data.into(),
                data_offset_in_bits: r.position_in_bits().unwrap() as usize,
                data_size_in_bits,
            });
        }

        Ok(())
    }
}
