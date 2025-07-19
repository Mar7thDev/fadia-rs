use std::io;

use common::time_util;
use flatbuffers::FlatBufferBuilder;
use tracing::{debug, info, warn};

use crate::{
    data::{
        ClientKeepAliveCmd, ClientLoginReq, ClientTravelCmd, ClientTravelCmdArgs, ServerVersionCmd,
        ServerVersionCmdArgs,
    },
    net::ClientConnection,
    packet::{MessageID, Packet},
};

#[derive(thiserror::Error, Debug)]
pub enum ProcessPacketError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid flatbuffer received: {0}")]
    InvalidFlatbuffer(#[from] flatbuffers::InvalidFlatbuffer),
}

pub async fn on_packet(
    client_connection: &ClientConnection,
    packet: Packet,
) -> Result<(), ProcessPacketError> {
    if let Ok(message_id) = packet
        .message_id()
        .inspect_err(|unknown| warn!("unknown message received, id: {unknown}"))
    {
        match message_id {
            MessageID::GatewayClientLoginReq => {
                on_client_login_req(client_connection, packet).await?
            }
            MessageID::GatewayClientKeepAliveCmd => {
                on_client_keep_alive_cmd(client_connection, packet).await?
            }
            unhandled => warn!("ignoring message with id: {unhandled:?}"),
        }
    }

    Ok(())
}

async fn on_client_login_req(
    client_connection: &ClientConnection,
    packet: Packet,
) -> Result<(), ProcessPacketError> {
    let client_login_req = flatbuffers::root::<ClientLoginReq>(packet.payload())?;
    info!("{client_login_req:?}");

    let mut builder = FlatBufferBuilder::new();
    let player_uid = builder.create_string("9_1337");
    let empty_1 = builder.create_string("");
    let server_addr = builder.create_string("127.0.0.1:30150");
    let empty_2 = builder.create_string("");
    let empty_3 = builder.create_string("");
    let player_character_bp = builder
        .create_string("/Game/Blueprints/Character/Player/Player_039_Fadia.Player_039_Fadia_C");

    let client_travel_cmd = ClientTravelCmd::create(
        &mut builder,
        &ClientTravelCmdArgs {
            unk_131644: 131644,
            player_uid: Some(player_uid),
            unk_empty_string: Some(empty_1),
            server_addr: Some(server_addr),
            unk_empty_string_2: Some(empty_2),
            unk_empty_string_3: Some(empty_3),
            player_role_id: 1337,
            player_character_bp: Some(player_character_bp),
            compressed_data_blob: None,
            unk_322: 322,
            unk_1: 1,
            unk_cdcdcdcd: 0xCDCDCDCD,
        },
    );

    builder.finish(client_travel_cmd, None);

    // TODO: figure out header format
    client_connection
        .send_packet(
            MessageID::GatewayClientTravelCmd,
            &hex::decode("0000000000000a000c000400000008000a000000").unwrap(),
            builder.finished_data(),
        )
        .await;

    Ok(())
}

async fn on_client_keep_alive_cmd(
    _client_connection: &ClientConnection,
    packet: Packet,
) -> Result<(), ProcessPacketError> {
    let client_keep_alive = flatbuffers::root::<ClientKeepAliveCmd>(packet.payload())?;
    let current_time_in_ticks = time_util::current_time_in_ticks();

    debug!(
        "received KeepAlive from client, current_time_in_ticks: {current_time_in_ticks} {client_keep_alive:?}"
    );

    Ok(())
}

pub async fn send_server_version_cmd(client_connection: &ClientConnection) {
    let mut builder = FlatBufferBuilder::new();

    let server_version = builder.create_string("20200704");
    let client_wan_ip = builder.create_string(&client_connection.addr.ip().to_string());

    let server_version_cmd = ServerVersionCmd::create(
        &mut builder,
        &ServerVersionCmdArgs {
            server_version: Some(server_version),
            client_wan_ip: Some(client_wan_ip),
            client_wan_port: client_connection.addr.port() as i32,
            heart_type: 1, // Disable = 0, Check_Alive = 1, Tracert = 2
            heart_client_interval: 30,
            check_server_interval: u32::MAX,
            server_id: 11000,
            server_time_utc: time_util::unix_utc_timestamp_ms(),
            server_time: time_util::unix_timestamp_ms(),
        },
    );

    builder.finish(server_version_cmd, None);

    client_connection
        .send_packet(
            MessageID::GatewayServerVersionCmd,
            &hex::decode("0000000000000a000c000400000008000a000000").unwrap(),
            builder.finished_data(),
        )
        .await;
}
