use std::{
    io::{self, Cursor},
    sync::mpsc,
    thread,
};

use bitstream_io::{BitReader, LittleEndian};
use fadia_engine::util::UnterminatedBitsError;
use tracing::{error, warn};

use crate::{
    assets::GameAssets,
    config::GameplayGlobals,
    net::{ConnectParams, NetworkEventListener, ReceiveParams},
};

use super::scope::{LogicScope, LogicScopeManager};

enum ClusterInput {
    NewConnection(ConnectParams),
    ReceivePacket(ReceiveParams),
}

pub struct ClusterHandle(mpsc::Sender<ClusterInput>);

impl NetworkEventListener for ClusterHandle {
    fn on_connect(&self, params: ConnectParams) {
        self.0.send(ClusterInput::NewConnection(params)).unwrap();
    }

    fn on_receive(&self, params: ReceiveParams) {
        self.0.send(ClusterInput::ReceivePacket(params)).unwrap();
    }
}

fn cluster_logic_loop(
    rx: mpsc::Receiver<ClusterInput>,
    globals: &'static GameplayGlobals,
    assets: &'static GameAssets,
) {
    let mut scope_manager = LogicScopeManager::default();

    while let Ok(input) = rx.recv() {
        match input {
            ClusterInput::NewConnection(params) => {
                scope_manager.create_scope(params, globals, assets);
            }
            ClusterInput::ReceivePacket(params) => {
                if let Some(scope) = scope_manager.get_scope_for_session(params.session_id) {
                    if let Err(err) = receive_packet(scope, params) {
                        error!("receive_packet failed: {err}");
                    }
                } else {
                    warn!("no scope for session_id: {}", params.session_id);
                }
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
enum BitReadError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    UnterminatedBits(#[from] UnterminatedBitsError),
}

fn receive_packet(scope: &mut LogicScope, params: ReceiveParams) -> Result<(), BitReadError> {
    let world = &mut scope.world;
    let connection = scope.connections.get_mut(&params.session_id).unwrap();

    let mut r = BitReader::endian(Cursor::new(params.data.as_ref()), LittleEndian);
    r.seek_bits(io::SeekFrom::Start(params.data_offset_in_bits as u64))?;

    if let Err(err) = connection.received_packet(world, &mut r, params.data_size_in_bits, true) {
        error!("failed to receive packet: {err}");
    }

    world.tick(connection);
    connection.flush_net()?;

    Ok(())
}

pub fn allocate_cluster(
    globals: &'static GameplayGlobals,
    assets: &'static GameAssets,
) -> ClusterHandle {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || cluster_logic_loop(rx, globals, assets));

    ClusterHandle(tx)
}
