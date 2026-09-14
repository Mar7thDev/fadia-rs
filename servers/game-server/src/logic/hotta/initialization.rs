//! Initial state required by the 1.3.10 world loading task.
use bitstream_io::BitWrite;
use fadia_engine::util::{OutBitWriter, WritePrimitivesExt};
use tracing::info;

use super::generated_rpc_indices;
use crate::logic::{ObjectLayout, layout::PlayerState};

const ACTIVITY_DATA_LAYERS_INITIALIZED: u32 = 0x2B;

fn activity_data_layers_initialized_rpc() -> Box<[u8]> {
    let mut bytes = Vec::new();
    let mut out = OutBitWriter::new(&mut bytes);
    out.write_bit(true).unwrap();
    // FHottaRpcMessage derives from FParameterWrapperArray. Even though message
    // 0x2B does not require any wrapped parameters to set the readiness flag,
    // its inherited fields precede Type in the RPC's FRepLayout.
    out.write_u16(0).unwrap(); // ParameterWrappers
    out.write_u32(0).unwrap(); // ReadParameterIndex
    out.write_u32(ACTIVITY_DATA_LAYERS_INITIALIZED).unwrap();
    out.write_bit(true).unwrap();
    out.byte_align().unwrap();
    bytes.into_boxed_slice()
}

pub fn before_rpc(layout: &dyn ObjectLayout, index: u32) -> Vec<(u32, Box<[u8]>)> {
    if index != generated_rpc_indices::INITIAL_RPCS_FINISHED
        || layout.type_id() != std::any::TypeId::of::<PlayerState>()
    {
        return Vec::new();
    }
    // FRepLayout RPC parameters each have a presence bit. Send empty arrays and
    // leave the four optional FNames at None. This initializes the local account's
    // quest component through the native client RPC, including its ready delegate.
    let empty_array_rpc = |name_count: usize| {
        let mut bytes = Vec::new();
        let mut out = OutBitWriter::new(&mut bytes);
        out.write_bit(true).unwrap();
        out.write_u16(0).unwrap();
        for _ in 0..name_count {
            out.write_bit(false).unwrap();
        }
        out.write_bit(true).unwrap();
        out.byte_align().unwrap();
        bytes.into_boxed_slice()
    };

    // AHTPlayerState::ClientDataLayerRPC receives FHottaRpcMessage. After its
    // inherited parameter fields, the 32-bit Type selects the action. Message 0x2B removes
    // the supplied (empty here) activity-layer list and then sets
    // bHasInitActivityDataLayer, the final streaming readiness gate.
    info!("sending player initialization RPCs, including activity data-layer readiness");
    vec![
        (
            generated_rpc_indices::CLIENT_DATA_LAYER_RPC,
            activity_data_layers_initialized_rpc(),
        ),
        (
            generated_rpc_indices::RELIABLE_SUBMITTED_QUEST,
            empty_array_rpc(0),
        ),
        (
            generated_rpc_indices::RELIABLE_QUEST_INFO,
            empty_array_rpc(4),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::activity_data_layers_initialized_rpc;

    #[test]
    fn serializes_activity_data_layer_ready_message() {
        assert_eq!(
            activity_data_layers_initialized_rpc().as_ref(),
            &[1, 0, 0, 0, 0, 0, 0x56, 0, 0, 0, 2]
        );
    }
}
