use std::{
    io::{self, Cursor},
    net::{IpAddr, SocketAddr},
    sync::LazyLock,
};

use bitstream_io::{BitRead, BitWrite, BitWriter, LittleEndian};
use hmac::{Hmac, Mac};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use rand::RngCore;
use sha1::Sha1;
use tracing::{debug, info};

use super::SessionID;

const HANDSHAKE_MIN_VERSION: u8 = 3;
const LOCAL_NETWORK_VERSION: u32 = 2566650454;
const LOCAL_NETWORK_FEATURES: u16 = 0;

// TODO: regenerate it every 15 seconds
static HANDSHAKE_SECRET: LazyLock<[u8; 64]> = LazyLock::new(|| {
    let mut key = [0u8; 64];
    rand::rng().fill_bytes(&mut key);

    key
});

pub enum HandshakeResult {
    None,
    Send(Vec<u8>),
    SendAndCreateSession(Vec<u8>, u16, u16),
}

pub fn on_receive<R: BitRead>(
    session_id: &SessionID,
    mut r: R,
    addr: SocketAddr,
) -> io::Result<HandshakeResult> {
    use HandshakeResult::*;

    let restart_handshake = r.read_bit()?;
    assert!(
        !restart_handshake,
        "restart handshake is not implemented yet"
    );

    let min_version: u8 = r.read(8)?;
    let cur_version: u8 = r.read(8)?;
    let handshake_packet_type: u8 = r.read(8)?;
    let sent_handshake_packet_count_local_or_remote: u8 = r.read(8)?;

    let local_network_version: u32 = r.read(32)?;
    let local_network_features: u16 = r.read(16)?;

    let Ok(handshake_packet_type): Result<HandshakePacketType, _> =
        handshake_packet_type.try_into()
    else {
        debug!(
            "dropped packet from {session_id}, invalid handshake packet type: {handshake_packet_type}"
        );
        return Ok(None);
    };

    debug!(
        "received handshake packet from {session_id}: min_version: {min_version}, cur_version: {cur_version}, type: {handshake_packet_type:?}, sent_count: {sent_handshake_packet_count_local_or_remote}, net_version: {local_network_version}, net_features: {local_network_features}"
    );

    match handshake_packet_type {
        HandshakePacketType::InitialPacket => {
            let secret_id_bit = false;
            let timestamp = f64::from_bits(rand::rng().next_u64());
            let cookie = generate_cookie(timestamp, addr);

            let mut out_buf = Vec::new();
            let mut w = BitWriter::endian(Cursor::new(&mut out_buf), LittleEndian);

            begin_handshake_packet(
                &mut w,
                session_id,
                HandshakePacketType::Challenge,
                cur_version,
                sent_handshake_packet_count_local_or_remote,
            )?;

            w.write_bit(secret_id_bit)?;
            w.write(64, timestamp.to_bits())?;
            w.write_bytes(&cookie)?;

            finish_handshake_packet(&mut w)?;
            w.byte_align()?;

            Ok(Send(out_buf))
        }
        HandshakePacketType::Response => {
            let _in_secret_id = r.read_bit()?;
            let in_timestamp = f64::from_bits(r.read(64)?);
            let mut in_cookie = [0u8; 20];
            r.read_bytes(&mut in_cookie)?;

            let cookie = generate_cookie(in_timestamp, addr);

            if in_cookie != cookie {
                debug!("cookie mismatch, dropping client {session_id}!");
                return Ok(None);
            }

            let mut out_buf = Vec::new();
            let mut w = BitWriter::endian(Cursor::new(&mut out_buf), LittleEndian);

            begin_handshake_packet(
                &mut w,
                session_id,
                HandshakePacketType::Ack,
                cur_version,
                sent_handshake_packet_count_local_or_remote,
            )?;

            w.write_bit(true)?;
            w.write(64, (-1_f64).to_bits())?;
            w.write_bytes(&cookie)?;

            finish_handshake_packet(&mut w)?;
            w.byte_align()?;

            info!("handshake finished! {session_id}");

            let last_server_sequence =
                u16::from_le_bytes(in_cookie[..2].try_into().unwrap()) & (16384 - 1);
            let last_client_sequence =
                u16::from_le_bytes(in_cookie[2..4].try_into().unwrap()) & (16384 - 1);

            Ok(SendAndCreateSession(
                out_buf,
                last_server_sequence,
                last_client_sequence,
            ))
        }
        unhandled => todo!("handshake packet {unhandled:?}"),
    }
}

#[derive(TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u8)]
pub enum HandshakePacketType {
    InitialPacket = 0,
    Challenge = 1,
    Response = 2,
    Ack = 3,
    RestartHandshake = 4,
    RestartResponse = 5,
    VersionUpgrade = 6,
}

fn generate_cookie(timestamp: f64, addr: SocketAddr) -> [u8; 20] {
    let mut hasher: Hmac<Sha1> = Mac::new_from_slice(HANDSHAKE_SECRET.as_ref()).unwrap();
    let IpAddr::V4(ipv4) = addr.ip() else {
        unreachable!();
    };

    let mut data = [0u8; 14];
    data[..8].copy_from_slice(&timestamp.to_bits().to_le_bytes());
    data[8..12].copy_from_slice(&ipv4.octets());
    data[12..14].copy_from_slice(&addr.port().to_le_bytes());

    hasher.update(&data);
    hasher.finalize().into_bytes().into()
}

fn finish_handshake_packet<W: BitWrite>(w: &mut W) -> io::Result<()> {
    let mut random_data = [0u8; 16];
    rand::rng().fill_bytes(&mut random_data);

    w.write_bytes(&random_data)?;
    w.write_bit(true) // termination bit
}

fn begin_handshake_packet<W: BitWrite>(
    w: &mut W,
    id: &SessionID,
    ty: HandshakePacketType,
    version: u8,
    sent_count: u8,
) -> io::Result<()> {
    w.write(2, id.session_id)?;
    w.write(3, id.client_id)?;
    w.write_bit(true)?; // is_handshake
    w.write_bit(false)?; // restart_handshake
    w.write(8, HANDSHAKE_MIN_VERSION)?; // min_version
    w.write(8, version)?; // cur_version
    w.write::<u8>(8, ty.into())?;
    w.write(8, sent_count)?;
    w.write(32, LOCAL_NETWORK_VERSION)?;
    w.write(16, LOCAL_NETWORK_FEATURES)
}
